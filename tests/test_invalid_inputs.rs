use aether_renderer_core::{render, RenderConfig};
use std::path::PathBuf;

#[test]
fn test_nonexistent_input_path() {
    let cfg = RenderConfig {
        input: PathBuf::from("nonexistent_folder/"),
        output: "out.mp4".into(),
        fps: 30,
        format: "mp4".into(),
        fade_in: 0.0,
        fade_out: 0.0,
        bitrate: None,
        crf: None,
        open: false,
        preview: None,
        file_pattern: None,
        verbose: false,
        verbose_ffmpeg: false,
    };

    let result = render(cfg);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("does not exist"));
}

#[test]
fn test_unsupported_format() {
    let cfg = RenderConfig {
        input: PathBuf::from("frames/"), // this can be mocked or skipped during dry run
        output: "out.avi".into(),
        fps: 30,
        format: "avi".into(), // unsupported
        fade_in: 0.0,
        fade_out: 0.0,
        bitrate: None,
        crf: None,
        open: false,
        preview: None,
        file_pattern: None,
        verbose: false,
        verbose_ffmpeg: false,
    };

    let result = render(cfg);
    assert!(result.is_err());
    let err = result.unwrap_err();
    print!("Error: {}", err);
    assert!(err.contains("Unsupported format"));
}

#[test]
fn test_unsupported_format2() {
    let cfg = RenderConfig {
        input: PathBuf::from("frames/"), // this can be mocked or skipped during dry run
        output: "out.avi".into(),        // semi invalid!
        fps: 30,
        format: "webm".into(),
        fade_in: 0.0,
        fade_out: 0.0,
        bitrate: None,
        crf: None,
        open: false,
        preview: None,
        file_pattern: None,
        verbose: false,
        verbose_ffmpeg: false,
    };

    let result = render(cfg);
    assert!(result.is_err());
    let err = result.unwrap_err();
    print!("Error: {}", err);
    assert!(err.contains("Input path 'frames/' does not exist"));
}

#[test]
fn test_unsupported_format3() {
    let cfg = RenderConfig {
        input: PathBuf::from("tests/testdata/two-frames.zip"), // this can be mocked or skipped during dry run
        output: "out.avi".into(),                              // semi invalid!
        fps: 30,
        format: "webm".into(),
        fade_in: 0.0,
        fade_out: 0.0,
        bitrate: None,
        crf: None,
        open: false,
        preview: None,
        file_pattern: None,
        verbose: false,
        verbose_ffmpeg: false,
    };

    let result = render(cfg);
    //assert!(result.is_err());
    //let err = result.unwrap_err();
    //print!("Error: {}", result.unwrap());
    //assert!(result.is_ok_and(f).contains("⚠️ Warning: Output extension 'avi' does not match format 'webm'"));
    let report = result.unwrap();
    assert!(report.notes.unwrap_or_default().contains("⚠️ Warning"));
}
