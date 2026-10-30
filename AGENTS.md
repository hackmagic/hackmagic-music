# AGENTS.md — HackMagic Music Player

Rust music player: CLI + native GPUI GUI, 5 audio engines, AI similarity search. Project docs (`开发计划.md`, `监控进度.md`, `docs/缺陷分析与改进计划.md`) and many source comments are in Chinese — preserve them. Coding conventions also live in `.atomcode/skills/project-conventions/` (Chinese).

## 验收（强制 — 先读这条）

**"写完" ≠ "完成"。** 任何功能 / 修复 / 重构在报告完成之前必须：

1. 跑 `./verify.sh --full`（G0 闸门：clippy 0/0、单元测试全绿）；涉及交付再跑 `./verify.sh --size`。
2. 按 `docs/验收标准.md` §4 对**本次改动涉及的模块**做实际功能验证，留下证据（命令输出 / 日志 / 截图）。
3. 把结论追加到 `docs/验收标准.md` §7 验收记录，并追加 `监控进度.md` 章节。

只做了静态验证（编译 / clippy / 单测）的功能，状态只能记 **⚠️ 部分**，不得记 ✅。
闸门未通过就**不要提交**。标准、体积基线、已知例外全部以 `docs/验收标准.md` 为准。

## 仓库管理（git / gh）

- **两个远端，必须同时推、保持锁步**：`origin` → `github.com/hackmagic/hackmagic-music`
  （主）、`atomgit` → `atomgit.com/qq_27401047/hackmagic-music`（镜像）。
  只推 `origin` 会让镜像长期分叉 —— 推完两个都要 `git rev-list --left-right --count <remote>/main...HEAD`
  确认是 `0 0`。
- **工作流是直接提交到 `main`，不开 PR**：仓库历史上从没有用过 PR（`gh pr list` 为空），
  提交信息写清「修了什么旧行为 / 为什么旧行为是错的」（见 `监控进度.md` §15.3、§21.3）。
- **按主题拆提交**，一个提交只说一件事；每个提交单独 checkout 都必须可编译
  （靠 git 的 tree 快照实现：未暂存的文件在提交树里保持原样）。
- `gh` CLI 已装并以 `hackmagic` 账号登录（ssh 协议）。仓库管理用 `gh`：
  `gh issue create --body-file`、`gh release create --notes-file`、`gh repo view --json`。
- **发布资产命名有硬约束**（程序自己的自动更新读它）：`src/commands/system.rs` 的
  `find_asset_url` 要求资产名**含目标三元组**且以 `.zip` 结尾；`replace_exe_windows`
  要求 zip 内是**扁平布局**——根目录 `hm.exe` + `bass*.dll`，GUI 放 `gui/hm-gui.exe`。
  命名示例：`hm-v1.0.0-x86_64-pc-windows-msvc.zip`。
- 未跟踪的 `.h3-video-task.json` / `h3-prompt.txt` 属其他工具，**不要入库**。

## Build / test / run

- Build: `cargo build`. Four binaries from one crate `hm` (edition 2021): `hm` (CLI, `src/main.rs`), `hm-gui` (GUI, `src/bin/gui.rs`), plus two test tools `hm-e2e-test` / `hm-screenshot` (`src/bin/`); shared lib `hm` in `src/lib.rs`. The CLI binary does not link gpui (linker drops it).
- Test: `cargo test --lib`; `tests/ai_pipeline.rs` (full `cargo test`) is hermetic — generates temp WAVs and a temp vector DB, works without the ONNX model.
- No custom clippy/rustfmt config; plain `cargo clippy` / `cargo fmt`.
- GUI: `cargo run --bin hm-gui` — requires a Windows display; cannot run headless.
- Windows prereq: run `.\download-bass.ps1` once. `build.rs` copies `bass*.dll` (project root, fallback `target/release/`) into the output dir at build time.

## Architecture

- `core/` — `Player`, playlist, `EngineType` / `AudioEngine` trait (`audio_common.rs`). 5 engines: BASS (`src/bass/`, libloading), FFmpeg subprocess (`ffmpeg_engine.rs`), MCI, rodio, symphonia. Parse engine names with `EngineType::try_from_str` → `Option` (`None` for unknown); there is no `from_str` on `EngineType` (the one on `SourceId` also returns `Option`).
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
- Release profile is size-tuned (`opt-level="s"`, `lto="fat"`, `codegen-units=1`, `panic="abort"`, `strip=true`) → a full release build is slow (~15 min). Iterate in debug. `panic="abort"` is intentional: no `catch_unwind` in the codebase, so unwind tables are dead weight. Do not add `catch_unwind` without revisiting this.
