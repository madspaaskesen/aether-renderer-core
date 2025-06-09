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
        return aether_renderer_core::render_from_config(config.to_str().unwrap())
            .map_err(|e| e.into());
    }

    if let Some(input) = args.input {
        let output = args.output.unwrap_or_else(|| PathBuf::from("output.webm"));

        if args.verbose {
            println!("Rendering from CLI arguments");
        }

        let cfg = aether_renderer_core::RenderConfig {
            input,
            output: output.to_string_lossy().into_owned(),
            fps: 30,
            format: "webm".into(),
            fade_in: 0.0,
            fade_out: 0.0,
            bitrate: None,
            crf: None,
            preview: false,
            file_pattern: None,
        };

        return aether_renderer_core::render(cfg).map_err(|e| e.into());
    }

    Args::command().print_help()?;
    println!();
    Err("No input provided".into())
}
