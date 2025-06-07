# 🌌 Aether Renderer Core

**Aether Renderer Core** is a lightweight, Rust-based CLI tool that converts image sequences (PNG/WebP) into transparent `.webm` or `.mp4` videos using `ffmpeg`.

Built with love for artists, developers, and sacred animation workflows.

![Rust](https://img.shields.io/badge/built_with-rust-orange)
![FFmpeg](https://img.shields.io/badge/rendered_by-ffmpeg-blue)

---

## ✨ Features

- ✅ Supports alpha channel export (via `yuva420p`)
- ✅ Input PNG/WebP sequences with alpha from folder or .zip file
- ✅ Export `.webm` with alpha (via `libvpx`)
- ✅ Export `.gif` with alpha
- ✅ `.mp4` fallback (no alpha)
 - ✅ CLI flags for FPS, input folder, output path, format
 - ✅ Optional `--fade-in` and `--fade-out` for smooth loops

---

## 🧪 Usage

```bash
cargo run --release -- \
  --input ./frames \
  --output my.webm \
  --fps 30 \
  --format webm \
  --fade-in 1 \
  --fade-out 1
```

The `--fade-in` and `--fade-out` flags apply ffmpeg's [`fade`](https://ffmpeg.org/ffmpeg-filters.html#fade) filter under the hood. The start of the fade out is automatically calculated from the frame count and FPS.

📂 Your input folder should contain files like:

```
frame_0000.png
frame_0001.png
frame_0002.png
...
```

---

You can now also pass a .zip file containing frames:

```bash
cargo run -- --input ./my-frames.zip --output my.webm --fps 30 --format webm
```


📂 Your input folder or ZIP file must contain images named like:

```
frame_0000.png
frame_0001.png
frame_0002.png
...
```

Alpha-enabled PNGs are recommended for transparent .webm.

---

Convert to gif file with transparent background:

```bash
cargo run -- --input ./my-frames.zip --output my.gif --fps 30 --format gif
```

(Just make sure ffmpeg is installed)

---

## 🧰 Requirements

- Rust & Cargo installed: https://rustup.rs
- `ffmpeg` must be installed and accessible in your system path

---

## 🧪 Tests

This project contains both unit tests and integration tests. The unit tests live
next to the code they verify (for example in
`src/utils/unzip_frames.rs`) while the integration tests reside in
`tests/integration.rs`.

Run all tests using Cargo:

```bash
cargo test
```

The integration suite relies on `ffmpeg` being available on your system. If
`ffmpeg` is missing, the rendering test is skipped but all other tests still
run.

---

## 📦 Example ZIP

You can test the renderer using the provided frame sequence:

[sacred-stars.zip](https://ojkwbrxgljlgelqndiai.supabase.co/storage/v1/object/public/sacred-ai/web/aether-renderer/sacred-stars.zip)

Run it like this:

```bash
cargo run -- --input examples/sacred-stars.zip --output demo.webm --fps 30 --format webm
```

This will generate a loopable .webm video with alpha.

---

## ✨ Example Output

See full demo here (just started):
[Webpage demo](https://sacred-ai.com/about/aetherrenderer)

Here’s one frame from the sacred animation:

<img src="https://ojkwbrxgljlgelqndiai.supabase.co/storage/v1/object/public/sacred-ai/web/aether-renderer/sacred-stars-preview.png" width="480" alt="Frame example">

---

## ✨ Example Animation

![demo](https://ojkwbrxgljlgelqndiai.supabase.co/storage/v1/object/public/sacred-ai/web/aether-renderer/sacred-stars-preview.gif?v=1)

▶️ [Watch output video](https://ojkwbrxgljlgelqndiai.supabase.co/storage/v1/object/public/sacred-ai/web/aether-renderer/sacred-stars-preview.webm)

---

## 🔮 Roadmap

- [x] Render `.png` → `.webm` (with alpha)
- [ ] Support `.mp4` export
- [ ] Add bitrate / CRF quality control
- [x] `--fade-in`, `--fade-out` for soft loops
- [ ] Handle errors & missing frames gracefully
- [ ] Add optional CLI preview
- [ ] Begin GUI version with Tauri (`aether-renderer`) 🌟

---

## 🌿 License

MIT — created with sacred care by [@madspaaskesen](https://github.com/madspaaskesen)

---

## 🌐 Related Projects

- 🕊️ [Sacred-AI](https://sacred-ai.com)
- 📈 [MySiteChart](https://mysitechart.com)
- 🛠️ [MP-IT](https://mp-it.dk)
- 🧵 [DDD Favoritter](https://ddd-favoritter.dk)

---

## 💛 Made with love by Sacred-AI

🙏 Made with clarity and care by [@mads](https://github.com/madspaaskesen) @ [@sacred-ai](https://github.com/Sacred-AI) 💛

🌸 Powered by [Rust Lang](https://www.rust-lang.org/), [Rust getting started](https://www.rust-lang.org/learn/get-started)

Aether Renderer Core is the sacred heart of a lightweight animation rendering toolkit.
Converts frame sequences to video with love, transparency, and full creative control.
