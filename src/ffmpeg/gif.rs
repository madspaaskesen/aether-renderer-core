use std::fs;
use std::path::PathBuf;

use crate::report::RenderReport;
use crate::utils;

/// Render a GIF using palettegen + paletteuse filters
pub fn render_gif(
    input_pattern: &str,
    output: &str,
    fps: u32,
    fade_filter: Option<&str>,
    verbose_ffmpeg: bool,
) -> Result<RenderReport, String> {
    let palette_path = "palette.png";

    // ----- 1. Generate palette -----
    let mut palette_args: Vec<String> = Vec::new();
    if input_pattern.contains('*') {
        palette_args.push("-pattern_type".into());
        palette_args.push("glob".into());
    }
    palette_args.push("-i".into());
    palette_args.push(input_pattern.into());
    palette_args.push("-vf".into());
    palette_args.push("fps=30,scale=640:-1:flags=lanczos,palettegen".into());
    palette_args.push("-y".into());
    palette_args.push(palette_path.into());
    if !verbose_ffmpeg {
        palette_args.push("-loglevel".into());
        palette_args.push("warning".into());
    }

    let palette_stderr = match utils::run_ffmpeg_with_output(&palette_args) {
        Ok((_, stderr)) => stderr,
        Err(e) => return Err(format!("❌ Failed to run ffmpeg for palettegen: {}", e)),
    };
    let _palette_warnings = utils::scan_ffmpeg_stderr(&palette_stderr);

    // ----- 2. Build filter chain -----
    let mut gif_filter = String::from("fps=30,scale=640:-1:flags=lanczos");
    if let Some(filter) = fade_filter {
        if !filter.is_empty() {
            gif_filter.push(',');
            gif_filter.push_str(filter);
        }
    }

    // ----- 3. Render final GIF -----
    let mut gif_args: Vec<String> = vec!["-framerate".into(), fps.to_string()];
    if input_pattern.contains('*') {
        gif_args.push("-pattern_type".into());
        gif_args.push("glob".into());
    }
    gif_args.push("-i".into());
    gif_args.push(input_pattern.into());
    gif_args.push("-i".into());
    gif_args.push(palette_path.into());
    gif_args.push("-lavfi".into());
    gif_args.push(format!("{} [x]; [x][1:v] paletteuse", gif_filter));
    gif_args.push("-y".into());
    gif_args.push(output.into());
    if !verbose_ffmpeg {
        gif_args.push("-loglevel".into());
        gif_args.push("warning".into());
    }

    let gif_stderr = match utils::run_ffmpeg_with_output(&gif_args) {
        Ok((_, stderr)) => stderr,
        Err(e) => return Err(format!("❌ Failed to render final GIF: {}", e)),
    };
    let _gif_warnings = utils::scan_ffmpeg_stderr(&gif_stderr);

    // ----- 4. Clean up -----
    fs::remove_file(palette_path)
        .unwrap_or_else(|e| eprintln!("⚠️ Failed to remove palette file: {}", e));

    Ok(RenderReport {
        output_path: PathBuf::from(output),
        frames_rendered: None,
        ffmpeg_warnings: _gif_warnings,
        preview: false,
        notes: Some("GIF export via palettegen".into()),
    })
}
