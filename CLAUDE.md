# CLAUDE.md - Tapeit

## Project Overview

**Tapeit** is an ultra-lightweight, cross-platform screen recorder built with **Tauri 2 + Rust + SolidJS**. It targets Windows, macOS, and Linux with a ~5MB bundle size using native WebView instead of Electron.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────┐
│       SolidJS Frontend (TypeScript)         │
│                                             │
│  ┌──────────────┐   ┌──────────────────┐    │
│  │  Main Window  │   │  Overlay Window  │    │
│  │  (380x520)    │   │  (280x72)        │    │
│  │               │   │                  │    │
│  │ RecorderPanel │   │ OverlayPanel     │    │
│  │  SourcePicker │   │  Timer + Dot     │    │
│  │  AudioCtrls   │   │  Pause/Stop Btns │    │
│  │  FPS Select   │   │  Drag anywhere   │    │
│  │  Record Btn   │   │                  │    │
│  └──────┬───────┘   └────────┬─────────┘    │
│         │                    │               │
│  useRecorderStore()          │ invoke()      │
│  (Solid signals)             │               │
└─────────┬────────────────────┘───────────────┘
          │ Tauri IPC (invoke)
          ▼
┌─────────────────────────────────────────────┐
│          Rust Backend (Tauri 2)              │
│  commands/mod.rs ← IPC command handlers     │
│  main.rs ← Window management commands       │
│  ┌────────┐  ┌─────────┐  ┌──────────┐     │
│  │Capture │  │ Encoder │  │  Audio   │     │
│  │ (scap) │  │ (FFmpeg)│  │  (cpal)  │     │
│  └────────┘  └─────────┘  └──────────┘     │
└─────────────────────────────────────────────┘
```

### Window Management Flow

```
[User clicks Record]
    │
    ├──> start_recording() ──> Rust spawns capture thread
    ├──> show_overlay()    ──> Creates floating overlay window
    └──> minimize_main()   ──> Minimizes main config window

[User clicks Stop on overlay]
    │
    ├──> stop_recording()  ──> Stops capture, encodes MP4
    ├──> hide_overlay()    ──> Closes overlay window
    └──> restore_main()    ──> Restores main window to foreground
```

### Overlay Window Properties
- **Size**: 280x72px, pill-shaped (border-radius: 40px)
- **Always on top**: Floats above all other windows
- **Transparent + undecorated**: No title bar, transparent background
- **Draggable**: Entire overlay is a drag handle (via Tauri `startDragging`)
- **Skip taskbar**: Does not appear in taskbar/dock
- **Not captured**: Transparent window excluded from screen capture
- **Created dynamically**: Built via `WebviewWindowBuilder` at runtime, not in static config

### Data Flow

1. **Source Enumeration**: Frontend calls `get_capture_sources()` on mount -> backend uses `scap` to list displays/windows
2. **Start Recording**: `start_recording(config)` spawns a background thread with `scap::Capturer` capturing raw BGRA frames
3. **Frame Loop**: Frames captured at configured FPS, stored in-memory, pause/resume via atomic flag
4. **Encoding**: On stop, frames piped to FFmpeg CLI process (BGRA -> YUV420P -> H.264 MP4)
5. **Output**: MP4 saved with timestamp-based filename to configured output directory

### Concurrency Model

- Frame capture runs in a **spawned thread** (not async) to avoid blocking UI
- **Tokio** async runtime for I/O operations
- **Atomic flag** for pause/resume without lock contention

## Tech Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Desktop Shell | Tauri 2 | Native WebView, Rust backend, ~5MB bundle |
| Frontend | SolidJS 1.9 + TypeScript | Fine-grained reactive UI |
| Build Tool | Vite 6 | Fast HMR, multi-page builds (main + overlay) |
| Screen Capture | scap 0.0.8 | Cross-platform native capture APIs |
| Audio Capture | cpal 0.15 | Cross-platform audio (WASAPI/CoreAudio/PulseAudio) |
| Video Encoding | FFmpeg (CLI) | H.264 encoding (libx264 for now) |

### Platform-Specific APIs

- **Windows**: Windows Graphics Capture API, WASAPI, Direct3D11/DXGI
- **macOS**: ScreenCaptureKit, Core Audio, Objective-C interop via `objc2`
- **Linux**: PipeWire/X11, PulseAudio, GTK3

## Project Structure

```
src/                          # SolidJS frontend
├── App.tsx                   # Root component (main window)
├── index.tsx                 # Main window entry point
├── overlay.tsx               # Overlay window entry point
├── components/
│   ├── recorder/             # Main window components
│   │   ├── RecorderPanel.tsx # Config UI (source, audio, fps, record btn)
│   │   ├── SourcePicker.tsx  # Display/window dropdown
│   │   ├── AudioControls.tsx # Mic + system audio toggles
│   │   └── Timer.tsx         # Duration display
│   └── overlay/              # Overlay window components
│       └── OverlayPanel.tsx  # Floating recording controls (timer + stop/pause)
├── stores/recorder.ts        # Global state (Solid signals)
├── styles/
│   ├── global.css            # Design tokens + main window styles
│   ├── recorder.css          # Recorder component styles
│   └── overlay.css           # Overlay window styles
└── utils/format.ts           # Utility functions

