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
    // .lines() also strips the \r of Windows line endings.
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().map(|l| format!("{l},")).collect()
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

/// RGBA of pixel (x, y) in frame `n` of a `w`×`h` video.
fn pixel(file: &Path, n: u32, (w, h): (usize, usize), (x, y): (usize, usize)) -> [u8; 4] {
    let decoder: &[&str] = if file.extension().is_some_and(|e| e == "webm") {
        &["-c:v", "libvpx-vp9"]
    } else {
        &[]
    };
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
    assert_eq!(px.len(), w * h * 4, "unexpected frame size");
    let i = (y * w + x) * 4;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

fn close_to(got: [u8; 4], want: [u8; 4]) -> bool {
    got.iter()
        .zip(want)
        .all(|(&g, w)| (g as i16 - w as i16).abs() <= 14)
}

#[test]
fn background_color_works_for_every_format() {
    require_ffmpeg!();
    let f = frames();
    let dir = f.path().to_str().unwrap();
    let blue = [0x20, 0x40, 0xff, 255];
    let red = [255, 0, 0, 255];
    for (fmt, size) in [
        ("webm", (321, 241)),
        ("mov", (321, 241)),
        ("gif", (321, 241)),
        ("mp4", (322, 242)),
    ] {
        let out = f.path().join(format!("bg.{fmt}"));
        let res = render(&[
            "-i",
            dir,
            "-o",
            out.to_str().unwrap(),
            "--fps",
            "12",
            "--background",
            "#2040ff",
            "--fade-in",
            "0.5",
        ]);
        assert!(res.status.success(), "{fmt}");
        let corner = pixel(&out, 6, size, (0, 0));
        let center = pixel(&out, 6, size, (160, 120));
        let faded = pixel(&out, 0, size, (160, 120));
        assert!(close_to(corner, blue), "{fmt} corner {corner:?}");
        assert!(close_to(center, red), "{fmt} center {center:?}");
        assert!(
            close_to(faded, blue),
            "{fmt} fades to the background {faded:?}"
        );
        if fmt == "mp4" {
            let pad = pixel(&out, 6, size, (321, 100));
            assert!(close_to(pad, blue), "mp4 padding {pad:?}");
        }
    }
}

#[test]
fn mp4_without_background_is_black_and_says_so() {
    require_ffmpeg!();
    let f = frames();
    let out = f.path().join("plain.mp4");
    let res = render(&[
        "-i",
        f.path().to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert!(res.status.success());
    let corner = pixel(&out, 6, (322, 242), (0, 0));
    assert!(close_to(corner, [0, 0, 0, 255]), "{corner:?}");
    let stdout = String::from_utf8_lossy(&res.stdout);
    assert!(stdout.contains("--background"), "{stdout}");
}

#[test]
fn invalid_background_is_rejected() {
    require_ffmpeg!();
    let f = frames();
    let out = f.path().join("x.webm");
    let res = render(&[
        "-i",
        f.path().to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--background",
        "#12345",
    ]);
    assert!(!res.status.success());
    assert!(String::from_utf8_lossy(&res.stderr).contains("Invalid background color"));
    assert!(!out.exists());
}

#[test]
fn progress_json_lines() {
    require_ffmpeg!();
    let f = frames();
    let out = f.path().join("p.webm");
    let res = render(&[
        "-i",
        f.path().to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--progress",
        "json",
    ]);
    assert!(res.status.success());
    let events: Vec<serde_json::Value> = String::from_utf8_lossy(&res.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).expect("every stdout line is JSON"))
        .collect();
    let progress: Vec<&serde_json::Value> =
        events.iter().filter(|e| e["event"] == "progress").collect();
    assert_eq!(progress.first().unwrap()["frame"], 0);
    let last = progress.last().unwrap();
    assert_eq!(last["frame"], 12);
    assert_eq!(last["total_frames"], 12);
    assert_eq!(last["percent"], 100.0);
    let done = events.last().unwrap();
    assert_eq!(done["event"], "done");
    assert_eq!(done["frames"], 12);

    // Errors are reported as JSON too.
    let res = render(&["-i", "does/not/exist", "-o", "x.webm", "--progress", "json"]);
    assert!(!res.status.success());
    let line = String::from_utf8_lossy(&res.stdout);
    let err: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(err["event"], "error");
}

#[test]
fn library_progress_callback() {
    require_ffmpeg!();
    let f = frames();
    let cfg = aether_renderer_core::config::PartialConfig {
        input: Some(f.path().to_path_buf()),
        output: Some(f.path().join("cb.mp4").to_string_lossy().into_owned()),
        ..Default::default()
    }
    .into_config()
    .unwrap();
    let mut seen = Vec::new();
    aether_renderer_core::render_with_progress(cfg, |p| seen.push(p)).unwrap();
    assert_eq!(seen.first().unwrap().frame, 0);
    assert_eq!(seen.last().unwrap().frame, 12);
    assert_eq!(seen.last().unwrap().percent, 100.0);
    assert!(seen.windows(2).all(|w| w[0].frame <= w[1].frame));
}

// ---------------------------------------------------------------------------
// PNG stream on stdin (`--input -`), 0.7.0

/// The test frames as one PNG stream, in frame order (f_0.png … f_11.png).
fn png_stream(dir: &Path) -> Vec<u8> {
    (0..12)
        .flat_map(|i| std::fs::read(dir.join(format!("f_{i}.png"))).unwrap())
        .collect()
}

/// Run the CLI with `stdin` piped in.
fn render_stdin(args: &[&str], stdin: &[u8]) -> std::process::Output {
    use std::io::Write;
    let mut child = bin()
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut pipe = child.stdin.take().unwrap();
    let data = stdin.to_vec();
    // Write on a thread: a large stream must not block on a full pipe.
    let writer = std::thread::spawn(move || {
        let _ = pipe.write_all(&data);
    });
    let out = child.wait_with_output().unwrap();
    writer.join().unwrap();
    if !out.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&out.stderr));
    }
    out
}

