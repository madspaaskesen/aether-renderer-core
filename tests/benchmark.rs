use aether_renderer_core::{render, RenderConfig};
use std::process::Command;
use std::time::Instant;

#[test]
fn benchmark_render_single_frame() -> Result<(), Box<dyn std::error::Error>> {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("skipping cli_renders_mp4 - ffmpeg not installed");
        return Ok(());
    }
    let config = RenderConfig {
        input: "tests/testdata".into(),
        output: "tests/testdata/benchmark.webm".into(),
        format: "webm".into(),
        file_pattern: Some("frame_*.png".into()),
        fps: 30,
        fade_in: 0.0,
        fade_out: 0.0,
        bitrate: None,
        crf: None,
        open: false,
        verbose: false,
        verbose_ffmpeg: false,
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
    // Clean up output file after benchmark
    let _ = std::fs::remove_file("tests/testdata/benchmark.webm");
    println!("Output file cleaned up after benchmark.");
    Ok(())
}
