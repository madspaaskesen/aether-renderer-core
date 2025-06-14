use aether_renderer_core::{render, RenderConfig, RenderReport};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_render_report_summary() {
    let report = RenderReport::new(
        PathBuf::from("output.mp4"),
        Some(42),
        vec![
            "Frame drop detected".to_string(),
            "Color mismatch warning".to_string(),
        ],
        true,
        Some("Test render completed".to_string()),
    );

    let summary = report.summary();
    assert!(summary.contains("output.mp4"));
    assert!(summary.contains("Frames rendered: 42"));
    assert!(summary.contains("⚠️ FFmpeg Warnings"));
    assert!(summary.contains("Frame drop detected"));
    assert!(summary.contains("Color mismatch warning"));
    assert!(summary.contains("🔍 Preview mode enabled."));
    assert!(summary.contains("📝 Notes: Test render completed"));
}

#[test]
fn test_render_video_from_folder() {
    let input = PathBuf::from("tests/testdata");
    let output = "tests/output/test_video.mp4";

    let cfg = RenderConfig {
        input: input.clone(),
        output: output.into(),
        fps: 30,
        format: "mp4".into(),
        fade_in: 0.0,
        fade_out: 0.0,
        bitrate: None,
        crf: Some(20),
        open: false,
        preview: None,
        file_pattern: Some("*.png".into()),
        verbose: false,
        verbose_ffmpeg: false,
    };

    let report = render(cfg).expect("Render should succeed");
    assert!(PathBuf::from(output).exists());
    assert_eq!(report.frames_rendered, Some(1));
    assert!(report.ffmpeg_warnings.is_empty());

    fs::remove_file(output).ok(); // cleanup
}

#[test]
#[ignore]
fn test_render_gif_from_zip() {
    let input = PathBuf::from("tests/testdata/two-frames.zip");
    let output = "tests/output/test.gif";

    let cfg = RenderConfig {
        input: input.clone(),
        output: output.into(),
        fps: 10,
        format: "gif".into(),
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

    let report = render(cfg).expect("GIF render should succeed");
    assert!(PathBuf::from(output).exists());
    assert_eq!(report.frames_rendered, Some(2));
    assert!(report.ffmpeg_warnings.is_empty());

    fs::remove_file(output).ok();
}
