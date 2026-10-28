# Changelog

## 0.7.0

### Added
- **`--input -`: frames from a PNG stream on stdin** (PNG files back to back), e.g.
  `aether-renderer-capture --stdout | aether-renderer-core -i - -o out.webm`. Every format
  works; the bytes go straight to ffmpeg, so there are no temp files. Output is pixel for
  pixel the same as rendering the frames from a folder.
- **`--frames N`** for a stdin stream: optional, gives a real progress percentage and is
  needed for `--fade-out`. Without it, progress reports `total_frames: 0` (unknown).
- **`render_stream(config, reader, expected_frames, on_progress)`** for library users:
  render from any `Read`, e.g. a child process's stdout.
- `utils::run_ffmpeg_with_input` and `input::png_bytes_have_alpha`.
- Clear errors for an empty stream, `--fade-out` without `--frames`, `--frames` without
  `-i -`, and `--preview` on a stream. A failed stream render leaves no half-written file.

No breaking changes: `RenderConfig` and the existing functions are unchanged.

## 0.6.0

### Added
- **`--background <COLOR>`** (`background` in config files): paint the frames onto a solid
  color, e.g. `'#1e1e1e'`, `'#fff'` or `white`. Works for every format and makes the output
  opaque; fades go to that color, and mp4's padding pixel gets it too.
- **Real progress.** `--verbose` shows a progress bar with frames done instead of a spinner.
- **`--progress json`**: one JSON object per line on stdout (`progress`, then `done` or
  `error`), so GUIs and scripts can show their own progress.
- **`render_with_progress(config, |p| ...)`** for library users, with `RenderProgress`
  (`frame`, `total_frames`, `percent`).
- `--verbose-ffmpeg` now prints ffmpeg's log as it runs.

### Changed
- **MP4 flattens transparency onto black explicitly** (or onto `--background`). Before,
  transparent pixels kept whatever color hid under them, which could look different per
  exporter. When the frames have transparency, the report says so and points to `--background`.
- Breaking for library users: `RenderConfig` has a new `background` field, and
  `ffmpeg::video::render_video` / `ffmpeg::gif::render_gif` take `background` and an
  `on_frame` callback.

## 0.5.0

A release about trust: everything the README promised now works, and every bug below
has a test that renders real transparent frames.

### Added
- `--format mov`: **ProRes 4444 with alpha**, for Premiere, DaVinci Resolve, Final Cut and After Effects.
- `--fade-in`, `--fade-out`, `--bitrate` and `--crf` on the command line (before, only in config files).
- **TOML config files** (`--config render.toml`), next to JSON.
- **Config + command line**: options on the command line now override the config file.
- The format follows the output file name (`-o loop.gif` renders a GIF); `--format` still wins.
- `--version`, short flags `-i -o -c -v`, examples in `--help`.
- `config::PartialConfig` for library users who want the same defaults and merging as the CLI.

### Changed
- **WebM uses VP9** instead of VP8, with constant quality `crf 30` by default: about 40% smaller files and better quality (VP8 ran at ~100 kbit/s).
- **Fades on transparent formats fade the alpha channel**, so animations fade to transparent instead of black. MP4 still fades from/to black.
- Frames are sorted in **natural order** (`f_2.png` before `f_10.png`).
- MP4 gets `+faststart`, so it starts playing before it has fully downloaded.
- When ffmpeg fails, the error now includes ffmpeg's own explanation.
- `zip` 0.6 → 2.x.

### Fixed
- **MP4 failed for frames with odd width or height** (common for canvas/AI exports) and left an empty file. Frames are now padded by at most one pixel.
- **GIF ignored `--fps`** (always 30) and was always **scaled to 640 px** wide. It now keeps the frames' size and the requested fps.
- **Windows**: the default `*.png` relied on ffmpeg's glob input, which Windows builds of ffmpeg lack. Frames are now handed to ffmpeg as a numbered sequence on every platform.
- `--file-pattern 'frame_%04d.png'` found no frames.
- `--preview` without a number did a full render instead of saving the middle frame.
- `"preview": false` in a config file (as in the shipped example) failed to parse.
- GIF rendering wrote `palette.png` into the current folder; it now uses a single ffmpeg pass.
- Failed renders no longer leave a broken output file behind.
- ZIPs made on Windows (with `\` in paths) are extracted correctly.
- Input folders with `[`, `]` or `*` in their name.

### Project
- CI on Linux, macOS and Windows, with clippy; release binaries now include **macOS Apple Silicon**.
- 40 tests (was 22).
- Homepage moved to [mp-it.dk/projects/aether-renderer](https://mp-it.dk/projects/aether-renderer); README images no longer depend on Supabase.

## 0.4.0 and earlier

See the [GitHub releases](https://github.com/madspaaskesen/aether-renderer-core/releases).
