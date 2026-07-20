# Phase 2 进展：补全"空壳"菜单与对话框逻辑（2026-07-20）

上一轮（Phase 0/1）修了重绘冻结、均衡器、专辑封面、深色重绘、死代码。
本轮针对用户"逻辑不通 / 好多界面都不同"的反馈，把原版 7 大菜单里仍是
`log!` 空壳的项全部接成真实行为，并新增模态对话框系统。

## 本次改动（src/gui/mod.rs）

### 1. 歌词（Lyric 菜单）— 全部接实
- **偏移 ±0.5s**：`lyric_offset_ms` 字段 + `poll_player_state` 实时应用偏移，菜单项 `weak.update` 累加 ±500ms。
- **显示翻译**：`lyric_show_translation` 字段，重载当前曲目时写入 `TranslateMode::Separate/Hidden`。
- **复制当前行 / 复制全部**：用平台命令（`clip.exe`/`pbcopy`/`xclip`）实现零依赖剪贴板 `copy_text_to_clipboard`。
- **重新加载歌词**：真正回调 `load_lyrics_for_track`。
- **桌面歌词**：`desktop_lyrics_open` 开关 → 全窗口歌词视图（gpui 0.2.2 无 `.absolute()`，用全屏接管实现）。

### 2. 模态对话框系统（新增 ModalKind）
- `About`：复用 `dialogs::render_about_dialog`，全窗口居中卡片 + 关闭按钮。
- `SongInfo`（歌曲信息）：从当前曲目聚合 标题/艺术家/专辑/时长/类型/比特率/采样率/声道/收藏/路径。
- `FormatConvert`（格式转换）：选目标格式 → 文件对话框 → `ffmpeg -y -i src dst`（需系统装 ffmpeg）。
- 菜单接线：Help→关于、Tools→歌曲信息/格式转换 均打开对应模态。

### 3. View 菜单开关
- **菜单栏显隐**：新增 `MENUBAR_VISIBLE` 静态，`render()` 中 `show_menubar() && MENUBAR_VISIBLE` 双条件。
- **状态栏显隐**：复用已有 `STATUSBAR_VISIBLE` 静态，同样双条件门控。
- **均衡器菜单项**：从 `log!` 改为 `ACTIVE_PANEL = 7`（Equalizer 面板）。

### 4. 其他
- 清理了上一轮遗留的死代码 `layout.rs`（之前已删 menu_bar 骨架）。

## 运行方式（重要）
- ✅ 正确：`cargo run --bin hm-gui`（或 `target/debug/hm-gui.exe`）。
- ❌ 不要跑 `start.bat`：它 `cd gui && npm start`，但本仓库**没有 `gui/` 目录**，会直接报错。
- 若之前"没区别"：请确认跑的是 `hm-gui` 而非旧二进制；且需先加载音频文件（菜单/歌词/封面才会显示内容）。

## 仍待办（与"原版一致"的差距）
- **整体布局像素级还原**：当前是"功能完整但风格不同"的 GPUI 界面，与原版 MFC 的经典三栏布局仍有视觉差距（需大面积 reskin）。
- **歌词编辑器 9 按钮**（大多"待实现"）、**浮动播放列表独立窗口**、**始终置顶**（gpui 0.2.2 无 `set_always_on_top`，需平台 API）、**迷你模式专用布局细节**。
- **格式转换** 依赖系统 ffmpeg；若未安装会报错提示（功能本身可用）。
- GUI 行为需在真实窗口运行验证（无法 headless 自动化测试）。

## 构建状态
`cargo build` 与 `cargo build --bin hm-gui` 均通过（仅 15 个 unused 警告，无 error）。
