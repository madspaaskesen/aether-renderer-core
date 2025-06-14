use aether_renderer_core::RenderReport;
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
