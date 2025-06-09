use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;
use zip::ZipArchive;

/// Extracts `frame_*.png` from a ZIP into a temporary folder and returns the
/// folder path along with the temp directory guard.
pub fn unzip_frames(
    zip_path: &Path,
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

        let filename = file.name().rsplit('/').next().unwrap_or("");
        if !filename.ends_with(".png") {
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

        println!("✅ Extracting: {}", full_out_path.display());
        extracted += 1;
    }

    if extracted == 0 {
        return Err("❌ No PNG files found in zip archive".into());
    }

    println!("🗂️  Extracted frames to: {}", temp_path.display());
    Ok((temp_path.clone(), temp_dir))
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

#[cfg(test)]
mod tests {
    use super::unzip_frames;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use tempfile::tempdir;
    use zip::write::{FileOptions, ZipWriter};
    use zip::CompressionMethod;

    // Helper to create a small zip containing two fake PNG files
    fn create_test_zip(path: &Path) -> zip::result::ZipResult<()> {
        let file = File::create(path)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(CompressionMethod::Stored);

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

        let (out_dir, _guard) = unzip_frames(&zip_path)?;

        let count = std::fs::read_dir(&out_dir)?.count();
        assert_eq!(count, 2);
        assert!(out_dir.join("frame_0000.png").exists());
        assert!(out_dir.join("frame_0001.png").exists());

        Ok(())
    }
}
