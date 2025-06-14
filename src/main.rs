use aether_renderer_core::RenderReport;
use clap::{CommandFactory, Parser};
use std::path::PathBuf;

/// 🌸 Aether Renderer Core
#[derive(Parser, Debug)]
#[command(name = "aether-renderer")]
#[command(about = "Render using configuration file or inline options", long_about = None)]
struct Args {
    /// Path to render configuration JSON
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Input frames folder or ZIP
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output video path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Optional file pattern (e.g. *.png)
    #[arg(long)]
    file_pattern: Option<String>,

    /// Optional fps override
    #[arg(long)]
    fps: Option<u32>,

    /// Optional format (e.g. webm, gif)
    #[arg(long)]
    format: Option<String>,

    /// Output video preview after rendering
    #[arg(long, default_value_t = false)]
    open: bool,

    /// Extract a single frame instead of full render
    #[arg(long, num_args = 0..=1, value_name = "N", value_parser = clap::value_parser!(u32))]
    preview: Option<Option<u32>>,

    /// Enable verbose logging
    #[arg(long)]
    verbose: bool,

    /// Show full ffmpeg logs
    #[arg(long)]
    verbose_ffmpeg: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if let Some(config) = args.config {
        if args.verbose {
            println!("Loading config from {}", config.display());
        }

        return match aether_renderer_core::render_from_config(config.to_str().unwrap()) {
            Ok(report) => {
                get_render_report(report, args.verbose);
                Ok(())
            }
            Err(e) => Err(e.into()),
        };
    }

    if let Some(input) = args.input {
        let output = args.output.unwrap_or_else(|| PathBuf::from("output.webm"));

        if args.verbose {
            println!("Rendering from CLI arguments");
        }

        let cfg = aether_renderer_core::RenderConfig {
            input,
            output: output.to_string_lossy().into_owned(),
            fps: args.fps.unwrap_or(30),
            format: args.format.unwrap_or_else(|| "webm".into()),
            fade_in: 0.0, // or add to Args if needed
            fade_out: 0.0,
            bitrate: None,
            crf: None,
            open: args.open,
            preview: args.preview.flatten().map(|n| n as usize),
            file_pattern: args.file_pattern,
            verbose: args.verbose,
            verbose_ffmpeg: args.verbose_ffmpeg,
        };

        return match aether_renderer_core::render(cfg) {
            Ok(report) => {
                get_render_report(report, args.verbose);
                Ok(())
            }
            Err(e) => Err(e.into()),
        };
    }

    Args::command().print_help()?;
    println!();
    Err("No input provided".into())
}

fn get_render_report(report: RenderReport, verbose: bool) {
    println!(
        "✅ {}",
        report.notes.as_deref().unwrap_or("Render complete.")
    );

    if verbose {
        println!("{}", report.summary());
    }
}
