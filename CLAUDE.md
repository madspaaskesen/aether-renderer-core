# aether-renderer-core

Rust library + CLI that renders PNG sequences (folder or ZIP) to WebM (VP9 + alpha),
MOV (ProRes 4444 + alpha), GIF or MP4 by driving the `ffmpeg` binary.
Published on crates.io; the Tauri GUI (`../aether-renderer-gui`) depends on it.

## Commands

```bash
cargo build
cargo test --all-targets            # needs ffmpeg + ffprobe on PATH; ffmpeg tests skip without them
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -- -i frames/ -o out.webm -v               # CLI
cargo run -- -i frames/ -o out.mp4 --background '#1e1e1e' --progress json
```

CI (`.github/workflows/ci.yml`) runs fmt + clippy, then tests on Ubuntu, macOS and Windows.

## Layout

| File | Role |
|---|---|
| `src/main.rs` | CLI (clap). `Args::overrides()` → `PartialConfig`, merged over a `--config` file. `--progress json` prints JSON lines via `emit()` |
| `src/config.rs` | `RenderConfig` (resolved), `PartialConfig` (all optional, `or()` merge, `into_config()` defaults + validation), `FORMATS`, `parse_background()` |
| `src/lib.rs` | `render()` / `render_with_progress()`: validate → unzip → collect frames → numbered sequence → fade + background → ffmpeg. Also `preview_frame()` |
| `src/ffmpeg/video.rs` | webm / mp4 / mov encoder args and `-vf` chain |
| `src/ffmpeg/gif.rs` | one-pass `split → palettegen → paletteuse` GIF |
| `src/utils.rs` | `run_ffmpeg_with_progress()` (spawns ffmpeg with `-progress pipe:1`, parses `frame=`, reads stderr on a thread), `background_filter()`, unzip helpers |
| `src/input.rs` | frame globbing, natural sort, `FrameSequence` (numbered links so ffmpeg works on every OS), `png_has_alpha()` |
| `src/report.rs` | `RenderReport`, `RenderProgress` |
| `tests/test_formats.rs` | end-to-end tests: generate odd-sized (321×241) transparent frames with ffmpeg, render, probe pixels |

## Rules that are easy to break

- **Adding a config option** touches: `RenderConfig`, `PartialConfig` (field + `or()` + `into_config()`),
  `Args` + `overrides()` in `main.rs`, the options table in `README.md`, and every
  `RenderConfig { .. }` literal in `tests/` (plus the GUI's `run_renderer`). Adding a field to
  `RenderConfig` is a breaking change for library users → minor version bump + CHANGELOG note.
- **Adding a format**: `config::FORMATS`, the clap `value_parser` in `main.rs`, the match in
  `ffmpeg/video.rs`, the extension check in `lib.rs`, README format table.
- **Filter order**: fades always use `alpha=1`, then `utils::background_filter()` paints the
  background (so fades go to transparent or to the background color). mp4 always gets a
  background (black by default) and `pad=ceil(iw/2)*2:ceil(ih/2)*2:color=<bg>` for even sizes.
  With a background, use non-alpha pix_fmts (`yuv420p`, `yuv444p10le`).
- `background_filter()` uses `drawbox ... t=fill:replace=1` (needs FFmpeg ≥ 4.4) and labels
  prefixed `ar_` so it can sit inside the GIF's `-filter_complex`.
- Never pass a glob to ffmpeg on the main path: Windows ffmpeg builds lack `-pattern_type glob`.
  `render()` always hands ffmpeg a numbered sequence from `FrameSequence`.
- On ffmpeg failure, remove the half-written output (`run_ffmpeg_with_progress` does) and include
  ffmpeg's last stderr lines in the error.
- Tests must handle Windows: build config JSON with `serde_json`, strip `\r` from ffprobe output.
- Every bug fix gets a test in `tests/test_formats.rs` that renders real frames.

## Releasing

1. Bump `version` in `Cargo.toml`, add a `## x.y.z` section to `CHANGELOG.md`, update the
   README (version in the library snippet, roadmap).
2. PR → merge to `main` → tag `vX.Y.Z`; `release.yml` builds the binaries.
3. `cargo publish` from `main` by hand. Then update the GUI's dependency.
