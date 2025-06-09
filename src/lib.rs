pub mod config;
pub mod ffmpeg;
pub mod input;
pub mod utils;

pub use config::RenderConfig;

use indicatif::{ProgressBar, ProgressStyle};
use std::process::Command;
use std::time::Duration;

/// Load configuration from file then render
pub fn render_from_config(config_path: &str) -> Result<(), String> {
    let args = RenderConfig::from_file(config_path)?;
    render(args)
}

/// Orchestrate rendering from a parsed configuration
pub fn render(args: RenderConfig) -> Result<(), String> {
    // Check for ffmpeg availability upfront
    match Command::new("ffmpeg").arg("-version").status() {
        Ok(s) if s.success() => {}
        Ok(_) => {
            return Err("❌ ffmpeg failed to run correctly.".into());
        }
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
    }

    if !args.input.exists() {
        return Err(format!(
            "❌ Input path '{}' does not exist.",
            args.input.display()
        ));
    }

    let input_path = &args.input;
    let (working_input_path, _temp_guard) = if input_path
        .extension()
        .map(|ext| ext == "zip")
        .unwrap_or(false)
    {
        let (path, guard) = utils::unzip_frames(input_path).map_err(|e| e.to_string())?;
        (path, Some(guard))
    } else {
        (input_path.clone(), None)
    };

    let pattern = args
        .file_pattern
        .clone()
        .unwrap_or_else(|| "*.png".to_string());
    let frames = input::collect_input_frames(&working_input_path, Some(pattern.clone()))
        .map_err(|e| format!("❌ Failed to read frames: {}", e))?;
    let frame_count = frames.len() as u32;

    // Use the provided file pattern when building the ffmpeg input string
    let input_pattern = working_input_path.join(&pattern);
    let input_str = input_pattern.to_str().unwrap();

    if frame_count == 0 {
        return Err(format!(
            "❌ No input files found in '{}' matching pattern '{}'.",
            working_input_path.display(),
            pattern
        ));
    }

    let duration = frame_count as f32 / args.fps as f32;

    let mut fade_filter = String::new();
    if args.fade_in > 0.0 {
        fade_filter.push_str(&format!("fade=t=in:st=0:d={}", args.fade_in));
    }
    if args.fade_out > 0.0 {
        if !fade_filter.is_empty() {
            fade_filter.push(',');
        }
        let start = (duration - args.fade_out).max(0.0);
        fade_filter.push_str(&format!("fade=t=out:st={}:d={}", start, args.fade_out));
    }

    println!(
        "🌿 Rendering {} → {} at {} FPS...",
        input_str, args.output, args.fps
    );

    let maybe_spinner = if args.verbose {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} 🌿 Rendering with FFmpeg... {elapsed_precise}",
            )
            .unwrap()
            .tick_chars("⠁⠃⠇⠧⠷⠿⠻⠟⠯⠷⠾⠽⠻⠛⠋"),
        );
        pb.enable_steady_tick(Duration::from_millis(120));
        Some(pb)
    } else {
        None
    };

    if args.format == "gif" {
        ffmpeg::gif::render_gif(input_str, &args.output, args.fps, Some(&fade_filter))?;
    } else {
        ffmpeg::video::render_video(
            input_str,
            &args.output,
            args.fps,
            &args.format,
            args.bitrate.as_deref(),
            args.crf,
            Some(&fade_filter),
        )?;
    }

    if let Some(pb) = &maybe_spinner {
        pb.finish_with_message("✅ FFmpeg rendering complete!");
    }

    if args.preview {
        if let Err(e) = utils::open_output(&args.output) {
            eprintln!("⚠️ Failed to open video preview: {}", e);
        }
    }
    Ok(())
}
