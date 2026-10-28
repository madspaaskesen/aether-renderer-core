pub mod gif;
/// FFmpeg rendering helpers for different output formats
pub mod video;

use std::io::Read;

/// Where ffmpeg reads the frames from.
pub(crate) enum FfmpegInput<'a> {
    /// An image input such as `frames/%06d.png` (or a glob, for callers that pass one).
    Sequence(&'a str),
    /// Concatenated PNG files on ffmpeg's stdin, fed from this reader.
    PngStream(Box<dyn Read + Send>),
}

impl FfmpegInput<'_> {
    /// The ffmpeg input arguments, and the reader to feed to its stdin, if any.
    pub(crate) fn into_args(self, fps: u32) -> (Vec<String>, Option<Box<dyn Read + Send>>) {
        let fps = fps.to_string();
        match self {
            Self::Sequence(pattern) => {
                let mut args = vec!["-framerate".into(), fps];
                if pattern.contains('*') {
                    args.extend(["-pattern_type".into(), "glob".into()]);
                }
                args.extend(["-i".into(), pattern.to_string()]);
                (args, None)
            }
            Self::PngStream(reader) => (
                vec![
                    "-f".into(),
                    "image2pipe".into(),
                    "-framerate".into(),
                    fps,
                    "-c:v".into(),
                    "png".into(),
                    "-i".into(),
                    "pipe:0".into(),
                ],
                Some(reader),
            ),
        }
    }
}