/// One md5 per decoded RGBA frame, to compare two files pixel for pixel.
fn frame_hashes(file: &Path) -> Vec<String> {
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(file)
        .args(["-pix_fmt", "rgba", "-f", "framemd5", "-"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.rsplit(',').next().unwrap().trim().to_string())
        .collect()
}

#[test]
fn stdin_stream_matches_folder_for_every_format() {
    require_ffmpeg!();
    let f = frames();
    let dir = f.path().to_str().unwrap();
    let stream = png_stream(f.path());
    for ext in ["webm", "mp4", "gif", "mov"] {
        let from_dir = f.path().join(format!("dir.{ext}"));
        let from_pipe = f.path().join(format!("pipe.{ext}"));
        assert!(render(&["-i", dir, "-o", from_dir.to_str().unwrap()])
            .status
            .success());
        assert!(
            render_stdin(&["-i", "-", "-o", from_pipe.to_str().unwrap()], &stream)
                .status
                .success(),
            "{ext} from stdin"
        );
        assert_eq!(probe(&from_pipe), probe(&from_dir), "{ext}: stream info");
        let hashes = frame_hashes(&from_pipe);
        assert_eq!(hashes.len(), 12, "{ext}: frame count");
        assert_eq!(hashes, frame_hashes(&from_dir), "{ext}: pixels");
    }
}

#[test]
fn stdin_progress_with_and_without_frame_count() {
    require_ffmpeg!();
    let f = frames();
    let stream = png_stream(f.path());
    let events = |extra: &[&str]| -> Vec<serde_json::Value> {
        let out = f.path().join("p.webm");
        let mut args = vec!["-i", "-", "-o", out.to_str().unwrap(), "--progress", "json"];
        args.extend(extra);
        let res = render_stdin(&args, &stream);
        assert!(res.status.success());
        String::from_utf8_lossy(&res.stdout)
            .lines()
            .map(|l| serde_json::from_str(l).expect("every stdout line is JSON"))
            .collect()
    };

    // Known count: a real percentage, ending at 100%.
    let known = events(&["--frames", "12"]);
    let last = known.iter().rfind(|e| e["event"] == "progress").unwrap();
    assert_eq!(last["total_frames"], 12);
    assert_eq!(last["percent"], 100.0);
    assert_eq!(known.last().unwrap()["frames"], 12);

    // Unknown count: total 0 while running, frames counted at the end.
    let unknown = events(&[]);
    assert_eq!(unknown[0]["total_frames"], 0);
    assert_eq!(unknown.last().unwrap()["event"], "done");
    assert_eq!(unknown.last().unwrap()["frames"], 12);
}

#[test]
fn stdin_fade_out_needs_frame_count() {
    require_ffmpeg!();
    let f = frames();
    let stream = png_stream(f.path());
    let out = f.path().join("fade.webm");
    let o = out.to_str().unwrap();

    let res = render_stdin(&["-i", "-", "-o", o, "--fade-out", "0.5"], &stream);
    assert!(!res.status.success());
    assert!(String::from_utf8_lossy(&res.stderr).contains("--frames"));
    assert!(!out.exists());

    let args = [
        "-i",
        "-",
        "-o",
        o,
        "--fps",
        "12",
        "--frames",
        "12",
        "--fade-out",
        "0.5",
    ];
    assert!(render_stdin(&args, &stream).status.success());
    let vp9 = ["-c:v", "libvpx-vp9"];
    assert_eq!(alpha_at(&out, 6, &vp9).1, 255, "middle untouched");
    assert!(alpha_at(&out, 11, &vp9).1 < 64, "fading out at the end");
}

#[test]
fn stdin_errors_are_clear_and_leave_no_file() {
    require_ffmpeg!();
    let f = frames();
    let out = f.path().join("err.webm");
    let o = out.to_str().unwrap();
    let stderr = |res: std::process::Output| {
        assert!(!res.status.success());
        String::from_utf8_lossy(&res.stderr).into_owned()
    };

    assert!(stderr(render_stdin(&["-i", "-", "-o", o], b"")).contains("empty"));
    assert!(stderr(render_stdin(&["-i", "-", "-o", o], b"not a png at all")).contains("ffmpeg"));
    let dir = f.path().to_str().unwrap();
    assert!(stderr(render(&["-i", dir, "-o", o, "--frames", "12"])).contains("--input -"));
    let stream = png_stream(f.path());
    assert!(stderr(render_stdin(&["-i", "-", "-o", o, "--preview"], &stream)).contains("stream"));
    assert!(!out.exists(), "no half-written output");
}

#[test]
fn library_render_stream_from_a_reader() {
    require_ffmpeg!();
    let f = frames();
    let cfg = aether_renderer_core::config::PartialConfig {
        input: Some("-".into()),
        output: Some(f.path().join("lib.gif").to_string_lossy().into_owned()),
        ..Default::default()
    }
    .into_config()
    .unwrap();
    let reader = std::io::Cursor::new(png_stream(f.path()));
    let mut seen = Vec::new();
    let report =
        aether_renderer_core::render_stream(cfg, reader, Some(12), |p| seen.push(p)).unwrap();
    assert_eq!(report.frames_rendered, Some(12));
    assert_eq!(seen.first().unwrap().frame, 0);
    assert_eq!(seen.last().unwrap().percent, 100.0);
    assert!(seen.windows(2).all(|w| w[0].frame <= w[1].frame));
    assert_eq!(frame_hashes(&f.path().join("lib.gif")).len(), 12);
}
