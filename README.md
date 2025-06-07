# 🌸 aether-renderer-core

Convert PNG or WebP frame sequences into transparent `.webm` or `.mp4` videos using Rust + ffmpeg.

---

## ✨ Features

- Supports alpha channel export (via `yuva420p`)
- CLI-based control over:
  - Frame rate
  - Format (webm/mp4)
  - Output path

---

## 🧱 Getting started

Install rust & cargo:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Read more 👉 [Rust getting started](https://www.rust-lang.org/learn/get-started)

## 🦄 Usage

```bash
cargo run --release -- \\
  --input ./frames \\
  --output sacred-animation.webm \\
  --fps 30 \\
  --format webm
```

📂 Input folder should contain frames like:
frame_0000.png, frame_0001.png, ...

---

## 🔮 Roadmap

- Add fade-in/fade-out control
- Add audio syncing
- Add Tauri-based GUI (coming soon: aether-renderer)

Licensed under MIT — by Siria

---

### 🧪 Next Step: Try It!

```bash
cargo run -- --input ./frames --output my.webm --fps 30 --format webm
```

(Just make sure ffmpeg is installed)

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

