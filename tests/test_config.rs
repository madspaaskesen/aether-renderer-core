use aether_renderer_core::RenderConfig;
use std::fs;
use std::path::Path;

#[test]
fn test_parse_valid_config() {
    let json = r#"
    {
        "input": "frames/",
        "output": "out.mp4",
        "fps": 24,
        "format": "mp4",
        "fade_in": 1.0,
        "fade_out": 1.0,
        "preview": 10,
        "verbose": true,
        "verbose_ffmpeg": false
    }
    "#;

    let path = Path::new("tests/temp_config.json");
    fs::write(path, json).expect("Failed to write temp config");

    let cfg = RenderConfig::from_file(path.to_str().unwrap()).expect("Failed to parse config");
    assert_eq!(cfg.fps, 24);
    assert_eq!(cfg.format, "mp4");
    assert_eq!(cfg.fade_in, 1.0);
    assert_eq!(cfg.preview, Some(10));
    assert!(cfg.verbose);
    assert!(!cfg.verbose_ffmpeg);

    fs::remove_file(path).ok();
}

#[test]
fn test_missing_required_fields() {
    let json = r#"{ "fps": 30 }"#;
    let path = Path::new("tests/invalid_config.json");
    fs::write(path, json).expect("Failed to write temp config");

    let result = RenderConfig::from_file(path.to_str().unwrap());
    assert!(result.is_err());

    fs::remove_file(path).ok();
}

#[test]
fn example_config_parses() {
    // The shipped example uses "preview": false, which 0.4 rejected.
    let cfg = RenderConfig::from_file("examples/example_config.json").expect("example parses");
    assert!(!cfg.is_preview());
    assert_eq!(cfg.crf, Some(24));
}

#[test]
fn toml_config_and_format_from_extension() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("render.toml");
    fs::write(
        &path,
        "input = \"frames\"\noutput = \"loop.GIF\"\nfade_in = 0.5\npreview = 3\n",
    )
    .unwrap();
    let cfg = RenderConfig::from_file(path.to_str().unwrap()).expect("toml parses");
    assert_eq!(cfg.format, "gif", "format follows the output extension");
    assert_eq!(cfg.fps, 30);
    assert_eq!(cfg.fade_in, 0.5);
    assert_eq!(cfg.preview, Some(3));
}

#[test]
fn cli_values_override_config_values() {
    use aether_renderer_core::config::PartialConfig;
    let file = PartialConfig {
        input: Some("frames".into()),
        output: Some("a.webm".into()),
        fps: Some(24),
        verbose: Some(true),
        ..Default::default()
    };
    let cli = PartialConfig {
        output: Some("b.mov".into()),
        ..Default::default()
    };
    let cfg = cli.or(file).into_config().unwrap();
    assert_eq!(
        (cfg.output.as_str(), cfg.format.as_str(), cfg.fps),
        ("b.mov", "mov", 24)
    );
    assert!(cfg.verbose);
}

#[test]
fn unsupported_format_is_rejected_early() {
    use aether_renderer_core::config::PartialConfig;
    let err = PartialConfig {
        input: Some("frames".into()),
        format: Some("avi".into()),
        ..Default::default()
    }
    .into_config()
    .unwrap_err();
    assert!(err.contains("webm, mp4, gif, mov"), "{err}");
}
