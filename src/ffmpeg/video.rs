use std::path::PathBuf;

use super::FfmpegInput;
use crate::report::RenderReport;
use crate::utils;

/// Render a video (webm/mp4/mov) using ffmpeg.
///
/// - `webm`: VP9 with alpha (`yuva420p`); constant quality `crf 30` by default
/// - `mp4`: H.264 (`yuv420p`, no alpha), padded to even dimensions, `+faststart`
/// - `mov`: ProRes 4444 with alpha, for video editors (bitrate/crf are ignored)
///
/// `input_pattern` is an ffmpeg image input such as `frames/%06d.png`.
///
/// With a `background` color (an ffmpeg color such as `0x1E1E1E` or `white`) the
/// frames are painted onto it and the output is opaque; `fade_filter` runs first,
/// so fades go to the background color. mp4 has no alpha, so it uses black when
/// no background is given.
///
/// `on_frame` is called with the number of frames written so far.
#[allow(clippy::too_many_arguments)]
pub fn render_video(
    input_pattern: &str,
    output: &str,
    fps: u32,
    format: &str,
    bitrate: Option<&str>,
    crf: Option<u32>,
    fade_filter: Option<&str>,
    background: Option<&str>,
    verbose_ffmpeg: bool,
    on_frame: &mut dyn FnMut(u32),
) -> Result<RenderReport, String> {
    render_video_from(
        FfmpegInput::Sequence(input_pattern),
        output,
        fps,
        format,
        bitrate,
        crf,
        fade_filter,
        background,
        verbose_ffmpeg,
        on_frame,
    )
}

/// [`render_video`] for any [`FfmpegInput`], including a PNG stream on stdin.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_video_from(
    input: FfmpegInput,
    output: &str,
    fps: u32,
    format: &str,
    bitrate: Option<&str>,
    crf: Option<u32>,
    fade_filter: Option<&str>,
    background: Option<&str>,
    verbose_ffmpeg: bool,
    on_frame: &mut dyn FnMut(u32),
) -> Result<RenderReport, String> {
    if !matches!(format, "webm" | "mp4" | "mov") {
        return Err(format!(
            "❌ Unsupported format: {}. Use 'webm', 'mp4' or 'mov'.",
            format
        ));
    }

    let mut args = utils::ffmpeg_base_args(verbose_ffmpeg);
    // A glob is kept for callers passing their own; render() passes a numbered sequence.
    let (input_args, stdin) = input.into_args(fps);
    args.extend(input_args);

    let background = background.or((format == "mp4").then_some("black"));
    let opaque = background.is_some();

    let mut filters: Vec<String> = Vec::new();
    if let Some(f) = fade_filter.filter(|f| !f.is_empty()) {
        filters.push(f.to_string());
    }
    if let Some(color) = background {
        filters.push(utils::background_filter(color));
    }

    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
    match format {
        "webm" => {
            args.extend(s(&[
                "-c:v",
                "libvpx-vp9",
                "-pix_fmt",
                if opaque { "yuv420p" } else { "yuva420p" },
                "-auto-alt-ref",
                "0",
                "-row-mt",
                "1",
                "-deadline",
                "good",
                "-cpu-used",
                "2",
            ]));
            match (crf, bitrate) {
                // crf wins when both are set; "-b:v 0" makes VP9 use pure constant quality
                (Some(c), _) => args.extend(s(&["-crf", &c.to_string(), "-b:v", "0"])),
                (None, Some(b)) => args.extend(s(&["-b:v", b])),
                (None, None) => args.extend(s(&["-crf", "30", "-b:v", "0"])),
            }
        }
        "mp4" => {
            // H.264 needs even width and height; pad by at most one pixel.
            // The extra pixel gets the background color too.
            filters.push(format!(
                "pad=ceil(iw/2)*2:ceil(ih/2)*2:color={}",
                background.unwrap_or("black")
            ));
            args.extend(s(&[
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-preset",
                "medium",
                "-movflags",
                "+faststart",
            ]));
            match (crf, bitrate) {
                (Some(c), _) => args.extend(s(&["-crf", &c.to_string()])),
                (None, Some(b)) => args.extend(s(&["-b:v", b])),
                (None, None) => args.extend(s(&["-crf", "23"])),
            }
        }
        _ => {
            // mov: ProRes 4444 keeps full-quality alpha; the format editors expect.
            // With a background there is no alpha to keep.
            args.extend(s(&[
                "-c:v",
                "prores_ks",
                "-profile:v",
                "4444",
                "-pix_fmt",
                if opaque {
                    "yuv444p10le"
                } else {
                    "yuva444p10le"
                },
                "-vendor",
                "apl0",
            ]));
        }
    }

    let filter_chain = filters.join(",");
    if !filter_chain.is_empty() {
        args.extend(["-vf".into(), filter_chain]);
    }

    args.extend(["-y".into(), output.to_string()]);

    let video_stderr =
        utils::run_ffmpeg_with_input(&args, output, verbose_ffmpeg, on_frame, stdin)?;
    let warnings = utils::scan_ffmpeg_stderr(&video_stderr);

    Ok(RenderReport {
        output_path: PathBuf::from(output),
        frames_rendered: None,
        ffmpeg_warnings: warnings,
        preview: false,
        notes: Some("Video render complete.".into()),
    })
}
