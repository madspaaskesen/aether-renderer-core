use std::fs::File;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use zip::ZipArchive;

/// Extracts frame_*.png from a ZIP into a temp folder, returns the folder path
pub fn unzip_frames(zip_path: &Path) -> Result<(PathBuf, tempfile::TempDir), Box<dyn std::error::Error>> {
    let file = File::open(zip_path)
        .map_err(|e| format!("❌ Failed to open zip file '{}': {}", zip_path.display(), e))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("❌ Failed to read zip archive: {}", e))?;

    let temp_dir = tempdir()
        .map_err(|e| format!("❌ Failed to create temp dir: {}", e))?;

    let temp_path = temp_dir.path().to_path_buf();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("❌ Failed to access file in zip at index {}: {}", i, e))?;

        let filename = file.name().rsplit('/').next().unwrap_or("");
        if !filename.ends_with(".png") {
            continue;
        }

        let full_out_path = temp_path.join(filename);
        let mut out_file = File::create(&full_out_path)
            .map_err(|e| format!("❌ Failed to create output file '{}': {}", full_out_path.display(), e))?;

        std::io::copy(&mut file, &mut out_file)
            .map_err(|e| format!("❌ Failed to copy content to '{}': {}", full_out_path.display(), e))?;

        println!("✅ Extracting: {}", full_out_path.display());
    }

    println!("🗂️  Extracted frames to: {}", temp_path.display());
    Ok((temp_path.clone(), temp_dir))
}
