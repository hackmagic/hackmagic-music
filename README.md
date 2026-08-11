# HackMagic Music Player

Cross-platform music player with native GPUI, five audio engines, AI-powered similarity search, and full i18n.

## Features

- **5 Audio Engines** — BASS, rodio, FFmpeg (subprocess), MCI (Windows), Symphonia (pure Rust)
- **Native GUI** — Built with [GPUI](https://www.gpui.rs/), lightweight and responsive
- **AI Similarity Search** — ONNX audio embedding + LanceDB vector search
- **Lyrics** — LRC/KSC/VTT parsing, karaoke mode, desktop lyrics, auto-download
- **Equalizer** — 10-band graphic EQ with presets
- **Media Library** — Scan, cache, search, favourites, play stats
- **Last.fm Scrobbling** — Track scrobbling and now-playing updates
- **Global Hotkeys** — Keyboard media keys, system-wide (Windows)
- **SMTC Integration** — System Media Transport Controls (Windows)
- **i18n** — English and Chinese, switchable at runtime
- **Tag Editing** — Read/write metadata, album art, embedded lyrics
- **Playlist Management** — M3U8/TTPL/WPL import/export, cue sheets, multi-version merge
- **CLI & Daemon** — Full command-line interface, background daemon with HTTP API

## Quick Start

```bash
cargo run -- help
cargo run -- play path/to/music.flac
cargo run --bin hm-gui    # launch GUI (or use start.bat)
```

## Build

```bash
# Dependencies (Windows): BASS DLLs
.\download-bass.ps1

# Build and test
cargo build
cargo test --lib
```

## Global Hotkeys (Windows)

| Function | Shortcut |
|---|---|
| Play / Pause | `Space` |
| Stop | `Ctrl+S` |
| Forward | `Ctrl+Shift+Right` |
| Rewind | `Ctrl+Shift+Left` |
| Previous | `Ctrl+Left` |
| Next | `Ctrl+Right` |
| Volume Up / Down | `Ctrl+Up` / `Ctrl+Down` |
| Show / Hide Player | `Ctrl+H` |
| Show / Hide Desktop Lyrics | `Ctrl+L` |
| Add to Favorites | `Ctrl+D` |
| Exit | `Alt+F4` |

Global hotkeys can be disabled in **Settings → Hotkeys** (they register system-wide on Windows).

## CLI

```bash
cargo run -- help          # list all subcommands
cargo run -- play path/to/song.flac
cargo run -- search-lyrics "关键词" --limit 5
cargo run -- index path/to/library   # scan + build AI vector index
cargo run -- daemon start            # background daemon + HTTP API
```

See [开发计划.md](开发计划.md) for project roadmap (Chinese).
