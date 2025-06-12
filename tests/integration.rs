use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;
use zip::ZipArchive;

fn unzip_to_dir(zip_path: &Path, out_dir: &Path) -> zip::result::ZipResult<()> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut zipped = archive.by_index(i)?;
        let out_path = out_dir.join(zipped.name());

        if zipped.is_dir() {
            std::fs::create_dir_all(&out_path)?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut out_file = File::create(out_path)?;
        std::io::copy(&mut zipped, &mut out_file)?;
    }
    Ok(())
}

#[test]
fn cli_renders_webm() -> Result<(), Box<dyn std::error::Error>> {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("skipping cli_renders_webm - ffmpeg not installed");
        return Ok(());
    }
    let zip_path = Path::new("tests/testdata/two-frames.zip");
    assert!(
        zip_path.exists(),
        "test zip not found: {}",
        zip_path.display()
    );

    let tmp = tempdir()?;
    unzip_to_dir(zip_path, tmp.path())?;

    let out_file = tmp.path().join("out.webm");
    let config_path = tmp.path().join("config.json");
    let mut f = File::create(&config_path)?;
    writeln!(
        f,
        "{{\"input\":\"{}\",\"output\":\"{}\",\"fps\":30,\"format\":\"webm\"}}",
        tmp.path().display(),
        out_file.to_str().unwrap()
    )?;

    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .status()?;

    assert!(status.success(), "cargo run failed");
    assert!(out_file.exists(), "output file was not created");
    let meta = fs::metadata(&out_file)?;
    assert!(meta.len() > 0, "output file is empty");
    // TempDir cleans up automatically
    Ok(())
}

#[test]
fn cli_renders_mp4() -> Result<(), Box<dyn std::error::Error>> {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("skipping cli_renders_mp4 - ffmpeg not installed");
        return Ok(());
    }
    let zip_path = Path::new("tests/testdata/two-frames.zip");
    assert!(
        zip_path.exists(),
        "test zip not found: {}",
        zip_path.display()
    );

    let tmp = tempdir()?;
    unzip_to_dir(zip_path, tmp.path())?;

    let out_file = tmp.path().join("out.mp4");
    let config_path = tmp.path().join("config.json");
    let mut f = File::create(&config_path)?;
    writeln!(
        f,
        "{{\"input\":\"{}\",\"output\":\"{}\",\"fps\":30,\"format\":\"mp4\"}}",
        tmp.path().display(),
        out_file.to_str().unwrap()
    )?;

    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .status()?;

    assert!(status.success(), "cargo run failed");
    assert!(out_file.exists(), "output file was not created");
    let meta = fs::metadata(&out_file)?;
    assert!(meta.len() > 0, "output file is empty");
    Ok(())
}

#[test]
fn cli_errors_on_invalid_zip() -> Result<(), Box<dyn std::error::Error>> {
    let zip_path = Path::new("tests/testdata/two-frames-error.zip");
    assert!(
        zip_path.exists(),
        "test zip not found: {}",
        zip_path.display()
    );

    let tmp_dir = tempdir()?;
    let config_path = tmp_dir.path().join("config.json");
    let mut f = File::create(&config_path)?;
    writeln!(
        f,
        "{{\"input\":\"{}\",\"output\":\"out.webm\",\"fps\":30,\"format\":\"webm\"}}",
        zip_path.display()
    )?;

    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .status()?;

    assert!(!status.success(), "expected failure for invalid zip input");
    Ok(())
}

#[test]
fn cli_errors_on_missing_folder() -> Result<(), Box<dyn std::error::Error>> {
    let missing = Path::new("tests/testdata/does_not_exist");
    assert!(!missing.exists());

    let tmp = tempdir()?;
    let config_path = tmp.path().join("config.json");
    let mut f = File::create(&config_path)?;
    writeln!(
        f,
        "{{\"input\":\"{}\",\"output\":\"out.webm\"}}",
        missing.display()
    )?;

    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .status()?;

    assert!(
        !status.success(),
        "expected failure for missing input folder"
    );
    Ok(())
}

#[test]
fn cli_errors_on_empty_folder() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempdir()?;
    let config_path = tmp.path().join("config.json");
    let mut f = File::create(&config_path)?;
    writeln!(
        f,
        "{{\"input\":\"{}\",\"output\":\"out.webm\"}}",
        tmp.path().display()
    )?;

    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            config_path.to_str().unwrap(),
        ])
        .status()?;

    assert!(!status.success(), "expected failure for empty input folder");
    Ok(())
}

#[test]
fn cli_preview_from_zip() -> Result<(), Box<dyn std::error::Error>> {
    let zip_path = Path::new("tests/testdata/two-frames.zip");
    assert!(zip_path.exists());

    let tmp = tempdir()?;
    let out_file = tmp.path().join("prev.png");

    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--input",
            zip_path.to_str().unwrap(),
            "--output",
            out_file.to_str().unwrap(),
            "--preview",
            "1",
        ])
        .status()?;

    assert!(status.success(), "cargo run failed");
    assert!(out_file.exists(), "preview image not created");
    Ok(())
}
