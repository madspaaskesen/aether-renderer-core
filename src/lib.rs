pub mod config;
pub mod ffmpeg;
pub mod input;
pub mod report;
pub mod utils;

pub use config::RenderConfig;
pub use report::{RenderProgress, RenderReport};

use ffmpeg::FfmpegInput;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// The `input` that means "read a PNG stream from stdin" (`--input -`).
pub const STDIN_INPUT: &str = "-";

/// Is `input` the stdin marker `-`?
pub fn is_stdin_input(input: &Path) -> bool {
    input.as_os_str() == STDIN_INPUT
}

/// Load configuration from file then render
pub fn render_from_config(config_path: &str) -> Result<RenderReport, String> {
    let args = RenderConfig::from_file(config_path)?;
    render(args)
}

/// Orchestrate rendering from a parsed configuration
pub fn render(args: RenderConfig) -> Result<RenderReport, String> {
    render_with_progress(args, |_| {})
}

/// Like [`render`], but calls `on_progress` while ffmpeg works: once with frame 0
/// when encoding starts, then about twice a second, and with 100% at the end.
/// Use it to drive a progress bar in a GUI or another tool.
///
/// ```no_run
/// # use aether_renderer_core::{render_with_progress, RenderConfig};
/// # fn demo(cfg: RenderConfig) -> Result<(), String> {
/// render_with_progress(cfg, |p| {
///     println!("{:.0}% ({}/{} frames)", p.percent, p.frame, p.total_frames);
/// })?;
/// # Ok(()) }
/// ```
pub fn render_with_progress<F>(
    args: RenderConfig,
    mut on_progress: F,
) -> Result<RenderReport, String>
where
    F: FnMut(RenderProgress),
{
    if is_stdin_input(&args.input) {
        return render_stream(args, std::io::stdin(), None, on_progress);
    }
    if args.verbose {
        let version = env!("CARGO_PKG_VERSION");
        eprintln!("🪼 Aether Renderer v{version} starting...");
    }
    let Prepared {
        background,
        mp4_default_background,
    } = prepare(&args)?;

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

    check_ffmpeg(args.verbose_ffmpeg)?;

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
            "🌿 Rendering {} frames from {} → {} ({}, {} fps, {:.2} s{})...",
            frame_count,
            working_input_path.display(),
            args.output,
            args.format,
            args.fps,
            duration,
            match &background {
                Some(c) => format!(", background {c}"),
                None => String::new(),
            }
        );
    }

    let maybe_bar = args.verbose.then(|| progress_bar(Some(frame_count)));

    on_progress(RenderProgress::new(0, frame_count));
    let mut on_frame = |n: u32| {
        let progress = RenderProgress::new(n, frame_count);
        if let Some(pb) = &maybe_bar {
            pb.set_position(progress.frame as u64);
        }
        on_progress(progress);
    };

    let result = encode(
        FfmpegInput::Sequence(&input_str),
        &args,
        &fade_filter,
        background.as_deref(),
        &mut on_frame,
    );
    if let Some(pb) = &maybe_bar {
        pb.finish_and_clear();
    }
    let mut render_report = result?;
    // ffmpeg's last report can come before the final frame is counted.
    on_progress(RenderProgress::new(frame_count, frame_count));

    if mp4_default_background && input::png_has_alpha(&frames[0]) {
        let note = "ℹ️ mp4 cannot be transparent, so transparent areas are black. \
                    Set a background color (--background) to choose another.";
        render_report.notes =
            Some(render_report.notes.clone().unwrap_or_default() + &format!("\n{}", note));
    }

    // Post-inject known input frame count after rendering
    render_report.frames_rendered = Some(frame_count as usize);

    Ok(finish(&args, render_report))
}

