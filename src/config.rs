use serde::{Deserialize, Deserializer};
use std::path::{Path, PathBuf};

/// Output formats the renderer can produce.
pub const FORMATS: &[&str] = &["webm", "mp4", "gif", "mov"];

/// `preview: Some(PREVIEW_MIDDLE)` previews the middle frame of the sequence.
pub const PREVIEW_MIDDLE: usize = usize::MAX;

/// Render configuration loaded from a JSON/TOML file or CLI arguments
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
    /// `None` = full render, `Some(n)` = extract frame `n` as a PNG
    #[serde(default, deserialize_with = "preview_value")]
    pub preview: Option<usize>,
    #[serde(default)]
    pub file_pattern: Option<String>,
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub verbose_ffmpeg: bool,
    /// Solid background color behind the frames, e.g. `"#1e1e1e"` or `"white"`.
    /// `None` (or `"transparent"`) keeps transparency; mp4 cannot store it and
    /// uses black. See [`parse_background`].
    #[serde(default)]
    pub background: Option<String>,
}

fn default_fps() -> u32 {
    30
}

fn default_format() -> String {
    "webm".into()
}

/// The format implied by an output file name (`out.gif` -> `gif`), if it is one we render.
pub fn format_from_extension(output: &str) -> Option<String> {
    let ext = Path::new(output).extension()?.to_str()?.to_lowercase();
    FORMATS.contains(&ext.as_str()).then_some(ext)
}

impl RenderConfig {
    /// Load configuration from a `.json` or `.toml` file.
    ///
    /// Fields that are missing fall back to defaults; when `format` is missing it is
    /// taken from the output file extension.
    pub fn from_file(path: &str) -> Result<Self, String> {
        PartialConfig::from_file(path)?.into_config()
    }

    pub fn is_preview(&self) -> bool {
        self.preview.is_some()
    }

    pub fn preview_frame_limit(&self) -> Option<usize> {
        self.preview
    }
}

/// A configuration where every field is optional: what a config file or the
/// command line actually specified. Merge several with [`PartialConfig::or`] and
/// turn the result into a [`RenderConfig`] with [`PartialConfig::into_config`].
#[derive(Debug, Default, Deserialize)]
pub struct PartialConfig {
    pub input: Option<PathBuf>,
    pub output: Option<String>,
    pub fps: Option<u32>,
    pub format: Option<String>,
    pub fade_in: Option<f32>,
    pub fade_out: Option<f32>,
    pub bitrate: Option<String>,
    pub crf: Option<u32>,
    pub open: Option<bool>,
    /// `Some(None)` = preview the middle frame, `Some(Some(n))` = frame `n`
    #[serde(default, deserialize_with = "preview_option")]
    pub preview: Option<Option<usize>>,
    pub file_pattern: Option<String>,
    pub verbose: Option<bool>,
    pub verbose_ffmpeg: Option<bool>,
    pub background: Option<String>,
}

impl PartialConfig {
    /// Read a `.json` or `.toml` config file (decided by extension; JSON otherwise).
    pub fn from_file(path: &str) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("❌ Could not read config file '{}': {}", path, e))?;
        let is_toml = Path::new(path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("toml"));
        let cfg: Self = if is_toml {
            toml::from_str(&text)
                .map_err(|e| format!("❌ Failed to parse config '{}': {}", path, e))?
        } else {
            serde_json::from_str(&text)
                .map_err(|e| format!("❌ Failed to parse config '{}': {}", path, e))?
        };
        Ok(cfg)
    }

    /// Fields set in `self` win; the rest are taken from `base`.
    pub fn or(self, base: PartialConfig) -> PartialConfig {
        PartialConfig {
            input: self.input.or(base.input),
            output: self.output.or(base.output),
            fps: self.fps.or(base.fps),
            format: self.format.or(base.format),
            fade_in: self.fade_in.or(base.fade_in),
            fade_out: self.fade_out.or(base.fade_out),
            bitrate: self.bitrate.or(base.bitrate),
            crf: self.crf.or(base.crf),
            open: self.open.or(base.open),
            preview: self.preview.or(base.preview),
            file_pattern: self.file_pattern.or(base.file_pattern),
            verbose: self.verbose.or(base.verbose),
            verbose_ffmpeg: self.verbose_ffmpeg.or(base.verbose_ffmpeg),
            background: self.background.or(base.background),
        }
    }

    /// Fill in defaults and validate.
    ///
    /// - `format` defaults to the output extension, else `webm`
    /// - `output` defaults to `output.<format>`
    pub fn into_config(self) -> Result<RenderConfig, String> {
        let input = self.input.ok_or(
            "❌ No input given. Pass --input <folder or .zip> (or \"input\" in the config).",
        )?;
        let format = match self.format {
            Some(f) => f.to_lowercase(),
            None => self
                .output
                .as_deref()
                .and_then(format_from_extension)
                .unwrap_or_else(default_format),
        };
        if !FORMATS.contains(&format.as_str()) {
            return Err(format!(
                "❌ Unsupported format '{}'. Use one of: {}.",
                format,
                FORMATS.join(", ")
            ));
        }
        let fps = self.fps.unwrap_or_else(default_fps);
        if fps == 0 {
            return Err("❌ fps must be greater than 0.".into());
        }
        let output = self.output.unwrap_or_else(|| format!("output.{format}"));
        let fade_in = self.fade_in.unwrap_or(0.0);
        let fade_out = self.fade_out.unwrap_or(0.0);
        if fade_in < 0.0 || fade_out < 0.0 {
            return Err("❌ Fade durations cannot be negative.".into());
        }
        let background = match self.background {
            Some(b) => parse_background(&b)?,
            None => None,
        };
        Ok(RenderConfig {
            input,
            output,
            fps,
            format,
            fade_in,
            fade_out,
            bitrate: self.bitrate,
            crf: self.crf,
            open: self.open.unwrap_or(false),
            // Previewing without a frame number means "the middle frame",
            // which preview_frame picks when it gets None.
            preview: self.preview.map(|n| n.unwrap_or(PREVIEW_MIDDLE)),
            file_pattern: self.file_pattern,
            verbose: self.verbose.unwrap_or(false),
            verbose_ffmpeg: self.verbose_ffmpeg.unwrap_or(false),
            background,
        })
    }
}

