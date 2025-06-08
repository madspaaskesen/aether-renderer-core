mod utils;
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use utils::unzip_frames::unzip_frames;


/// 🌸 Aether Renderer Core
#[derive(Parser, Debug)]
#[command(name = "aether-renderer")]
#[command(about = "Convert PNG frame sequences to WebM (alpha) or MP4", long_about = None)]
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

    /// Fade in duration in seconds
    #[arg(long, default_value_t = 0.0)]
    fade_in: f32,

    /// Fade out duration in seconds
    #[arg(long, default_value_t = 0.0)]
    fade_out: f32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Check for ffmpeg availability upfront
    match Command::new("ffmpeg").arg("-version").status() {
        Ok(s) if s.success() => {}
        Ok(_) => {
            return Err("❌ ffmpeg failed to run correctly.".into());
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err("❌ ffmpeg not found. Please install ffmpeg and ensure it is in your PATH.".into());
            } else {
                return Err(format!("❌ Failed to execute ffmpeg: {}", e).into());
            }
        }
    }

    if !args.input.exists() {
        return Err(format!("❌ Input path '{}' does not exist.", args.input.display()).into());
    }

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

    // Count frames to determine duration for fade-out
    let frame_count = fs::read_dir(&working_input_path)?
        .filter_map(|e| e.ok())
        .filter(|e| match e.path().extension().and_then(|s| s.to_str()) {
            Some("png") | Some("webp") => true,
            _ => false,
        })
        .count() as u32;

    if frame_count == 0 {
        return Err(format!("❌ No PNG files found in '{}'.", working_input_path.display()).into());
    }

    let duration = frame_count as f32 / args.fps as f32;

    // Build fade filter if requested
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
        let mut gif_filter = String::from("fps=30,scale=640:-1:flags=lanczos");
        if !fade_filter.is_empty() {
            gif_filter.push(',');
            gif_filter.push_str(&fade_filter);
        }
        let gif_status = match Command::new("ffmpeg")
            .args([
                "-framerate", &args.fps.to_string(),
                "-i", input_str,
                "-i", palette_path,
                "-lavfi",
                &format!("{} [x]; [x][1:v] paletteuse", gif_filter),
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
            eprintln!("❌ Unsupported format: {}. Use 'webm', 'mp4' or 'gif'.", output_format);
            return Ok(());
        }
    };

    let pix_fmt = match output_format {
        "webm" => "yuva420p", // supports alpha
        "mp4" => "yuv420p",   // no alpha
        _ => unreachable!(),
    };

    let mut ffmpeg_args = vec![
        "-framerate".to_string(),
        args.fps.to_string(),
        "-i".to_string(),
        input_str.to_string(),
        "-c:v".to_string(),
        codec.to_string(),
        "-pix_fmt".to_string(),
        pix_fmt.to_string(),
        "-auto-alt-ref".to_string(),
        "0".to_string(),
    ];

    if !fade_filter.is_empty() {
        ffmpeg_args.push("-vf".to_string());
        ffmpeg_args.push(fade_filter.clone());
    }

    ffmpeg_args.push("-y".to_string());
    ffmpeg_args.push(args.output.clone());

    let status = match Command::new("ffmpeg")
        .args(ffmpeg_args)

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
