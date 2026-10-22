use std::path::PathBuf;

use crate::report::RenderReport;
use crate::utils;

/// Render a transparent, looping GIF at the frames' own size and the given fps.
///
/// The palette is generated and applied in a single ffmpeg pass
/// (`split → palettegen → paletteuse`), so no temporary palette file is written.
///
/// With a `background` color the frames are painted onto it and the GIF is opaque.
/// `on_frame` is called with the number of frames written so far; since the
/// palette needs every frame first, it mostly jumps to the end at once.
pub fn render_gif(
    input_pattern: &str,
    output: &str,
    fps: u32,
    fade_filter: Option<&str>,
    background: Option<&str>,
    verbose_ffmpeg: bool,
    on_frame: &mut dyn FnMut(u32),
) -> Result<RenderReport, String> {
    let mut args = utils::ffmpeg_base_args(verbose_ffmpeg);
    args.extend(["-framerate".into(), fps.to_string()]);
    if input_pattern.contains('*') {
        args.extend(["-pattern_type".into(), "glob".into()]);
    }
    args.extend(["-i".into(), input_pattern.to_string()]);

    let mut chain = String::from("[0:v]");
    if let Some(f) = fade_filter.filter(|f| !f.is_empty()) {
        chain.push_str(f);
        chain.push(',');
    }
    match background {
        Some(color) => {
            chain.push_str(&utils::background_filter(color));
            chain.push_str(
                ",split[a][b];[a]palettegen=stats_mode=full[p];\
                 [b][p]paletteuse=dither=sierra2_4a",
            );
        }
        None => chain.push_str(
            "split[a][b];[a]palettegen=reserve_transparent=1:stats_mode=full[p];\
             [b][p]paletteuse=dither=sierra2_4a:alpha_threshold=128",
        ),
    }
    args.extend([
        "-filter_complex".into(),
        chain,
        "-loop".into(),
        "0".into(),
        "-y".into(),
        output.into(),
    ]);

    let stderr = utils::run_ffmpeg_with_progress(&args, output, verbose_ffmpeg, on_frame)?;
    let warnings = utils::scan_ffmpeg_stderr(&stderr);

    Ok(RenderReport {
        output_path: PathBuf::from(output),
        frames_rendered: None,
        ffmpeg_warnings: warnings,
        preview: false,
        notes: Some("GIF export via palettegen".into()),
    })
}
