use crate::utils;

/// Render a video (webm/mp4) using ffmpeg
pub fn render_video(
    input_pattern: &str,
    output: &str,
    fps: u32,
    format: &str,
    bitrate: Option<&str>,
    crf: Option<u32>,
    fade_filter: Option<&str>,
    verbose_ffmpeg: bool,
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

    let mut args: Vec<String> = vec!["-framerate".into(), fps.to_string()];

    if input_pattern.contains('*') {
        args.push("-pattern_type".into());
        args.push("glob".into());
    }

    args.push("-i".into());
    args.push(input_pattern.to_string());

    args.extend_from_slice(&[
        "-c:v".into(),
        codec.to_string(),
        "-pix_fmt".into(),
        pix_fmt.to_string(),
        "-auto-alt-ref".into(),
        "0".into(),
    ]);

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
    if !verbose_ffmpeg {
        args.push("-loglevel".into());
        args.push("warning".into());
    }

    let video_stderr = match utils::run_ffmpeg_with_output(&args) {
        Ok((_, stderr)) => stderr,
        Err(e) => return Err(format!("❌ Failed to execute ffmpeg: {}", e)),
    };
    let _video_warnings = utils::scan_ffmpeg_stderr(&video_stderr);

    println!("✅ Video exported: {}", output);
    Ok(())
}
