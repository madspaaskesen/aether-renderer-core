use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use tempfile::tempdir;
use zip::ZipArchive;

fn is_valid_image(file_name: &str) -> bool {
    let name = file_name.to_lowercase();
    name.ends_with(".png") && !name.starts_with("._")
}

/// Extracts `frame_*.png` from a ZIP into a temporary folder and returns the
/// folder path along with the temp directory guard.
pub fn unzip_frames(
    zip_path: &Path,
    verbose: bool,
) -> Result<(PathBuf, tempfile::TempDir), Box<dyn std::error::Error>> {
    let file = File::open(zip_path)
        .map_err(|e| format!("❌ Failed to open zip file '{}': {}", zip_path.display(), e))?;

    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("❌ Failed to read zip archive: {}", e))?;

    let temp_dir = tempdir().map_err(|e| format!("❌ Failed to create temp dir: {}", e))?;
    let temp_path = temp_dir.path().to_path_buf();

    let mut extracted = 0u32;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("❌ Failed to access file in zip at index {}: {}", i, e))?;

        let filename = file.name().rsplit(['/', '\\']).next().unwrap_or("");
        if !is_valid_image(filename) {
            continue;
        }

        let full_out_path = temp_path.join(filename);
        let mut out_file = File::create(&full_out_path).map_err(|e| {
            format!(
                "❌ Failed to create output file '{}': {}",
                full_out_path.display(),
                e
            )
        })?;

        std::io::copy(&mut file, &mut out_file).map_err(|e| {
            format!(
                "❌ Failed to copy content to '{}': {}",
                full_out_path.display(),
                e
            )
        })?;

        if verbose {
            println!("✅ Extracting: {}", full_out_path.display());
        }
        extracted += 1;
    }

    if extracted == 0 {
        return Err("❌ No PNG files found in zip archive".into());
    }

    if verbose {
        if extracted > 1 {
            println!("⚠️  Extracted {} frames from zip", extracted);
        } else {
            println!("✅ Extracted 1 frame from zip");
        }
        println!("🗂️  Extracted frames to: {}", temp_path.display());
    }
    Ok((temp_path.clone(), temp_dir))
}

/// Count PNG files inside a ZIP archive
pub fn count_pngs_in_zip(zip_path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    let file = File::open(zip_path)
        .map_err(|e| format!("❌ Failed to open zip file '{}': {}", zip_path.display(), e))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("❌ Failed to read zip archive: {}", e))?;
    let mut count = 0usize;
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let filename = file.name().rsplit(['/', '\\']).next().unwrap_or("");
        if is_valid_image(filename) {
            count += 1;
        }
    }
    Ok(count)
}

/// Extract a specific PNG frame from a ZIP archive
pub fn extract_frame_from_zip(
    zip_path: &Path,
    frame_index: usize,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(zip_path)
        .map_err(|e| format!("❌ Failed to open zip file '{}': {}", zip_path.display(), e))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("❌ Failed to read zip archive: {}", e))?;
    let mut png_indices = Vec::new();
    for i in 0..archive.len() {
        let f = archive.by_index(i)?;
        let name = f.name().rsplit(['/', '\\']).next().unwrap_or("");
        if is_valid_image(name) {
            png_indices.push(i);
        }
    }
    if png_indices.is_empty() {
        return Err("❌ No PNG files found in zip archive".into());
    }
    if frame_index >= png_indices.len() {
        return Err(format!(
            "❌ Frame index {} out of range (0..{})",
            frame_index,
            png_indices.len() - 1
        )
        .into());
    }
    let mut file = archive.by_index(png_indices[frame_index])?;
    let mut out = File::create(output).map_err(|e| {
        format!(
            "❌ Failed to create output file '{}': {}",
            output.display(),
            e
        )
    })?;
    std::io::copy(&mut file, &mut out)
        .map_err(|e| format!("❌ Failed to copy content to '{}': {}", output.display(), e))?;
    Ok(())
}

/// Open the rendered output in the default system viewer
pub fn open_output(path: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).status().map(|_| ())
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(path).status().map(|_| ())
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", path])
            .status()
            .map(|_| ())
    }
}

