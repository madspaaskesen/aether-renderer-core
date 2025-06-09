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

    let mut args = vec![
        "-framerate",
        &fps.to_string(),
        "-i",
        input_pattern,
        "-c:v",
        codec,
        "-pix_fmt",
        pix_fmt,
        "-auto-alt-ref",
        "0",
    ];

    if let Some(b) = bitrate {
        args.push("-b:v");
        args.push(b);
    }

    if let Some(c) = crf {
        args.push("-crf");
        args.push(&c.to_string());
    }

    if let Some(filter) = fade_filter {
        if !filter.is_empty() {
            args.push("-vf");
            args.push(filter);
        }
    }

    args.push("-y");
    args.push(output);

    let status = match Command::new("ffmpeg").args(&args).status() {
        Ok(s) => s,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(
                    "❌ ffmpeg not found. Please install ffmpeg and ensure it is in your PATH.".into(),
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
