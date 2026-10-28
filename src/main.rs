use aether_renderer_core::config::PartialConfig;
use aether_renderer_core::{is_stdin_input, RenderConfig, RenderProgress, RenderReport};
use clap::{CommandFactory, Parser};
use std::path::PathBuf;

/// 🪼 Aether Renderer Core: render PNG sequences to transparent WebM, MOV, GIF or MP4.
///
/// Every option can also be set in a JSON or TOML file passed with --config;
/// options given on the command line override the file.
#[derive(Parser, Debug)]
#[command(name = "aether-renderer-core", version, about, long_about = None)]
#[command(after_help = "Examples:\n  \
    aether-renderer-core -i frames/ -o out.webm\n  \
    aether-renderer-core -i frames.zip -o out.gif --fps 24\n  \
    aether-renderer-core -i frames/ -o out.mov            (ProRes 4444 with alpha, for video editors)\n  \
    aether-renderer-core -i frames/ -o out.mp4 --background '#1e1e1e'\n  \
    aether-renderer-core -i frames/ -o out.webm --progress json   (one JSON line per update, for other programs)\n  \
    aether-renderer-capture -i anim.html --stdout | aether-renderer-core -i - -o out.webm --frames 60\n  \
    aether-renderer-core --config render.toml --fps 60 -o draft.webm")]
struct Args {
    /// Config file (.json or .toml); command-line options override its values
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Folder or .zip with the frames, or '-' for a PNG stream on stdin
    /// (PNG files back to back, e.g. from aether-renderer-capture --stdout)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// How many frames the stdin stream has (with --input -). Optional; gives a
    /// real progress percentage and is needed for --fade-out
    #[arg(long, value_name = "N")]
    frames: Option<u32>,

    /// Output file [default: output.<format>]
    #[arg(short, long)]
    output: Option<String>,

    /// Which files to use, e.g. '*.png', 'scene1_*.png' or 'frame_%04d.png' [default: *.png]
    #[arg(long)]
    file_pattern: Option<String>,

    /// Frames per second [default: 30]
    #[arg(long)]
    fps: Option<u32>,

    /// Output format [default: from the output extension, else webm]
    #[arg(long, value_parser = ["webm", "mp4", "gif", "mov"])]
    format: Option<String>,

    /// Fade in over the first N seconds (fades transparency for webm/mov/gif)
    #[arg(long, value_name = "SECONDS")]
    fade_in: Option<f32>,

    /// Fade out over the last N seconds
    #[arg(long, value_name = "SECONDS")]
    fade_out: Option<f32>,

    /// Target bitrate, e.g. 2M or 2500k (webm/mp4)
    #[arg(long)]
    bitrate: Option<String>,

    /// Solid background color behind transparent frames, e.g. '#1e1e1e' or 'white'.
    /// Makes every format opaque. mp4 can't be transparent and uses black without it
    #[arg(long, value_name = "COLOR")]
    background: Option<String>,

    /// Constant quality, lower is better (webm: 0-63, default 30; mp4: 0-51, default 23)
    #[arg(long)]
    crf: Option<u32>,

    /// Open the result when done
    #[arg(long)]
    open: bool,

    /// Save a single frame as PNG instead of rendering (default: the middle frame)
    #[arg(long, num_args = 0..=1, value_name = "N")]
    preview: Option<Option<usize>>,

    /// Print details and a progress bar
    #[arg(short, long)]
    verbose: bool,

    /// Report progress for other programs: 'json' prints one JSON object per line
    /// on stdout ({"event":"progress",...}, then "done" or "error")
    #[arg(long, value_name = "FORMAT", value_parser = ["json"])]
    progress: Option<String>,

    /// Show ffmpeg's own output
    #[arg(long)]
    verbose_ffmpeg: bool,
}

impl Args {
    /// What was given on the command line. Flags only count when set, so a config
    /// file's `"verbose": true` is not switched off by leaving out `--verbose`.
    fn overrides(&self) -> PartialConfig {
        PartialConfig {
            input: self.input.clone(),
            output: self.output.clone(),
            fps: self.fps,
            format: self.format.clone(),
            fade_in: self.fade_in,
            fade_out: self.fade_out,
            bitrate: self.bitrate.clone(),
            crf: self.crf,
            open: self.open.then_some(true),
            preview: self.preview,
            file_pattern: self.file_pattern.clone(),
            verbose: self.verbose.then_some(true),
            verbose_ffmpeg: self.verbose_ffmpeg.then_some(true),
            background: self.background.clone(),
        }
    }
}

fn main() {
    let args = Args::parse();

    if args.config.is_none() && args.input.is_none() {
        let _ = Args::command().print_help();
        eprintln!("\n❌ Nothing to render: pass --input <folder or .zip> or --config <file>.");
        std::process::exit(2);
    }

    let json = args.progress.as_deref() == Some("json");
    if let Err(e) = run(&args, json) {
        if json {
            emit(serde_json::json!({ "event": "error", "message": e }));
        }
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run(args: &Args, json: bool) -> Result<(), String> {
    let file = match &args.config {
        Some(path) => {
            if args.verbose {
                println!("Loading config from {}", path.display());
            }
            PartialConfig::from_file(&path.to_string_lossy())?
        }
        None => PartialConfig::default(),
    };
    let cfg = args.overrides().or(file).into_config()?;
    let stdin = is_stdin_input(&cfg.input);
    if args.frames.is_some() && !stdin {
        return Err("❌ --frames only applies to a stream on stdin (--input -).".into());
    }
    let frames = args.frames;
    let verbose = cfg.verbose;
    if !json {
        let report = render(cfg, stdin, frames, |_| {})?;
        print_report(&report, verbose);
        return Ok(());
    }

    let mut last = None;
    let report = render(cfg, stdin, frames, |p| {
        // Skip repeats (ffmpeg reports the same frame while it waits on input).
        if last != Some(p.frame) {
            last = Some(p.frame);
            let mut line = serde_json::to_value(p).unwrap_or_default();
            line["event"] = "progress".into();
            emit(line);
        }
    })?;
    emit(serde_json::json!({
        "event": "done",
        "output": report.output_path,
        "frames": report.frames_rendered,
        "warnings": report.ffmpeg_warnings,
        "notes": report.notes,
    }));
    Ok(())
}

/// Render a folder/ZIP, or the PNG stream on stdin with its optional frame count.
fn render<F: FnMut(RenderProgress)>(
    cfg: RenderConfig,
    stdin: bool,
    frames: Option<u32>,
    on_progress: F,
) -> Result<RenderReport, String> {
    if stdin {
        aether_renderer_core::render_stream(cfg, std::io::stdin(), frames, on_progress)
    } else {
        aether_renderer_core::render_with_progress(cfg, on_progress)
    }
}

/// Print one JSON line and flush it right away, so a program reading the pipe
/// sees it immediately. If the reader has gone away, keep rendering anyway.
fn emit(value: serde_json::Value) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{value}");
    let _ = out.flush();
}

fn print_report(report: &RenderReport, verbose: bool) {
    println!(
        "✅ {} → {}",
        report.notes.as_deref().unwrap_or("Render complete."),
        report.output_path.display()
    );
    if verbose {
        println!("{}", report.summary());
    }
}
