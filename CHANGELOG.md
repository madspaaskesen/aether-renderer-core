# Changelog

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
