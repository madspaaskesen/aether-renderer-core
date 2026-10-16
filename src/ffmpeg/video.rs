use std::path::PathBuf;

use crate::report::RenderReport;
use crate::utils;

/// Render a video (webm/mp4/mov) using ffmpeg.
///
/// - `webm`: VP9 with alpha (`yuva420p`); constant quality `crf 30` by default
/// - `mp4`: H.264 (`yuv420p`, no alpha), padded to even dimensions, `+faststart`
/// - `mov`: ProRes 4444 with alpha, for video editors (bitrate/crf are ignored)
///
/// `input_pattern` is an ffmpeg image input such as `frames/%06d.png`.
#[allow(clippy::too_many_arguments)]
pub fn render_video(
    input_pattern: &str,
    output: &str,
    fps: u32,
    format: &str,
    bitrate: Option<&str>,
    crf: Option<u32>,
    fade_filter: Option<&str>,
    verbose_ffmpeg: bool,
) -> Result<RenderReport, String> {
    if !matches!(format, "webm" | "mp4" | "mov") {
        return Err(format!(
            "❌ Unsupported format: {}. Use 'webm', 'mp4' or 'mov'.",
            format
        ));
    }

    let mut args = utils::ffmpeg_base_args(verbose_ffmpeg);
    args.extend(["-framerate".into(), fps.to_string()]);
    if input_pattern.contains('*') {
        // Kept for callers passing their own glob; render() always passes a numbered sequence.
        args.extend(["-pattern_type".into(), "glob".into()]);
    }
    args.extend(["-i".into(), input_pattern.to_string()]);

    let mut filters: Vec<&str> = Vec::new();
    if let Some(f) = fade_filter.filter(|f| !f.is_empty()) {
        filters.push(f);
    }

    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
    match format {
        "webm" => {
            args.extend(s(&[
                "-c:v",
                "libvpx-vp9",
                "-pix_fmt",
                "yuva420p",
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
            filters.push("pad=ceil(iw/2)*2:ceil(ih/2)*2");
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
            args.extend(s(&[
                "-c:v",
                "prores_ks",
                "-profile:v",
                "4444",
                "-pix_fmt",
                "yuva444p10le",
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

    let video_stderr = utils::run_ffmpeg_for_output(&args, output)?;
    let warnings = utils::scan_ffmpeg_stderr(&video_stderr);

    Ok(RenderReport {
        output_path: PathBuf::from(output),
        frames_rendered: None,
        ffmpeg_warnings: warnings,
        preview: false,
        notes: Some("Video render complete.".into()),
    })
}
