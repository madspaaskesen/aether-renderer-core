use aether_renderer_core::config::PartialConfig;
use aether_renderer_core::RenderReport;
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
    aether-renderer-core --config render.toml --fps 60 -o draft.webm")]
struct Args {
    /// Config file (.json or .toml); command-line options override its values
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Folder or .zip with the frames
    #[arg(short, long)]
    input: Option<PathBuf>,

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

    /// Constant quality, lower is better (webm: 0-63, default 30; mp4: 0-51, default 23)
    #[arg(long)]
    crf: Option<u32>,

    /// Open the result when done
    #[arg(long)]
    open: bool,

    /// Save a single frame as PNG instead of rendering (default: the middle frame)
    #[arg(long, num_args = 0..=1, value_name = "N")]
    preview: Option<Option<usize>>,

    /// Print details and a progress spinner
    #[arg(short, long)]
    verbose: bool,

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

    if let Err(e) = run(&args) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run(args: &Args) -> Result<(), String> {
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
    let verbose = cfg.verbose;
    let report = aether_renderer_core::render(cfg)?;
    print_report(&report, verbose);
    Ok(())
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
