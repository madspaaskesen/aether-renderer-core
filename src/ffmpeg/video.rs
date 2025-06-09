use std::process::Command;

/// Render a video (webm/mp4) using ffmpeg
pub fn render_video(
    input_pattern: &str,
    output: &str,
    fps: u32,
    format: &str,
    bitrate: Option<&str>,
    crf: Option<u32>,
    fade_filter: Option<&str>,
) -> Result<(), String> {
    let codec = match format {
        "webm" => "libvpx",
        "mp4" => "libx264",
        _ => {
            return Err(format!(
                "❌ Unsupported format: {}. Use 'webm' or 'mp4'.",
                format
            ));
        }
    };

    let pix_fmt = match format {
        "webm" => "yuva420p",
        "mp4" => "yuv420p",
        _ => unreachable!(),
    };

    let mut args: Vec<String> = vec![
        "-framerate".into(),
        fps.to_string(),
        "-i".into(),
        input_pattern.to_string(),
        "-c:v".into(),
        codec.to_string(),
        "-pix_fmt".into(),
        pix_fmt.to_string(),
        "-auto-alt-ref".into(),
        "0".into(),
    ];

    if let Some(b) = bitrate {
        args.push("-b:v".into());
        args.push(b.to_string());
    }

    if let Some(c) = crf {
        args.push("-crf".into());
        args.push(c.to_string());
    }

    if let Some(filter) = fade_filter {
        if !filter.is_empty() {
            args.push("-vf".into());
            args.push(filter.to_string());
        }
    }

    args.push("-y".into()); // Overwrite output file if it exists
    args.push(output.to_string());

    let status = match Command::new("ffmpeg").args(&args).status() {
        Ok(s) => s,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(
                    "❌ ffmpeg not found. Please install ffmpeg and ensure it is in your PATH."
                        .into(),
                );
            } else {
                return Err(format!("❌ Failed to execute ffmpeg: {}", e));
            }
        }
    };

    if status.success() {
        println!("✅ Video exported: {}", output);
        Ok(())
    } else {
        Err("❌ ffmpeg failed. Check your frame pattern or input path.".into())
    }
}
