# Tapeit

Ultra-lightweight screen recorder built with **Tauri 2 + Rust + SolidJS**.

Cross-platform: Windows, macOS, Linux.

## Prerequisites

### All Platforms
- [Rust](https://rustup.rs/) (1.77+)
- [Node.js](https://nodejs.org/) (18+)
- [FFmpeg](https://ffmpeg.org/) (must be in PATH)

### Windows
- Visual Studio C++ Build Tools
- WebView2 (pre-installed on Windows 10/11)

```bash
# Install via winget
winget install Rustlang.Rustup
winget install Gyan.FFmpeg
```

### macOS
```bash
brew install rust ffmpeg
```

### Linux (Ubuntu/Debian)
```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget \
  libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
  ffmpeg pipewire libpipewire-0.3-dev
```

## Setup

```bash
# Install Node dependencies
npm install

# Install Tauri CLI
cargo install tauri-cli@^2

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

## Project Structure

```
tapeit/
├── src/                      # SolidJS frontend
│   ├── components/
│   │   └── recorder/         # Recording UI components
│   ├── stores/               # State management
│   ├── styles/               # CSS
│   └── utils/
├── src-tauri/                # Rust backend (Tauri 2)
│   └── src/
│       ├── capture/          # Screen capture engine (scap)
│       ├── audio/            # Audio capture (cpal)
│       ├── encoder/          # Video encoding (FFmpeg)
│       ├── commands/         # Tauri IPC commands
│       └── utils/
├── package.json
└── vite.config.ts
```

## Tech Stack

| Layer | Technology | Why |
|-------|-----------|-----|
| Desktop Shell | Tauri 2 | ~5MB bundle, native WebView, Rust backend |
| Recording Engine | Rust + scap | Cross-platform screen capture via native APIs |
| Audio | cpal | Cross-platform audio capture |
| Encoding | FFmpeg | Hardware-accelerated H.264 encoding |
| Frontend | SolidJS | Tiny bundle, fine-grained reactivity |

## Features

- Screen capture (displays + windows) via native APIs
- Mic + system audio capture with AAC encoding
- Adjustable FPS (15 / 24 / 30 / 60)
- Pause / resume recording
- Floating overlay window (timer + controls, draggable, always-on-top)
- Auto minimize/restore main window during recording
- MP4 output with timestamp naming (`tapeit_YYYYMMDD_HHMMSS.mp4`)

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+R` | Toggle recording (not yet wired) |
| `Ctrl+Shift+S` | Take screenshot (not yet implemented) |

## Roadmap

- [x] Phase 1: Screen capture + local save + audio
- [ ] Phase 2: Webcam overlay + annotations
- [ ] Phase 3: Cloud upload + shareable links
- [ ] Phase 4: Team workspaces + comments
- [ ] Phase 5: Auto-captions (Whisper)
