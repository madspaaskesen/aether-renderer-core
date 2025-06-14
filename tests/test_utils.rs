use aether_renderer_core::utils::{run_ffmpeg_with_output, scan_ffmpeg_stderr};

#[test]
fn test_scan_ffmpeg_stderr_detects_warnings() {
    let fake_stderr = "\"
[warning] Past duration 0.999998 too large
[Parsed_fps_0 @ 0x7fd660001c40] Frame rate very high for a muxer not efficiently supporting it.
[Parsed_scale_0 @ 0x7fd660001b40] deprecated option 'flags' used
\"";
    let warnings = scan_ffmpeg_stderr(&fake_stderr);

    assert!(warnings.iter().any(|w| w.contains("Past frame duration")));
    assert!(warnings.iter().any(|w| w.contains("Frame rate very high")));
    assert!(warnings.iter().any(|w| w.contains("Deprecated option")));
}

#[test]
fn test_run_ffmpeg_with_output_ffmpeg_not_found() {
    let result = run_ffmpeg_with_output(&[String::from("-version-NOTREAL")]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    println!("Error: {}", err);
    assert!(err.contains("❌ ffmpeg exited with code"));
}
