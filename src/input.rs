use glob::glob;
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// Collect files from `input_folder` matching the optional pattern, in natural
/// order (`frame_2.png` before `frame_10.png`). Defaults to `*.png`.
///
/// Besides glob patterns (`*.png`, `scene1_*.png`) this accepts ffmpeg-style
/// sequence patterns such as `frame_%04d.png`.
pub fn collect_input_frames(
    input_folder: &Path,
    file_pattern: Option<String>,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let pattern = file_pattern.unwrap_or_else(|| "*.png".to_string());
    let (glob_pattern, sequence) = match sequence_to_glob(&pattern) {
        Some(g) => (g, Some(pattern.clone())),
        None => (pattern.clone(), None),
    };
    // Escape the folder so names like "renders [final]" are taken literally.
    let folder = glob::Pattern::escape(&input_folder.to_string_lossy());
    let glob_path = format!("{}/{}", folder, glob_pattern);
    let mut frames: Vec<PathBuf> = glob(&glob_path)?
        .filter_map(Result::ok)
        .filter(|p| p.is_file())
        .filter(|p| {
            // Skip macOS resource forks like ._frame_0001.png
            !p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("._"))
        })
        .filter(|p| match &sequence {
            Some(seq) => matches_sequence(seq, p),
            None => true,
        })
        .collect();
    frames.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
    Ok(frames)
}

/// `frame_%04d.png` -> `frame_*.png`; `None` if the pattern has no `%d`.
fn sequence_to_glob(pattern: &str) -> Option<String> {
    let mut out = String::new();
    let mut found = false;
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let mut spec = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    spec.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if chars.peek() == Some(&'d') {
                chars.next();
                out.push('*');
                found = true;
                continue;
            }
            out.push('%');
            out.push_str(&spec);
        } else {
            out.push(c);
        }
    }
    found.then_some(out)
}

/// Does the file name match `frame_%04d.png` exactly (the `*` from the glob is digits)?
fn matches_sequence(seq: &str, path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let Some(start) = seq.find('%') else {
        return true;
    };
    let Some(d) = seq[start..].find('d').map(|i| start + i) else {
        return true;
    };
    let (prefix, suffix) = (&seq[..start], &seq[d + 1..]);
    let width: usize = seq[start + 1..d]
        .trim_start_matches('0')
        .parse()
        .unwrap_or(0);
    name.len() > prefix.len() + suffix.len()
        && name.starts_with(prefix)
        && name.ends_with(suffix)
        && {
            let digits = &name[prefix.len()..name.len() - suffix.len()];
            digits.chars().all(|c| c.is_ascii_digit()) && digits.len() >= width
        }
}

/// Compare strings with digit runs compared as numbers: `a2 < a10`.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.as_bytes(), b.as_bytes());
    loop {
        match (a.first(), b.first()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let na = a.iter().take_while(|c| c.is_ascii_digit()).count();
                let nb = b.iter().take_while(|c| c.is_ascii_digit()).count();
                let (da, db) = (&a[..na], &b[..nb]);
                let ta = trim_zeros(da);
                let tb = trim_zeros(db);
                let ord = ta
                    .len()
                    .cmp(&tb.len())
                    .then_with(|| ta.cmp(tb))
                    .then_with(|| na.cmp(&nb));
                if ord != Ordering::Equal {
                    return ord;
                }
                a = &a[na..];
                b = &b[nb..];
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(y);
                }
                a = &a[1..];
                b = &b[1..];
            }
        }
    }
}

fn trim_zeros(d: &[u8]) -> &[u8] {
    let i = d.iter().position(|&c| c != b'0').unwrap_or(d.len());
    &d[i..]
}

/// A numbered copy of a frame list that ffmpeg can read on every platform.
///
/// ffmpeg's glob input (`-pattern_type glob`) does not exist on Windows, and its
/// numbered input stops at the first gap. So the frames are linked into a temp
/// folder as `000000.png`, `000001.png`, … (symlinks on Unix, hard links or copies
/// on Windows), and ffmpeg reads `<temp>/%06d.png`.
pub struct FrameSequence {
    _dir: tempfile::TempDir,
    pattern: PathBuf,
}

