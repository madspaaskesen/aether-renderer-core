use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::process::Command;
use std::time::Duration;

/// Render a GIF using palettegen + paletteuse filters
pub fn render_gif(
    input_pattern: &str,
    output: &str,
    fps: u32,
    fade_filter: Option<&str>,
) -> Result<(), String> {
    let palette_path = "palette.png";

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

    let palette_status = match Command::new("ffmpeg").args(&palette_args).status() {
        Ok(s) => s,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(
                    "❌ ffmpeg not found. Please install ffmpeg and ensure it is in your PATH."
                        .into(),
                );
            } else {
                return Err(format!("❌ Failed to execute ffmpeg: {}", e));
            }
        }
    };

    if !palette_status.success() {
        return Err("❌ Failed to generate palette".into());
    }

    let mut gif_filter = String::from("fps=30,scale=640:-1:flags=lanczos");
    if let Some(filter) = fade_filter {
        if !filter.is_empty() {
            gif_filter.push(',');
            gif_filter.push_str(filter);
        }
    }
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

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} 🌿 Rendering with FFmpeg... {elapsed_precise}",
        )
        .unwrap()
        .tick_chars("⠁⠃⠇⠧⠷⠿⠻⠟⠯⠷⠾⠽⠻⠛⠋"),
    );
    pb.enable_steady_tick(Duration::from_millis(120));

    let gif_status = match Command::new("ffmpeg").args(&gif_args).status() {
        Ok(s) => s,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(
                    "❌ ffmpeg not found. Please install ffmpeg and ensure it is in your PATH."
                        .into(),
                );
            } else {
                return Err(format!("❌ Failed to execute ffmpeg: {}", e));
            }
        }
    };

    fs::remove_file(palette_path)
        .unwrap_or_else(|e| eprintln!("⚠️ Failed to remove palette file: {}", e));

    if gif_status.success() {
        pb.finish_with_message("✅ GIF rendering complete!");
        println!("✅ GIF exported: {}", output);
        Ok(())
    } else {
        pb.finish_with_message("❌ Failed to export GIF");
        Err("❌ Failed to export GIF".into())
    }
}
