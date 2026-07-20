# 实施进度：Phase 0 + Phase 1（HackMagic Music Player）

> 对照原版 MusicPlayer2 的缺陷分析与改进计划见 `docs/缺陷分析与改进计划.md`。
> 本文件记录已落地的修复。`cargo build`（全量，含 `hm` 与 `hm-gui`）已通过，无 error。

## ✅ B1 — 播放时界面冻结（最致命 bug，已修）

**根因**：GPUI 为立即模式，`render()` 只读 `poll_player_state` 但全仓无定时重绘驱动，
`cx.notify()` 从未被调用 → 声音正常但进度条/频谱/歌词冻结，直到移动鼠标才偶发刷新。

**修复**：在 `src/gui/mod.rs` 的 `MusicPlayer::new` 中新增 ~30fps 重绘定时器（基于 GPUI 0.2.2 的
`Context::spawn` + `AsyncApp::background_executor().timer()` + `WeakEntity::update().notify()`）。
定时器持有 `WeakEntity<MusicPlayer>`，窗口关闭后 `update` 返回 `Err` 自动退出循环。

**影响**：进度条、频谱条、歌词滚动现在随播放实时更新。

## ✅ 死代码清理（已修）

删除 `src/gui/layout.rs` 中未接线的骨架：`menu_bar` + 7 个空子菜单、`main_content`、
`icon_sidebar`、`sidebar_btn`、`control_bar`、`status_bar`。保留实际被 `mod.rs` 使用的
`txt` / `title_bar` / `menu_dropdown` / `content_area`（及依赖）。

## ✅ B2 — 均衡器接通音频引擎（已修）

- `eq_sliders` 改为命名局部变量，在 `new` 中为每个频段 `cx.subscribe`：拖动即调
  `player.eq_set(i, v as i32)`（仅启用时生效）。
- 启用开关 `on_click` 调 `player.eq_enable(...)`，启用时把各滑块当前值推给引擎、禁用时全置 0。
- 预设按钮：写入每个滑块 `set_value` 并（若启用）同步到引擎。
- 曲线预览由写死 `val=0.0` 改为读取真实滑块值。

## ✅ B3 — 专辑封面显示（已修）

- 新增 `album_art: Option<PathBuf>` 字段与 `extract_cover()` 方法：优先读取内嵌图
  （`tag::writer::read_pictures`，写入临时文件），其次目录内 `cover/folder/album/front.*`。
- 换曲时在 `poll_player_state` 中自动提取。
- 渲染用 `gpui::img(path)`：迷你模式 200px 主图、控制栏 48px 缩略图；无封面回退灰色占位。

## ✅ B4 — 深色 / 主题切换实时重绘（已修）

`render_menu_bar` 内取 `WeakEntity`；深色模式与"切换主题颜色"菜单项原本只写配置，
现在改为 `weak.update(cx, |this, cx| { this.colours = UiColors::build(dark, &theme); cx.notify(); })`，
**无需重启即生效**。

## ⏳ 待办（Phase 2 / Phase 3）

- **Lyric 菜单**：翻译显示、桌面歌词、±0.5s 偏移、复制当前/全部、保存、内嵌歌词——当前多为 log 占位。
- **歌词编辑器**：9 个按钮（保存/插行/删行/±100ms/±500ms 等）仍为"待实现"。
- **View 菜单**：浮动播放列表、始终置顶、状态栏（组件已写未接入）、迷你模式专用布局。
- **关于 / 歌曲信息对话框**：目前只 log，未弹窗。
- **格式转换、补齐其余缺失菜单项**。

> 说明：上述 GUI 行为需在真实窗口中运行验证（无法 headless 自动化测试）。
> 建议先 `cargo run --bin hm-gui` 试用 B1/B2/B3/B4 的实际效果，再推进 Phase 2/3。