src-tauri/                    # Rust backend
├── src/
│   ├── main.rs               # App init, window management commands (show/hide overlay, minimize/restore main)
│   ├── commands/mod.rs        # Recording IPC command handlers
│   ├── capture/
│   │   ├── recorder.rs        # ScreenRecorder (capture loop, state machine)
│   │   └── sources.rs         # CaptureSource enumeration
│   ├── encoder/video.rs       # VideoEncoder (FFmpeg CLI wrapper)
│   ├── audio/capture.rs       # AudioCapturer (mic + system audio)
│   └── utils/mod.rs           # Platform-specific path utilities
├── Cargo.toml                 # Rust dependencies
└── tauri.conf.json            # App config (main window, shortcuts)

index.html                    # Main window HTML entry
overlay.html                  # Overlay window HTML entry
vite.config.ts                # Multi-page Vite config (main + overlay)
```

## Development

### Prerequisites

- Rust 1.77+, Node.js 18+, FFmpeg in PATH
- Windows: Visual Studio C++ Build Tools + WebView2
- macOS: Xcode Command Line Tools
- Linux: libwebkit2gtk, GTK3, PipeWire

### Commands

```bash
npm install              # Install frontend dependencies
cargo install tauri-cli@^2  # Install Tauri CLI
cargo tauri dev          # Run in dev mode (frontend on port 1420)
cargo tauri build        # Production build
```

### Dev Server

- Vite dev server runs on **port 1420** (required by Tauri)
- Multi-page build: `index.html` (main) + `overlay.html` (overlay)
- Hot module replacement enabled for frontend changes

## IPC Commands (Frontend <-> Backend)

| Command | Location | Purpose |
|---------|----------|---------|
| `get_capture_sources()` | commands/mod.rs | List available displays and windows |
| `start_recording(config)` | commands/mod.rs | Begin capture with source, fps, output dir |
| `stop_recording()` | commands/mod.rs | Stop capture and encode to MP4 |
| `pause_recording()` | commands/mod.rs | Pause frame capture |
| `resume_recording()` | commands/mod.rs | Resume frame capture |
| `get_recording_state()` | commands/mod.rs | Poll current recorder state |
| `show_overlay()` | main.rs | Create and show floating overlay window |
| `hide_overlay()` | main.rs | Close overlay window |
| `minimize_main()` | main.rs | Minimize main config window |
| `restore_main()` | main.rs | Restore and focus main window |

## Global Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+R` | Toggle recording start/stop |
| `Ctrl+Shift+S` | Take screenshot (not yet implemented) |

## App Configuration

- **App ID**: `com.tapeit.app`
- **Main Window**: 380x520, resizable, decorated, centered
- **Overlay Window**: 280x72, always-on-top, transparent, undecorated, draggable (created dynamically)
- **Output format**: MP4 (H.264)
- **FPS options**: 15, 24, 30, 60

## Current State & Known Limitations

### Phase 1 - In Progress
- [x] Screen capture from displays and windows
- [x] Local file saving with timestamp naming (`tapeit_YYYYMMDD_HHMMSS.mp4`)
- [x] Global keyboard shortcuts
- [x] Adjustable FPS, pause/resume
- [x] Floating overlay window (timer + controls, draggable)
- [x] Main window minimize/restore on record start/stop
- [ ] Audio capture integration (AudioCapturer exists but not wired into pipeline)
- [ ] Audio muxing into MP4 output

### MVP Limitations
- Encoding uses **FFmpeg CLI wrapper** (not zero-copy FFI bindings)
- No GPU-accelerated encoding yet (software libx264 only)
- Audio capture exists but is **not muxed** into final MP4
- Screenshot shortcut registered but not implemented

## Roadmap

- [ ] Phase 1: Screen capture + local save + audio (in progress)
- [ ] Phase 2: Webcam overlay + annotations
- [ ] Phase 3: Cloud upload + shareable links
- [ ] Phase 4: Team workspaces + comments
- [ ] Phase 5: Auto-captions (Whisper)

## Design Decisions

1. **Tauri over Electron**: ~5MB vs ~150MB bundle, native performance, Rust backend
2. **SolidJS over React**: Smaller bundle, true fine-grained reactivity, no virtual DOM
3. **No system tray**: Replaced with floating overlay window for cleaner UX during recording
4. **Dynamic overlay window**: Created via `WebviewWindowBuilder` at runtime, not in static config — only exists while recording
5. **scap for capture**: Native API abstraction (WGC/SCK/PipeWire) without manual FFI
6. **FFmpeg CLI wrapper for MVP**: Ship fast, upgrade to zero-copy bindings later
7. **Spawned thread for capture**: Dedicated thread avoids async executor overhead for tight frame loops
