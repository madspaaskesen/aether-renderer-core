use std::path::PathBuf;

use crate::report::RenderReport;
use crate::utils;

/// Render a transparent, looping GIF at the frames' own size and the given fps.
///
/// The palette is generated and applied in a single ffmpeg pass
/// (`split → palettegen → paletteuse`), so no temporary palette file is written.
pub fn render_gif(
    input_pattern: &str,
    output: &str,
    fps: u32,
    fade_filter: Option<&str>,
    verbose_ffmpeg: bool,
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
    chain.push_str(
        "split[a][b];[a]palettegen=reserve_transparent=1:stats_mode=full[p];\
         [b][p]paletteuse=dither=sierra2_4a:alpha_threshold=128",
    );
    args.extend([
        "-filter_complex".into(),
        chain,
        "-loop".into(),
        "0".into(),
        "-y".into(),
        output.into(),
    ]);

    let stderr = utils::run_ffmpeg_for_output(&args, output)?;
    let warnings = utils::scan_ffmpeg_stderr(&stderr);

    Ok(RenderReport {
        output_path: PathBuf::from(output),
        frames_rendered: None,
        ffmpeg_warnings: warnings,
        preview: false,
        notes: Some("GIF export via palettegen".into()),
    })
}