pub fn scan_ffmpeg_stderr(stderr: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let stderr_lc = stderr.to_lowercase(); // 👈 normalize

    if stderr_lc.contains("drop") {
        warnings.push("⚠️ FFmpeg reported frame drops.".to_string());
    }

    if stderr_lc.contains("missing") {
        warnings.push("⚠️ Possible missing or unreadable frame(s).".to_string());
    }

    if stderr_lc.contains("buffer") || stderr_lc.contains("underrun") {
        warnings.push("⚠️ Buffer underrun or encoding delay detected.".to_string());
    }

    if stderr_lc.contains("deprecated") {
        warnings.push("⚠️ Deprecated options used in FFmpeg command.".to_string());
    }

    if stderr_lc.contains("high frame rate") {
        warnings.push("⚠️ High frame rate detected, may cause performance issues.".to_string());
    }

    if stderr_lc.contains("invalid frame") {
        warnings.push("⚠️ Invalid frame detected in input.".to_string());
    }

    if stderr_lc.contains("no such file or directory") {
        warnings.push("⚠️ Input file not found or inaccessible.".to_string());
    }

    if stderr_lc.contains("unrecognized option") {
        warnings.push("⚠️ Unrecognized FFmpeg option used.".to_string());
    }

    if stderr_lc.contains("error") {
        warnings.push("❌ FFmpeg encountered an error.".to_string());
    }

    if stderr_lc.contains("warning") {
        warnings.push("⚠️ FFmpeg issued a warning.".to_string());
    }

    if stderr_lc.contains("frame rate very high") {
        warnings
            .push("⚠️ Frame rate very high for a muxer not efficiently supporting it.".to_string());
    }

    if stderr.contains("duration") && stderr.contains("Past") {
        warnings.push("⚠️ Past frame duration too large.".to_string());
    }

    warnings
}

/// Arguments every ffmpeg call starts with: no banner, and only warnings unless
/// `verbose_ffmpeg` is set.
pub fn ffmpeg_base_args(verbose_ffmpeg: bool) -> Vec<String> {
    let level = if verbose_ffmpeg { "info" } else { "warning" };
    vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-loglevel".into(),
        level.into(),
    ]
}

/// Run ffmpeg and return its stderr. On failure the error includes the last lines
/// ffmpeg printed, which usually say what went wrong.
pub fn run_ffmpeg_with_output(args: &[String]) -> Result<(ExitStatus, String), String> {
    let output = Command::new("ffmpeg").args(args).output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "❌ ffmpeg not found in PATH.".to_string()
        } else {
            format!("❌ ffmpeg failed to run: {}", e)
        }
    })?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        let mut msg = format!(
            "❌ ffmpeg exited with code {}",
            output.status.code().unwrap_or(-1)
        );
        let tail: Vec<&str> = stderr
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        if !tail.is_empty() {
            msg.push_str(":\n");
            for line in &tail[tail.len().saturating_sub(8)..] {
                msg.push_str("   ");
                msg.push_str(line);
                msg.push('\n');
            }
            msg.push_str("   (run with --verbose-ffmpeg for the full log)");
        }
        return Err(msg);
    }
    Ok((output.status, stderr))
}

/// Like [`run_ffmpeg_with_output`], but removes a half-written `output` file if ffmpeg fails.
pub fn run_ffmpeg_for_output(args: &[String], output: &str) -> Result<String, String> {
    match run_ffmpeg_with_output(args) {
        Ok((_, stderr)) => Ok(stderr),
        Err(e) => {
            let _ = std::fs::remove_file(output);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::unzip_frames;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use tempfile::tempdir;
    use zip::write::{SimpleFileOptions, ZipWriter};
    use zip::CompressionMethod;

    // Helper to create a small zip containing two fake PNG files
    fn create_test_zip(path: &Path) -> zip::result::ZipResult<()> {
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        zip.start_file("frame_0000.png", options)?;
        zip.write_all(b"png0")?;
        zip.start_file("frame_0001.png", options)?;
        zip.write_all(b"png1")?;
        zip.finish()?;
        Ok(())
    }

    #[test]
    fn unzip_frames_extracts_pngs() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let zip_path = dir.path().join("frames.zip");
        create_test_zip(&zip_path)?;

        let (out_dir, _guard) = unzip_frames(&zip_path, false)?;

        let count = std::fs::read_dir(&out_dir)?.count();
        assert_eq!(count, 2);
        assert!(out_dir.join("frame_0000.png").exists());
        assert!(out_dir.join("frame_0001.png").exists());

        Ok(())
    }
}