/// Render a PNG stream: PNG files back to back, as written by
/// `aether-renderer-capture --stdout` or `ffmpeg -f image2pipe -c:v png -`.
/// No temp files: the bytes go straight to ffmpeg's stdin. `args.input` is ignored.
///
/// `expected_frames` is optional. Without it progress reports `total_frames: 0`
/// (unknown) and `percent: 0` until the end, and `fade_out` can't be used, since
/// it needs to know where the stream ends.
///
/// ```no_run
/// # use aether_renderer_core::{render_stream, RenderConfig};
/// # fn demo(cfg: RenderConfig) -> Result<(), String> {
/// let frames = std::fs::File::open("frames.pngstream").map_err(|e| e.to_string())?;
/// render_stream(cfg, frames, Some(120), |p| println!("{}/{}", p.frame, p.total_frames))?;
/// # Ok(()) }
/// ```
pub fn render_stream<R, F>(
    args: RenderConfig,
    frames: R,
    expected_frames: Option<u32>,
    mut on_progress: F,
) -> Result<RenderReport, String>
where
    R: Read + Send + 'static,
    F: FnMut(RenderProgress),
{
    if args.verbose {
        let version = env!("CARGO_PKG_VERSION");
        eprintln!("🪼 Aether Renderer v{version} starting...");
    }
    let Prepared {
        background,
        mp4_default_background,
    } = prepare(&args)?;
    if args.is_preview() {
        return Err("❌ --preview needs a folder or .zip input, not a stream.".into());
    }
    if expected_frames == Some(0) {
        return Err("❌ --frames must be greater than 0.".into());
    }
    if args.fade_out > 0.0 && expected_frames.is_none() {
        return Err(
            "❌ --fade-out needs to know where the stream ends: pass the frame count with --frames N."
                .into(),
        );
    }
    check_ffmpeg(args.verbose_ffmpeg)?;

    // Peek at the start: an empty stream fails here with a clear message instead
    // of inside ffmpeg, and the first frame tells whether frames are transparent.
    let mut frames = frames;
    let mut head = Vec::new();
    (&mut frames)
        .take(STREAM_PEEK_BYTES)
        .read_to_end(&mut head)
        .map_err(|e| format!("❌ Failed to read the input stream: {e}"))?;
    if head.is_empty() {
        return Err("❌ No frames received: the input stream is empty.".into());
    }
    let has_alpha = input::png_bytes_have_alpha(&head);
    let stream: Box<dyn Read + Send> = Box::new(std::io::Cursor::new(head).chain(frames));

    let duration = expected_frames.map(|n| n as f32 / args.fps as f32);
    let fade_filter = fade_filter(&args, duration.unwrap_or(0.0));
    if args.verbose {
        println!(
            "🌿 Rendering a PNG stream → {} ({}, {} fps{}{})...",
            args.output,
            args.format,
            args.fps,
            match (expected_frames, duration) {
                (Some(n), Some(d)) => format!(", {n} frames, {d:.2} s"),
                _ => String::new(),
            },
            match &background {
                Some(c) => format!(", background {c}"),
                None => String::new(),
            }
        );
    }

    let maybe_bar = args.verbose.then(|| progress_bar(expected_frames));
    let progress = |frame: u32| match expected_frames {
        Some(total) => RenderProgress::new(frame, total),
        None => RenderProgress {
            frame,
            total_frames: 0,
            percent: 0.0,
        },
    };
    on_progress(progress(0));
    let mut frames_seen = 0u32;
    let mut on_frame = |n: u32| {
        frames_seen = frames_seen.max(n);
        if let Some(pb) = &maybe_bar {
            pb.set_position(n as u64);
        }
        on_progress(progress(n));
    };
    let result = encode(
        FfmpegInput::PngStream(stream),
        &args,
        &fade_filter,
        background.as_deref(),
        &mut on_frame,
    );
    if let Some(pb) = &maybe_bar {
        pb.finish_and_clear();
    }
    let mut render_report = result?;
    let total = expected_frames.unwrap_or(frames_seen);
    on_progress(RenderProgress::new(total, total));

    if mp4_default_background && has_alpha {
        let note = "ℹ️ mp4 cannot be transparent, so transparent areas are black. \
                    Set a background color (--background) to choose another.";
        render_report.notes =
            Some(render_report.notes.clone().unwrap_or_default() + &format!("\n{}", note));
    }
    render_report.frames_rendered = Some(frames_seen as usize);
    Ok(finish(&args, render_report))
}

