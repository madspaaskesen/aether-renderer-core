use std::path::PathBuf;

#[derive(Debug)]
pub struct RenderReport {
    pub output_path: PathBuf,
    pub frames_rendered: Option<usize>,
    pub ffmpeg_warnings: Vec<String>,
    pub preview: bool,
    pub notes: Option<String>,
}

impl RenderReport {
    pub fn new(
        output_path: PathBuf,
        frames_rendered: Option<usize>,
        ffmpeg_warnings: Vec<String>,
        preview: bool,
        notes: Option<String>,
    ) -> Self {
        Self {
            output_path,
            frames_rendered,
            ffmpeg_warnings,
            preview,
            notes,
        }
    }

    pub fn summary(&self) -> String {
        let mut summary = format!("✅ Rendered to: {}\n", self.output_path.display());

        if let Some(frames) = self.frames_rendered {
            summary.push_str(&format!("Frames rendered: {}\n", frames));
        } else {
            summary.push_str("Frames rendered: Unknown\n");
        }

        if !self.ffmpeg_warnings.is_empty() {
            summary.push_str("⚠️ FFmpeg Warnings:\n");
            for warning in &self.ffmpeg_warnings {
                summary.push_str(&format!("- {}\n", warning));
            }
        }

        if self.preview {
            summary.push_str("🔍 Preview mode enabled.\n");
        }

        if let Some(notes) = &self.notes {
            summary.push_str(&format!("📝 Notes: {}\n", notes));
        }

        summary
    }
}
