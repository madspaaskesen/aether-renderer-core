use clap::Parser;
use std::path::PathBuf;

/// 🌸 Aether Renderer Core
#[derive(Parser, Debug)]
#[command(name = "aether-renderer")]
#[command(about = "Render using configuration file", long_about = None)]
struct Args {
    /// Path to render configuration JSON
    #[arg(short, long)]
    config: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    aether_renderer_core::render_from_config(args.config.to_str().unwrap())
        .map_err(|e| e.into())
}

