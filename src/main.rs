mod utils;
use clap::Parser;
use std::process::Command;
use std::path::PathBuf;
use utils::unzip_frames::unzip_frames;


/// 🌸 Aether Renderer Core
#[derive(Parser, Debug)]
#[command(name = "aether-renderer")]
#[command(about = "Convert PNG frame sequences to WebM/MP4 with alpha", long_about = None)]
struct Args {
    /// Folder containing input PNG files
    #[arg(short, long)]
    input: PathBuf,

    /// Output file name
    #[arg(short, long, default_value = "output.webm")]
    output: String,

    /// Frames per second
    #[arg(short, long, default_value_t = 30)]
    fps: u32,

    /// Output format: webm or mp4
    #[arg(short = 't', long, default_value = "webm")]
    format: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let input_path = &args.input;
    let (working_input_path, _temp_guard) = if input_path.extension().map(|ext| ext == "zip").unwrap_or(false) {
        let (path, guard) = unzip_frames(input_path)?;
        (path, Some(guard))
    } else {
        (input_path.clone(), None)
    };

    // Build input pattern path
    let input_pattern = working_input_path.join("frame_%04d.png");
    let input_str = input_pattern.to_str().unwrap();

    println!("🌿 Rendering {} → {} at {} FPS...", input_str, args.output, args.fps);

    // Build ffmpeg command (For videoes or GIF)
    let output_format = args.format.as_str();

    if output_format == "gif" {
        let palette_path = "palette.png";

        // Step 1: Generate palette
        let palette_status = match Command::new("ffmpeg")
            .args([
                "-i", input_str,
                "-vf", "fps=30,scale=640:-1:flags=lanczos,palettegen",
                "-y", palette_path,
            ])
            .status()
        {
            Ok(s) => s,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    eprintln!("❌ ffmpeg not found. Please install ffmpeg and ensure it is in your PATH.");
                } else {
                    eprintln!("❌ Failed to execute ffmpeg: {}", e);
                }
                return Ok(());
            }
        };

        if !palette_status.success() {
            eprintln!("❌ Failed to generate palette");
            return Ok(());
        }

        // Step 2: Generate GIF using palette
        let gif_status = match Command::new("ffmpeg")
            .args([
                "-framerate", &args.fps.to_string(),
                "-i", input_str,
                "-i", palette_path,
                "-lavfi", "fps=30,scale=640:-1:flags=lanczos [x]; [x][1:v] paletteuse",
                "-y",
                &args.output,
            ])
            .status()
        {
            Ok(s) => s,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    eprintln!("❌ ffmpeg not found. Please install ffmpeg and ensure it is in your PATH.");
                } else {
                    eprintln!("❌ Failed to execute ffmpeg: {}", e);
                }
                return Ok(());
            }
        };

        if gif_status.success() {
            println!("✅ GIF exported: {}", &args.output);
        } else {
            eprintln!("❌ Failed to export GIF");
        }

        // Remove temperary palette.png file
        std::fs::remove_file(palette_path)
            .unwrap_or_else(|e| eprintln!("⚠️ Failed to remove palette file: {}", e));

        return Ok(());
    }

    let codec = match output_format {
        "webm" => "libvpx",
        "mp4" => "libx264",
        _ => {
            eprintln!("❌ Unsupported format: {}", output_format);
            return Ok(());
        }
    };

    let pix_fmt = if output_format == "webm" {
        "yuva420p" // supports alpha
    } else {
        "yuv420p" // no alpha in mp4
    };

    let status = match Command::new("ffmpeg")
        .args([
            "-framerate", &args.fps.to_string(),
            "-i", input_str,
            "-c:v", codec,
            "-pix_fmt", pix_fmt,
            "-auto-alt-ref", "0", // ← required for alpha with libvpx
            "-y",
            &args.output,
        ])

        .status()
    {
        Ok(s) => s,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                eprintln!("❌ ffmpeg not found. Please install ffmpeg and ensure it is in your PATH.");
            } else {
                eprintln!("❌ Failed to execute ffmpeg: {}", e);
            }
            return Ok(());
        }
    };

    if status.success() {
        println!("✅ Video exported: {}", args.output);
    } else {
        eprintln!("❌ ffmpeg failed. Check your frame pattern or input path.");
    }

    Ok(())
}
