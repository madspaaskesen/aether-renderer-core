pub mod config;
pub mod ffmpeg;
pub mod input;
pub mod report;
pub mod utils;

pub use config::RenderConfig;
pub use report::RenderReport;

use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// Load configuration from file then render
pub fn render_from_config(config_path: &str) -> Result<RenderReport, String> {
    let args = RenderConfig::from_file(config_path)?;
    render(args)
}

/// Orchestrate rendering from a parsed configuration
pub fn render(args: RenderConfig) -> Result<RenderReport, String> {
    if args.verbose {
        let version = env!("CARGO_PKG_VERSION");
        eprintln!("🪼 Aether Renderer v{version} starting...");
    }
    // Validate output path
    if args.output.is_empty() {
        return Err("❌ Output path cannot be empty.".into());
    }

    // Is this a preview render?
    if args.is_preview() {
        if args.open {
            eprintln!("⚠️ '--open' is only supported for full render. Ignoring for preview.");
        }
        let mut out_path = PathBuf::from(&args.output);
        if out_path.extension().is_some() {
            out_path.set_extension("png");
        } else {
            out_path = out_path.with_extension("png");
        }
        preview_frame(
            &args.input,
            args.file_pattern.clone(),
            args.preview_frame_limit(),
            &out_path,
            args.verbose,
        )?;
        return Ok(RenderReport {
            output_path: PathBuf::from(out_path.to_string_lossy().into_owned()),
            frames_rendered: None,
            ffmpeg_warnings: Vec::new(),
            preview: true,
            notes: Some("Preview complete.".into()),
        });
    }

    // Check for ffmpeg availability upfront
    if args.verbose_ffmpeg {
        println!("🔍 Checking for ffmpeg...");
    }
    match {
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-version");

        if !args.verbose_ffmpeg {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }

        cmd.status()
    } {
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
        let (path, guard) =
            utils::unzip_frames(input_path, args.verbose).map_err(|e| e.to_string())?;
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

    if args.verbose {
        println!(
            "🌿 Rendering {} → {} at {} FPS...",
            input_str, args.output, args.fps
        );
    }

    let maybe_spinner = if args.verbose {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} 🌿 Rendering with FFmpeg... {elapsed_precise}",
            )
            .unwrap()
            //.tick_chars("⠁⠃⠇⠧⠷⠿⠻⠹⠸⠰⠠   ⠟⠏⠛⠋  ⠻⠯⠷⠾⠽"),
            .tick_chars("䷀䷫䷌䷅䷤䷥䷄䷍䷪"),
        );
        pb.enable_steady_tick(Duration::from_millis(120));
        Some(pb)
    } else {
        None
    };

    let render_report = if args.format == "gif" {
        ffmpeg::gif::render_gif(
            input_str,
            &args.output,
            args.fps,
            Some(&fade_filter),
            args.verbose_ffmpeg,
        )
    } else {
        ffmpeg::video::render_video(
            input_str,
            &args.output,
            args.fps,
            &args.format,
            args.bitrate.as_deref(),
            args.crf,
            Some(&fade_filter),
            args.verbose_ffmpeg,
        )
    }?;

    if let Some(pb) = &maybe_spinner {
        pb.finish_with_message("✅ FFmpeg rendering complete!");
    }

    if args.open {
        if let Err(e) = utils::open_output(&args.output) {
            eprintln!("⚠️ Failed to open video preview: {}", e);
        }
    }
    Ok(render_report)
}

/// Extract a single frame from an input folder or ZIP archive
pub fn preview_frame(
    input: &std::path::Path,
    file_pattern: Option<String>,
    frame_index: Option<usize>,
    output: &std::path::Path,
    verbose: bool,
) -> Result<String, String> {
    if !input.exists() {
        return Err(format!(
            "❌ Input path '{}' does not exist.",
            input.display()
        ));
    }

    if input.extension().map(|ext| ext == "zip").unwrap_or(false) {
        let count = utils::count_pngs_in_zip(input).map_err(|e| e.to_string())?;
        if count == 0 {
            return Err("❌ No PNG files found in zip archive".into());
        }
        let idx = frame_index.unwrap_or(count / 2);
        if idx >= count {
            return Err(format!(
                "❌ Frame index {} out of range (0..{})",
                idx,
                count - 1
            ));
        }
        utils::extract_frame_from_zip(input, idx, output).map_err(|e| e.to_string())?;
    } else {
        let pattern = file_pattern.clone().unwrap_or_else(|| "*.png".to_string());
        let frames = input::collect_input_frames(input, Some(pattern.clone()))
            .map_err(|e| format!("❌ Failed to read frames: {}", e))?;
        if frames.is_empty() {
            return Err(format!(
                "❌ No input files found in '{}' matching pattern '{}'",
                input.display(),
                pattern
            ));
        }
        let idx = frame_index.unwrap_or(frames.len() / 2);
        if idx >= frames.len() {
            return Err(format!(
                "❌ Frame index {} out of range (0..{})",
                idx,
                frames.len() - 1
            ));
        }
        std::fs::copy(&frames[idx], output)
            .map_err(|e| format!("❌ Failed to copy frame: {}", e))?;
    }

    if verbose {
        println!("🖼️ Preview saved to: {}", output.display());
    }
    Ok(output.to_string_lossy().into_owned())
}
