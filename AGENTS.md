# AGENTS.md — HackMagic Music Player

Rust music player: CLI + native GPUI GUI, 5 audio engines, AI similarity search. Project docs (`开发计划.md`, `监控进度.md`, `docs/缺陷分析与改进计划.md`) and many source comments are in Chinese — preserve them. Coding conventions also live in `.atomcode/skills/project-conventions/` (Chinese).

## Build / test / run

- Build: `cargo build`. Two binaries from one crate (`hm`, edition 2021): `hm` (CLI, `src/main.rs`) and `hm-gui` (GUI, `src/bin/gui.rs`); shared lib `hm` in `src/lib.rs`.
- Test: `cargo test --lib`; `tests/ai_pipeline.rs` (full `cargo test`) is hermetic — generates temp WAVs and a temp vector DB, works without the ONNX model.
- No custom clippy/rustfmt config; plain `cargo clippy` / `cargo fmt`.
- GUI: `cargo run --bin hm-gui` — requires a Windows display; cannot run headless.
- Windows prereq: run `.\download-bass.ps1` once. `build.rs` copies `bass*.dll` (project root, fallback `target/release/`) into the output dir at build time.

## Architecture

- `core/` — `Player`, playlist, `EngineType` / `AudioEngine` trait (`audio_common.rs`). 5 engines: BASS (`src/bass/`, libloading), FFmpeg subprocess (`ffmpeg_engine.rs`), MCI, rodio, symphonia. `EngineType::from_str` is NOT `std::str::FromStr` — unknown strings silently default to Bass, never error.
- `commands/` — CLI subcommands (audio, effects, media, playback, playlist, system, track).
- `gui/` — GPUI 0.2.2 + gpui-component 0.5.1 (pinned — do NOT bump; API is unstable). i18n via `gui/i18n.rs` (EN/CN runtime switch): route all UI strings through it, never hardcode.
- `config.rs` — TOML config (`toml` crate). `vecdb/` and `models/` are generated and gitignored (LanceDB + ONNX embedding).
- Windows-only modules (`smtc`, `tray`, `hotkey`, `mci_engine`) are `cfg(target_os = "windows")` — keep the gates when editing.

## Conventions

- No `unwrap()`/`expect()` in production code; no `println!`/`eprintln!` debug output — use `tracing` (GUI writes `hm_gui.log` in CWD; panics go to `hm_panic.log`).
- Errors: `thiserror` enums via `src/error.rs`; no panics in library code; no blocking I/O in async contexts (shared Tokio runtime: `hm::runtime()`).

## Gotchas

- GPUI pitfalls — scrollable `Div` needs `gpui_component::scroll::ScrollableElement` (no `overflow_y_scroll`), `cx.listener` closures take 4 args `(this, event, window, cx)`, `Root::new` needs `Context<Root>` not `&mut App`, `Button::new` ids must be tuples like `("id", i)`, `SetWindowPos` 2nd arg is `Option<HWND>`, no `border_right_1`. Read `.workbuddy/skills/gpui-gotchas/SKILL.md` before touching GUI code.
- GUI must run on an 8 MB-stack thread (`src/bin/gui.rs`) — GPUI render trees overflow the default 2 MB main-thread stack. Preserve that pattern.
- `start.bat` is broken (references a missing `gui/` dir) — don't use it. `src/gui/mod.rs.bak` is a stale backup.
- `main.rs` enforces a single instance on Windows via a named mutex (`InstanceLock`).
- Release profile sets `lto` + `codegen-units = 1` → release builds are slow; iterate in debug.