/// Turn a background color as a user writes it into one ffmpeg understands.
///
/// - `""`, `"transparent"`, `"none"` → `None` (keep transparency)
/// - `"#rgb"`, `"#rrggbb"`, `"rrggbb"`, `"0xrrggbb"` → `Some("0xRRGGBB")`
/// - a color name such as `"white"` or `"DarkSlateGray"` → `Some("darkslategray")`
///   (ffmpeg checks the name when rendering)
pub fn parse_background(value: &str) -> Result<Option<String>, String> {
    let v = value.trim();
    if v.is_empty() || v.eq_ignore_ascii_case("transparent") || v.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let hex = v
        .strip_prefix('#')
        .or_else(|| v.strip_prefix("0x"))
        .or_else(|| v.strip_prefix("0X"))
        .unwrap_or(v);
    let is_hex = hex.chars().all(|c| c.is_ascii_hexdigit());
    if is_hex && hex.len() == 6 {
        return Ok(Some(format!("0x{}", hex.to_uppercase())));
    }
    if is_hex && hex.len() == 3 {
        let long: String = hex.chars().flat_map(|c| [c, c]).collect();
        return Ok(Some(format!("0x{}", long.to_uppercase())));
    }
    if hex.len() == v.len() && v.chars().all(|c| c.is_ascii_alphabetic()) {
        return Ok(Some(v.to_lowercase()));
    }
    Err(format!(
        "❌ Invalid background color '{}'. Use a hex color like '#1e1e1e', a name like 'white', or 'transparent'.",
        value
    ))
}

/// Accepts `"preview": false | true | null | <frame number>` in config files.
#[derive(Deserialize)]
#[serde(untagged)]
enum PreviewValue {
    Flag(bool),
    Frame(usize),
}

/// For `RenderConfig`: `false`/`null` = no preview, `true` = middle frame, `n` = frame n.
fn preview_value<'de, D: Deserializer<'de>>(d: D) -> Result<Option<usize>, D::Error> {
    Ok(match Option::<PreviewValue>::deserialize(d)? {
        None | Some(PreviewValue::Flag(false)) => None,
        Some(PreviewValue::Flag(true)) => Some(PREVIEW_MIDDLE),
        Some(PreviewValue::Frame(n)) => Some(n),
    })
}

/// For `PartialConfig`: same values, but keeps "not set" apart from "no preview".
fn preview_option<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<usize>>, D::Error> {
    Ok(match Option::<PreviewValue>::deserialize(d)? {
        None | Some(PreviewValue::Flag(false)) => None,
        Some(PreviewValue::Flag(true)) => Some(None),
        Some(PreviewValue::Frame(n)) => Some(Some(n)),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_background;

    #[test]
    fn background_values() {
        let p = |v: &str| parse_background(v).unwrap();
        assert_eq!(p(""), None);
        assert_eq!(p(" transparent "), None);
        assert_eq!(p("None"), None);
        assert_eq!(p("#1e1e1e").as_deref(), Some("0x1E1E1E"));
        assert_eq!(p("1e1e1e").as_deref(), Some("0x1E1E1E"));
        assert_eq!(p("0xff8800").as_deref(), Some("0xFF8800"));
        assert_eq!(p("#fff").as_deref(), Some("0xFFFFFF"));
        assert_eq!(p("White").as_deref(), Some("white"));
        // "bad" is valid hex but 3 letters: treated as #bbaadd
        assert_eq!(p("#bad").as_deref(), Some("0xBBAADD"));
        assert!(parse_background("#12345").is_err());
        assert!(parse_background("red; rm -rf").is_err());
        assert!(parse_background("#1e1e1e80").is_err());
    }
}
