pub mod config;
pub mod ffmpeg;
pub mod input;
pub mod report;
pub mod utils;

pub use config::RenderConfig;
pub use report::RenderReport;

use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
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

    if !config::FORMATS.contains(&args.format.as_str()) {
        return Err(format!(
            "❌ Unsupported format '{}'. Use one of: {}.",
            args.format,
            config::FORMATS.join(", ")
        ));
    }
    if args.fps == 0 {
        return Err("❌ fps must be greater than 0.".into());
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
            args.preview_frame_limit()
                .filter(|&n| n != config::PREVIEW_MIDDLE),
            &out_path,
            args.verbose,
        )?;
        return Ok(RenderReport {
            output_path: PathBuf::from(out_path.to_string_lossy().into_owned()),
            frames_rendered: Some(1),
            ffmpeg_warnings: Vec::new(),
            preview: true,
            notes: Some("Preview complete.".into()),
        });
    }

    // Check for ffmpeg availability upfront
    if args.verbose_ffmpeg {
        println!("🔍 Checking for ffmpeg...");
    }
    let mut ffmpeg_check = Command::new("ffmpeg");
    ffmpeg_check.arg("-version");
    if !args.verbose_ffmpeg {
        ffmpeg_check.stdout(std::process::Stdio::null());
        ffmpeg_check.stderr(std::process::Stdio::null());
    }
    match ffmpeg_check.status() {
        Ok(s) if s.success() => {}
        Ok(_) => {
            return Err("❌ ffmpeg failed to run correctly.".into());
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(
                    "❌ ffmpeg not found. Install it and make sure it is on your PATH:\n   \
                     macOS: brew install ffmpeg · Ubuntu/Debian: sudo apt install ffmpeg · \
                     Windows: winget install Gyan.FFmpeg"
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

    if frame_count == 0 {
        return Err(format!(
            "❌ No input files found in '{}' matching pattern '{}'.",
            working_input_path.display(),
            pattern
        ));
    }

    // A numbered copy (links) of exactly these frames, readable by ffmpeg on every OS.
    let sequence = input::FrameSequence::new(&frames)
        .map_err(|e| format!("❌ Failed to prepare frames: {}", e))?;
    let input_str = sequence.pattern().to_string_lossy().into_owned();

    let duration = frame_count as f32 / args.fps as f32;
    let fade_filter = fade_filter(&args, duration);

    if args.verbose {
        println!(
            "🌿 Rendering {} frames from {} → {} ({}, {} fps, {:.2} s)...",
            frame_count,
            working_input_path.display(),
            args.output,
            args.format,
            args.fps,
            duration
        );
    }

    let maybe_spinner = if args.verbose {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} 🌿 Rendering with FFmpeg... {elapsed_precise}",
            )
            .unwrap()
            .tick_chars("䷀䷫䷌䷅䷤䷥䷄䷍䷪"),
        );
        pb.enable_steady_tick(Duration::from_millis(120));
        Some(pb)
    } else {
        None
    };

    let result = if args.format == "gif" {
        ffmpeg::gif::render_gif(
            &input_str,
            &args.output,
            args.fps,
            Some(&fade_filter),
            args.verbose_ffmpeg,
        )
    } else {
        ffmpeg::video::render_video(
            &input_str,
            &args.output,
            args.fps,
            &args.format,
            args.bitrate.as_deref(),
            args.crf,
            Some(&fade_filter),
            args.verbose_ffmpeg,
        )
    };
    if let Some(pb) = &maybe_spinner {
        pb.finish_and_clear();
    }
    let mut render_report = result?;

    // Post-inject known input frame count after rendering
    render_report.frames_rendered = Some(frame_count as usize);

    if let Some(ext) = Path::new(&args.output).extension().and_then(|s| s.to_str()) {
        let ext = ext.to_lowercase();
        let expected_ext = match args.format.as_str() {
            "webm" => "webm",
            "mp4" => "mp4",
            "gif" => "gif",
            "mov" => "mov",
            _ => "",
        };

        if ext != expected_ext {
            let warning = format!(
                "⚠️ Warning: Output extension '{}' does not match format '{}'",
                ext, args.format
            );
            render_report.notes =
                Some(render_report.notes.clone().unwrap_or_default() + &format!("\n{}", warning));
        }
    }

    if args.open {
        if let Err(e) = utils::open_output(&args.output) {
            eprintln!("⚠️ Failed to open video preview: {}", e);
        }
    }
    Ok(render_report)
}

/// The `fade` filters for the configured fade in/out. Formats with transparency
/// fade the alpha channel, so a transparent animation fades to transparent
/// instead of to black.
fn fade_filter(args: &RenderConfig, duration: f32) -> String {
    let alpha = if args.format == "mp4" { "" } else { ":alpha=1" };
    let mut fades = Vec::new();
    if args.fade_in > 0.0 {
        fades.push(format!("fade=t=in:st=0:d={}{}", args.fade_in, alpha));
    }
    if args.fade_out > 0.0 {
        let start = (duration - args.fade_out).max(0.0);
        fades.push(format!(
            "fade=t=out:st={}:d={}{}",
            start, args.fade_out, alpha
        ));
    }
    fades.join(",")
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

    // Zips are unpacked so frames are picked in the same (natural) order as a render.
    let (folder, _temp_guard) = if input.extension().is_some_and(|ext| ext == "zip") {
        let (path, guard) = utils::unzip_frames(input, verbose).map_err(|e| e.to_string())?;
        (path, Some(guard))
    } else {
        (input.to_path_buf(), None)
    };
    let pattern = file_pattern.unwrap_or_else(|| "*.png".to_string());
    let frames = input::collect_input_frames(&folder, Some(pattern.clone()))
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
    std::fs::copy(&frames[idx], output).map_err(|e| format!("❌ Failed to copy frame: {}", e))?;

    if verbose {
        println!("🖼️ Preview saved to: {}", output.display());
    }
    Ok(output.to_string_lossy().into_owned())
}