impl FrameSequence {
    pub fn new(frames: &[PathBuf]) -> std::io::Result<Self> {
        let dir = tempfile::Builder::new()
            .prefix("aether-frames-")
            .tempdir()?;
        let ext = frames
            .first()
            .and_then(|f| f.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_lowercase();
        for (i, frame) in frames.iter().enumerate() {
            let target = dir.path().join(format!("{i:06}.{ext}"));
            link_or_copy(frame, &target)?;
        }
        let pattern = dir.path().join(format!("%06d.{ext}"));
        Ok(Self { _dir: dir, pattern })
    }

    /// The `-i` argument for ffmpeg.
    pub fn pattern(&self) -> &Path {
        &self.pattern
    }
}

fn link_or_copy(src: &Path, dst: &Path) -> std::io::Result<()> {
    let src = std::fs::canonicalize(src)?;
    #[cfg(unix)]
    if std::os::unix::fs::symlink(&src, dst).is_ok() {
        return Ok(());
    }
    if std::fs::hard_link(&src, dst).is_ok() {
        return Ok(());
    }
    std::fs::copy(&src, dst).map(|_| ())
}

/// Whether a PNG file can hold transparency: an alpha channel (RGBA or gray +
/// alpha) or a `tRNS` chunk. Returns `false` for anything that is not a readable PNG.
pub fn png_has_alpha(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    if bytes.len() < 33 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return false;
    }
    // IHDR is always first: color type 4 = gray + alpha, 6 = RGBA.
    if matches!(bytes[25], 4 | 6) {
        return true;
    }
    // Palette / gray / RGB images can still mark colors transparent in tRNS,
    // which comes before the image data.
    let mut pos = 8;
    while pos + 8 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
            as usize;
        match &bytes[pos + 4..pos + 8] {
            b"tRNS" => return true,
            b"IDAT" | b"IEND" => return false,
            _ => pos += 12 + len,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
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

    #[test]
    fn numbered_pattern_and_natural_order() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        for name in [
            "f_10.png",
            "f_9.png",
            "f_1.png",
            "f_x.png",
            "._f_2.png",
            "g_3.png",
        ] {
            File::create(dir.path().join(name))?;
        }
        let names = |v: Vec<PathBuf>| -> Vec<String> {
            v.iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect()
        };
        let frames = collect_input_frames(dir.path(), Some("f_%d.png".into()))?;
        assert_eq!(names(frames), ["f_1.png", "f_9.png", "f_10.png"]);
        let frames = collect_input_frames(dir.path(), None)?;
        assert_eq!(
            names(frames),
            ["f_1.png", "f_9.png", "f_10.png", "f_x.png", "g_3.png"]
        );
        Ok(())
    }

    #[test]
    fn folder_with_glob_characters() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let sub = dir.path().join("renders [final]");
        std::fs::create_dir(&sub)?;
        File::create(sub.join("a.png"))?;
        assert_eq!(collect_input_frames(&sub, None)?.len(), 1);
        Ok(())
    }

    #[test]
    fn natural_cmp_orders_numbers() {
        assert_eq!(natural_cmp("a2", "a10"), Ordering::Less);
        assert_eq!(natural_cmp("a010", "a9"), Ordering::Greater);
        assert_eq!(natural_cmp("a01", "a1"), Ordering::Greater);
        assert_eq!(natural_cmp("b", "a9"), Ordering::Greater);
    }

    #[test]
    fn sequence_links_frames_in_order() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let a = dir.path().join("x 1.png");
        let b = dir.path().join("x 2.png");
        std::fs::write(&a, b"one")?;
        std::fs::write(&b, b"two")?;
        let seq = FrameSequence::new(&[b.clone(), a.clone()])?;
        let folder = seq.pattern().parent().unwrap();
        assert_eq!(std::fs::read(folder.join("000000.png"))?, b"two");
        assert_eq!(std::fs::read(folder.join("000001.png"))?, b"one");
        assert!(seq.pattern().ends_with("%06d.png"));
        Ok(())
    }
}