/// How much of a stream is read up front to check it isn't empty and to see
/// whether the first frame is transparent.
const STREAM_PEEK_BYTES: u64 = 64 * 1024;

/// Checked config values every render needs.
struct Prepared {
    /// ffmpeg color to paint behind the frames, if any (always set for mp4).
    background: Option<String>,
    /// mp4 without a chosen background: transparent areas become black.
    mp4_default_background: bool,
}

fn prepare(args: &RenderConfig) -> Result<Prepared, String> {
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
    // Normalized again here for callers that build a RenderConfig themselves.
    let chosen_background = config::parse_background(args.background.as_deref().unwrap_or(""))?;
    let mp4_default_background = chosen_background.is_none() && args.format == "mp4";
    Ok(Prepared {
        background: if mp4_default_background {
            Some("black".to_string())
        } else {
            chosen_background
        },
        mp4_default_background,
    })
}

/// Check for ffmpeg availability upfront, with install hints if it's missing.
fn check_ffmpeg(verbose_ffmpeg: bool) -> Result<(), String> {
    if verbose_ffmpeg {
        println!("🔍 Checking for ffmpeg...");
    }
    let mut ffmpeg_check = Command::new("ffmpeg");
    ffmpeg_check.arg("-version");
    if !verbose_ffmpeg {
        ffmpeg_check.stdout(std::process::Stdio::null());
        ffmpeg_check.stderr(std::process::Stdio::null());
    }
    match ffmpeg_check.status() {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err("❌ ffmpeg failed to run correctly.".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(
            "❌ ffmpeg not found. Install it and make sure it is on your PATH:\n   \
             macOS: brew install ffmpeg · Ubuntu/Debian: sudo apt install ffmpeg · \
             Windows: winget install Gyan.FFmpeg"
                .into(),
        ),
        Err(e) => Err(format!("❌ Failed to execute ffmpeg: {}", e)),
    }
}

/// The `--verbose` progress bar; a spinner when the frame count is unknown.
fn progress_bar(frames: Option<u32>) -> ProgressBar {
    let (pb, template) = match frames {
        Some(n) => (
            ProgressBar::new(n as u64),
            "{spinner:.green} 🌿 Rendering {bar:30.cyan/blue} {pos}/{len} frames {percent:>3}% {elapsed_precise}",
        ),
        None => (
            ProgressBar::new_spinner(),
            "{spinner:.green} 🌿 Rendering {pos} frames {elapsed_precise}",
        ),
    };
    pb.set_style(
        ProgressStyle::with_template(template)
            .unwrap()
            .tick_chars("䷀䷫䷌䷅䷤䷥䷄䷍䷪")
            .progress_chars("█▓░"),
    );
    pb.enable_steady_tick(Duration::from_millis(120));
    pb
}

/// Run the encoder for `args.format`.
fn encode(
    input: FfmpegInput,
    args: &RenderConfig,
    fade_filter: &str,
    background: Option<&str>,
    on_frame: &mut dyn FnMut(u32),
) -> Result<RenderReport, String> {
    if args.format == "gif" {
        ffmpeg::gif::render_gif_from(
            input,
            &args.output,
            args.fps,
            Some(fade_filter),
            background,
            args.verbose_ffmpeg,
            on_frame,
        )
    } else {
        ffmpeg::video::render_video_from(
            input,
            &args.output,
            args.fps,
            &args.format,
            args.bitrate.as_deref(),
            args.crf,
            Some(fade_filter),
            background,
            args.verbose_ffmpeg,
            on_frame,
        )
    }
}

/// Warn about an output extension that doesn't match the format, and `--open`.
fn finish(args: &RenderConfig, mut render_report: RenderReport) -> RenderReport {
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
    render_report
}

/// The `fade` filters for the configured fade in/out. They fade the alpha channel,
/// so a transparent animation fades to transparent, or, with a background color
/// (always for mp4), to that color.
fn fade_filter(args: &RenderConfig, duration: f32) -> String {
    let alpha = ":alpha=1";
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
