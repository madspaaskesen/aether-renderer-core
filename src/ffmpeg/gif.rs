use std::fs;
use std::process::Command;

/// Render a GIF using palettegen + paletteuse filters
pub fn render_gif(
    input_pattern: &str,
    output: &str,
    fps: u32,
    fade_filter: Option<&str>,
) -> Result<(), String> {
    let palette_path = "palette.png";

    let palette_status = match Command::new("ffmpeg")
        .args([
            "-i",
            input_pattern,
            "-vf",
            "fps=30,scale=640:-1:flags=lanczos,palettegen",
            "-y",
            palette_path,
        ])
        .status()
    {
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
    let gif_status = match Command::new("ffmpeg")
        .args([
            "-framerate",
            &fps.to_string(),
            "-i",
            input_pattern,
            "-i",
            palette_path,
            "-lavfi",
            &format!("{} [x]; [x][1:v] paletteuse", gif_filter),
            "-y",
            output,
        ])
        .status()
    {
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
        println!("✅ GIF exported: {}", output);
        Ok(())
    } else {
        Err("❌ Failed to export GIF".into())
    }
}
