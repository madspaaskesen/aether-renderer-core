use aether_renderer_core::{render, RenderConfig};
use std::path::PathBuf;

fn default_config() -> RenderConfig {
    RenderConfig {
        input: PathBuf::from("tests/data/frames"),
        format: "webm".into(),
        fps: 30,
        output: "out".into(), // default, overridden in test
        fade_in: 0.0,
        fade_out: 0.0,
        bitrate: None,
        crf: None,
        open: false,
        preview: None,
        file_pattern: None,
        verbose: false,
        verbose_ffmpeg: false,
    }
}

#[test]
fn test_weird_output_filenames() {
    let cases = vec![("output.💥💣💀", "weird unicode"), ("out", "no extension")];

    for (filename, desc) in cases {
        let mut cfg = default_config();
        cfg.output = filename.into();

        let result = render(cfg);
        assert!(result.is_err(), "Expected error for case: {}", desc);
    }
}
