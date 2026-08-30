# HackMagic Music Player - AI 全链路UI测试指南

## 概述

本项目支持三种 AI 驱动的 UI 测试方式：

1. **事件日志分析** — AI 读取 `hm_gui.log` 中的 `[UI_EVENT]` 分析用户交互和错误
2. **Computer Use E2E测试** — AI 通过桌面自动化操作播放器并截图验证
3. **截图视觉审查** — AI 对比截图与原版 MusicPlayer2 找出 UI 差异

## 已验证记录（2026-08-30）

全链路已实机验证通过：

- `hm_gui.log` 出现 `[UI_EVENT] #0 StopPressed`、`#1 PrevPressed`、`#2 NextPressed`
  （真实鼠标点击 → gpui `on_click` → `log_event` → tracing → 日志文件）
- `hm-screenshot` 在 175% DPI 缩放显示器上输出 2124x1464 物理分辨率截图，
  界面完整清晰（菜单、传输栏、PLAY QUEUE 面板、状态栏均可辨识）
- 启动/运行 0 panic；关闭时偶发 4 条「无效窗口句柄」ERROR（窗口销毁后
  定时器回调的竞态，良性，待后续治理）

## 自动化操作注意事项（Windows）

- **GPUI 窗口不暴露内部控件给 UIA**：`get_app_state` 只能看到标题栏按钮，
  界面内控件必须用坐标点击（先截图，再按光栅像素坐标派发）。
- **合成点击需 DPI 感知**：PowerShell/脚本进程默认非 DPI 感知，
  `SetCursorPos` 坐标会被虚拟化缩放导致点偏。先调用 `SetProcessDPIAware()`
  再 `SetCursorPos` + `mouse_event`（示例见下）。
- **点击前先激活窗口**：`open_application(activate=true)` 把目标窗口置前台，
  否则原生点击会落在遮挡窗口上。
- **CUA 帧校验 vs GPUI 持续重渲染**：GPUI 以 30fps 重绘，computer-use 的
  全屏帧在派发前容易判定为 stale。优先使用窗口级光栅，或在截图后立即派发；
  仍失败时用上述 SetCursorPos/mouse_event 兜底。

```powershell
# DPI 感知合成点击示例（点击停止按钮）
Add-Type @'
using System; using System.Runtime.InteropServices;
public class Click {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint b, IntPtr e);
}
'@
[Click]::SetProcessDPIAware() | Out-Null
[Click]::SetCursorPos(366, 848) | Out-Null
[Click]::mouse_event(0x0002,0,0,0,[IntPtr]::Zero)  # down
[Click]::mouse_event(0x0004,0,0,0,[IntPtr]::Zero)  # up
```

## 快速开始

### 1. 启动应用（带事件日志）

```bash
cargo run --bin hm-gui
```

应用启动后会自动：
- 将所有 UI 事件记录到 `hm_gui.log`
- 将事件日志导出到 `hm_ui_event.log`（通过菜单导出）
- panic 信息写入 `hm_panic.log`

### 2. AI 执行 E2E 测试流程

AI 可以按以下步骤执行全链路测试：

#### 步骤 1: 启动应用
```bash
cargo run --bin hm-gui &
```

#### 步骤 2: 截图初始状态
- 截图窗口，记录初始 UI 状态
- 检查窗口标题栏、菜单栏、播放控制栏是否正确渲染

#### 步骤 3: 测试播放控制
1. 点击"文件"菜单 → "打开文件"
2. 选择一个音频文件（mp3/flac/wav）
3. 验证：曲目开始播放，标题/艺术家信息显示
4. 点击暂停按钮 → 验证播放暂停
5. 点击播放按钮 → 验证恢复播放
6. 点击停止按钮 → 验证停止播放
7. 点击下一曲/上一曲 → 验证切换

#### 步骤 4: 测试播放列表
1. 通过菜单添加文件夹
2. 验证播放列表显示所有曲目
3. 在搜索框输入关键词 → 验证过滤
4. 点击列标题 → 验证排序
5. 右键曲目 → 验证上下文菜单
6. 拖拽曲目 → 验证排序变化

#### 步骤 5: 测试面板切换
1. 点击侧边栏按钮切换面板
2. 验证每个面板正确显示

#### 步骤 6: 测试视图模式
1. 菜单"视图" → "迷你模式"
2. 验证窗口缩小
3. 再次点击恢复
4. "视图" → "深色模式" → 验证主题切换
5. "视图" → "全屏" → 验证全屏

#### 步骤 7: 导出事件日志
通过命令行导出：
```bash
# 日志自动在 hm_gui.log 中
# 也可以通过代码导出分析报告
```

#### 步骤 8: 分析日志
AI 读取 `hm_gui.log`，搜索 `[UI_EVENT]` 标记的事件，分析：
- 是否有 `[ERROR]` 级别事件
- 事件顺序是否合理
- 是否有异常状态转换

### 3. 截图对比原版

1. 运行原版 MusicPlayer2（K:\workspaces\github\MusicPlayer2\MusicPlayer2）
2. 截图原版各个界面
3. 运行 HackMagic
4. 截图对应界面
5. AI 对比两组截图，找出差异

## 事件日志格式

每个 UI 事件以 JSON 格式记录：

```json
{
  "seq": 1,
  "timestamp_ms": 1234,
  "event": {
    "type": "PlayPressed",
    "category": "playback"
  }
}
```

### 事件类型

| 类别 | 事件 | 说明 |
|------|------|------|
| playback | PlayPressed | 播放按钮点击 |
| playback | PausePressed | 暂停按钮点击 |
| playback | StopPressed | 停止按钮点击 |
| playback | NextPressed | 下一曲 |
| playback | PrevPressed | 上一曲 |
| playback | Seek | 进度跳转 |
| playback | VolumeChanged | 音量变化 |
| playlist | TrackAdded | 添加曲目 |
| playlist | TrackRemoved | 删除曲目 |
| playlist | TrackSelected | 选中曲目 |
| playlist | PlaylistCleared | 清空列表 |
| playlist | PlaylistSorted | 排序 |
| playlist | TrackMoved | 移动曲目 |
| panel | PanelSwitched | 面板切换 |
| view | MiniModeToggled | 迷你模式 |
| view | FullscreenToggled | 全屏 |
| view | DarkModeToggled | 深色模式 |
| error | Error | 错误事件 |

## 测试用例

预定义测试用例在 `src/ui_test.rs` 的 `default_test_cases()` 函数中：

1. **tc_playback_basic** — 基础播放控制
2. **tc_volume_control** — 音量控制
3. **tc_playlist_ops** — 播放列表操作
4. **tc_progress_seek** — 进度条拖动
5. **tc_panel_switch** — 面板切换
6. **tc_view_modes** — 视图模式切换

## AI 修复流程

1. 运行测试，收集日志和截图
2. 分析日志中的错误事件
3. 对比截图找 UI 差异
4. 定位代码问题
5. 修复并重新测试
6. 确认修复通过

## 文件位置

- 事件日志模块: `src/ui_test.rs`
- GUI 启动入口: `src/bin/gui.rs`
- GUI 主代码: `src/gui/mod.rs`
- 运行时日志: `hm_gui.log`
- Panic 日志: `hm_panic.log`
