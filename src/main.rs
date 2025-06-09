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

    /// Optional preview toggle
    #[arg(long, default_value_t = false)]
    preview: bool,

    /// Enable verbose logging
    #[arg(long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if let Some(config) = args.config {
        if args.verbose {
            println!("Loading config from {}", config.display());
        }
        let out = aether_renderer_core::render_from_config(config.to_str().unwrap())?;
        if args.verbose {
            println!("Rendered to {}", out);
        }
        return Ok(());
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
            preview: args.preview,
            file_pattern: args.file_pattern,
            verbose: args.verbose,
        };

        let out = aether_renderer_core::render(cfg)?;
        if args.verbose {
            println!("Rendered to {}", out);
        }
        return Ok(());
    }

    Args::command().print_help()?;
    println!();
    Err("No input provided".into())
}
