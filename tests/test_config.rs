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
