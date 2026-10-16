//! Regression tests for 0.5.0: every format from odd-sized transparent frames,
//! config files, CLI overrides and error handling. Skipped without ffmpeg.

use std::path::Path;
use std::process::Command;
use tempfile::{tempdir, TempDir};

fn have_ffmpeg() -> bool {
    let ok = |cmd: &str| {
        Command::new(cmd)
            .arg("-version")
            .output()
            .is_ok_and(|o| o.status.success())
    };
    ok("ffmpeg") && ok("ffprobe")
}

macro_rules! require_ffmpeg {
    () => {
        if !have_ffmpeg() {
            eprintln!("skipping: ffmpeg/ffprobe not installed");
            return;
        }
    };
}

/// 12 transparent 321x241 frames (odd size, like many canvas exports) with an
/// opaque square in the middle, named without zero padding: f_0.png … f_11.png.
fn frames() -> TempDir {
    let dir = tempdir().unwrap();
    let status = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black@0.0:s=321x241:r=12:d=1,format=rgba",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=101x81:r=12:d=1,format=rgba",
            "-filter_complex",
            "[0][1]overlay=110:80",
            "-start_number",
            "0",
        ])
        .arg(dir.path().join("f_%d.png"))
        .status()
        .unwrap();
    assert!(status.success());
    dir
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aether-renderer-core"))
}

/// codec, width, height, pix_fmt, frame rate, duration
fn probe(file: &Path) -> String {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name,width,height,pix_fmt,r_frame_rate:format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(file)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).replace('\n', ",")
}

/// Alpha of the corner and center pixel of frame `n`.
fn alpha_at(file: &Path, n: u32, decoder: &[&str]) -> (u8, u8) {
    let out = Command::new("ffmpeg")
        .args(["-v", "error"])
        .args(decoder)
        .arg("-i")
        .arg(file)
        .args([
            "-vf",
            &format!("select=eq(n\\,{n})"),
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-",
        ])
        .output()
        .unwrap();
    let px = out.stdout;
    let (w, h) = (321usize, 241usize);
    assert_eq!(px.len(), w * h * 4, "unexpected frame size");
    let center = ((h / 2) * w + w / 2) * 4 + 3;
    (px[3], px[center])
}

fn render(args: &[&str]) -> std::process::Output {
    let out = bin().args(args).output().unwrap();
    if !out.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
    }
    out
}

#[test]
fn webm_is_vp9_with_alpha() {
    require_ffmpeg!();
    let f = frames();
    let out = f.path().join("out.webm");
    assert!(render(&[
        "-i",
        f.path().to_str().unwrap(),
        "-o",
        out.to_str().unwrap()
    ])
    .status
    .success());
    assert!(probe(&out).starts_with("vp9,321,241,"), "{}", probe(&out));
    assert_eq!(alpha_at(&out, 6, &["-c:v", "libvpx-vp9"]), (0, 255));
}

#[test]
fn mp4_with_odd_dimensions_is_padded() {
    require_ffmpeg!();
    let f = frames();
    let out = f.path().join("out.mp4");
    assert!(render(&[
        "-i",
        f.path().to_str().unwrap(),
        "-o",
        out.to_str().unwrap()
    ])
    .status
    .success());
    assert!(
        probe(&out).starts_with("h264,322,242,yuv420p"),
        "{}",
        probe(&out)
    );
}

#[test]
fn gif_keeps_size_fps_and_transparency() {
    require_ffmpeg!();
    let f = frames();
    let out = f.path().join("out.gif");
    let dir = f.path().to_str().unwrap();
    assert!(
        render(&["-i", dir, "-o", out.to_str().unwrap(), "--fps", "12"])
            .status
            .success()
    );
    let info = probe(&out);
    assert!(info.starts_with("gif,321,241,"), "{info}");
    // GIF delays are in 1/100 s, so 12 fps is stored as ~11.99 (12000/1001).
    assert!(
        info.contains(",12/1,") || info.contains(",12000/1001,"),
        "{info}"
    );
    assert!(info.contains(",1.0"), "12 frames at 12 fps = 1 s: {info}");
    assert_eq!(alpha_at(&out, 6, &[]), (0, 255));
    assert!(!Path::new("palette.png").exists(), "no palette file in cwd");
}

