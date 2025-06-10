use std::time::Instant;
use aether_renderer_core::{render, RenderConfig};

#[test]
fn benchmark_render_single_frame() {
    let config = RenderConfig {
        input: "tests/testdata".into(),
        output: "tests/benchmark.webm".into(),
        format: "webm".into(),
        file_pattern: Some("frame_*.png".into()),
        fps: 30,
        fade_in: 0.0,
        fade_out: 0.0,
        bitrate: None,
        crf: None,
        preview: false,
        verbose: false,
    };

    let start = Instant::now();
    let result = render(config);
    let duration = start.elapsed();

    assert!(
        result.is_ok(),
        "Benchmark render failed: {:?}",
        result.err()
    );

    println!(
        "✅ Render completed in {:.2}s (benchmark test)",
        duration.as_secs_f64()
    );
}
