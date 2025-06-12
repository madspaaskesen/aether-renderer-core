use serde::Deserialize;
use std::path::PathBuf;

/// Render configuration loaded from a JSON file or CLI arguments
#[derive(Debug, Deserialize)]
pub struct RenderConfig {
    pub input: PathBuf,
    pub output: String,
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub fade_in: f32,
    #[serde(default)]
    pub fade_out: f32,
    #[serde(default)]
    pub bitrate: Option<String>,
    #[serde(default)]
    pub crf: Option<u32>,
    #[serde(default)]
    pub open: bool,
    #[serde(default)]
    pub file_pattern: Option<String>,
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub verbose_ffmpeg: bool,
}

fn default_fps() -> u32 {
    30
}

fn default_format() -> String {
    "webm".into()
}

impl RenderConfig {
    /// Load configuration from a JSON file
    pub fn from_file(path: &str) -> Result<Self, String> {
        let config_str = std::fs::read_to_string(path)
            .map_err(|_| format!("❌ Config file '{}' not found.", path))?;
        serde_json::from_str(&config_str).map_err(|e| format!("❌ Failed to parse config: {}", e))
    }
}
