use glob::glob;
use std::path::{Path, PathBuf};

/// Collect files from `input_folder` matching the optional pattern.
/// Defaults to `*.png` when no pattern is provided.
pub fn collect_input_frames(
    input_folder: &Path,
    file_pattern: Option<String>,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let pattern = file_pattern.unwrap_or_else(|| "*.png".to_string());
    let glob_path = format!("{}/{}", input_folder.display(), pattern);
    let mut frames: Vec<PathBuf> = glob(&glob_path)?.filter_map(Result::ok).collect();
    frames.sort();
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::collect_input_frames;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn collects_all_png_by_default() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        File::create(dir.path().join("a.png"))?;
        File::create(dir.path().join("b.png"))?;
        let frames = collect_input_frames(dir.path(), None)?;
        assert_eq!(frames.len(), 2);
        Ok(())
    }

    #[test]
    fn collects_with_custom_pattern() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        File::create(dir.path().join("scene1_001.png"))?;
        File::create(dir.path().join("scene2_001.png"))?;
        let frames = collect_input_frames(dir.path(), Some("scene1_*.png".into()))?;
        assert_eq!(frames.len(), 1);
        assert!(frames[0].ends_with("scene1_001.png"));
        Ok(())
    }
}