#[test]
fn mov_is_prores_4444_with_alpha() {
    require_ffmpeg!();
    let f = frames();
    let out = f.path().join("out.mov");
    assert!(render(&[
        "-i",
        f.path().to_str().unwrap(),
        "-o",
        out.to_str().unwrap()
    ])
    .status
    .success());
    let info = probe(&out);
    assert!(info.starts_with("prores,321,241,yuva444p"), "{info}");
    assert_eq!(alpha_at(&out, 6, &[]), (0, 255));
}

#[test]
fn fades_go_to_transparent() {
    require_ffmpeg!();
    let f = frames();
    let out = f.path().join("fade.webm");
    let dir = f.path().to_str().unwrap();
    assert!(render(&[
        "-i",
        dir,
        "-o",
        out.to_str().unwrap(),
        "--fps",
        "12",
        "--fade-in",
        "0.5",
        "--fade-out",
        "0.5",
    ])
    .status
    .success());
    let vp9 = ["-c:v", "libvpx-vp9"];
    assert_eq!(alpha_at(&out, 0, &vp9).1, 0, "first frame fully faded");
    assert_eq!(alpha_at(&out, 6, &vp9).1, 255, "middle untouched");
    assert!(alpha_at(&out, 11, &vp9).1 < 64, "fading out at the end");
}

#[test]
fn numbered_pattern_and_unpadded_names() {
    require_ffmpeg!();
    let f = frames();
    std::fs::write(f.path().join("notes.txt"), "not a frame").unwrap();
    let out = f.path().join("seq.webm");
    let dir = f.path().to_str().unwrap();
    let res = render(&[
        "-i",
        dir,
        "-o",
        out.to_str().unwrap(),
        "--file-pattern",
        "f_%d.png",
        "--fps",
        "12",
        "-v",
    ]);
    assert!(res.status.success());
    assert!(String::from_utf8_lossy(&res.stdout).contains("Rendering 12 frames"));
}

#[test]
fn toml_config_with_cli_override() {
    require_ffmpeg!();
    let f = frames();
    let cfg = f.path().join("render.toml");
    let from_file = f.path().join("from_file.webm");
    std::fs::write(
        &cfg,
        format!(
            "input = {:?}\noutput = {:?}\nfps = 12\npreview = false\n",
            f.path(),
            from_file
        ),
    )
    .unwrap();
    // Output and format come from the command line, everything else from the file.
    let gif = f.path().join("override.gif");
    assert!(
        render(&["-c", cfg.to_str().unwrap(), "-o", gif.to_str().unwrap()])
            .status
            .success()
    );
    assert!(gif.exists());
    assert!(!from_file.exists(), "CLI --output must override the config");
    assert!(
        probe(&gif).contains(",1.0"),
        "fps 12 from the file: {}",
        probe(&gif)
    );
}

#[test]
fn failed_render_reports_ffmpeg_reason_and_leaves_no_file() {
    require_ffmpeg!();
    let f = frames();
    let out = f.path().join("bad.mp4");
    let res = render(&[
        "-i",
        f.path().to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--bitrate",
        "nonsense",
    ]);
    assert!(!res.status.success());
    let err = String::from_utf8_lossy(&res.stderr);
    assert!(err.contains("ffmpeg exited with code"), "{err}");
    assert!(err.contains("nonsense"), "ffmpeg's reason is shown: {err}");
    assert!(!out.exists(), "no half-written output");
}

#[test]
fn preview_flag_without_number_saves_middle_frame() {
    let f = tempdir().unwrap();
    for i in 0..5 {
        std::fs::write(f.path().join(format!("f_{i}.png")), format!("frame {i}")).unwrap();
    }
    let out = f.path().join("p.webm");
    assert!(render(&[
        "-i",
        f.path().to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--preview"
    ])
    .status
    .success());
    let png = f.path().join("p.png");
    assert_eq!(std::fs::read_to_string(png).unwrap(), "frame 2");
    assert!(!out.exists(), "preview must not do a full render");
}

#[test]
fn no_arguments_prints_help_and_fails() {
    let out = bin().output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("Usage"));
}
