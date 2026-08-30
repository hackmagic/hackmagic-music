//! 全链路UI测试框架
//!
//! 提供三种测试能力：
//! 1. 事件日志系统 — 记录所有UI交互和状态变化
//! 2. GPUI集成测试 — 在代码层面验证UI组件渲染和交互
//! 3. 截图对比工具 — 运行时截图供AI视觉审查

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::VecDeque;
use serde::{Serialize, Deserialize};

// ============================================================================
// 1. 事件日志系统
// ============================================================================

/// UI事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiEvent {
    // 播放控制
    PlayPressed,
    PausePressed,
    StopPressed,
    NextPressed,
    PrevPressed,
    Seek { position_ms: u64 },
    VolumeChanged { volume: u32 },
    SpeedChanged { speed: f32 },
    PitchChanged { pitch: i32 },

    // 播放列表
    TrackAdded { path: String, index: usize },
    TrackRemoved { index: usize },
    TrackSelected { index: usize },
    PlaylistCleared,
    PlaylistSorted { mode: String },
    TrackMoved { from: usize, to: usize },
    SearchQueryChanged { text: String },

    // 菜单
    MenuItemClicked { menu: String, item: String },

    // 面板切换
    PanelSwitched { from: u8, to: u8 },

    // 歌词
    LyricLoaded { line_count: usize },
    LyricOffsetChanged { offset_ms: i64 },

    // 界面
    MiniModeToggled { is_mini: bool },
    FullscreenToggled { is_fullscreen: bool },
    DarkModeToggled { is_dark: bool },
    ThemeChanged { theme: String },

    // 错误
    Error { source: String, message: String },

    // 引擎
    EngineStateChanged { state: String },
    TrackOpened { path: String },
    TrackEnded { path: String },

    // 自定义
    Custom { category: String, message: String },
}

impl UiEvent {
    pub fn category(&self) -> &'static str {
        match self {
            Self::PlayPressed | Self::PausePressed | Self::StopPressed |
            Self::NextPressed | Self::PrevPressed | Self::Seek { .. } |
            Self::VolumeChanged { .. } | Self::SpeedChanged { .. } |
            Self::PitchChanged { .. } => "playback",

            Self::TrackAdded { .. } | Self::TrackRemoved { .. } |
            Self::TrackSelected { .. } | Self::PlaylistCleared |
            Self::PlaylistSorted { .. } | Self::TrackMoved { .. } |
            Self::SearchQueryChanged { .. } => "playlist",

            Self::MenuItemClicked { .. } => "menu",
            Self::PanelSwitched { .. } => "panel",

            Self::LyricLoaded { .. } | Self::LyricOffsetChanged { .. } => "lyric",

            Self::MiniModeToggled { .. } | Self::FullscreenToggled { .. } |
            Self::DarkModeToggled { .. } | Self::ThemeChanged { .. } => "view",

            Self::Error { .. } => "error",
            Self::EngineStateChanged { .. } | Self::TrackOpened { .. } |
            Self::TrackEnded { .. } => "engine",
            Self::Custom { .. } => "custom",
        }
    }
}

/// 事件日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// 单调递增序号
    pub seq: u64,
    /// 时间戳（毫秒，从日志系统启动开始）
    pub timestamp_ms: u64,
    /// 事件
    pub event: UiEvent,
}

/// 全局事件日志（环形缓冲区）
pub struct EventLog {
    entries: VecDeque<LogEntry>,
    start_time: std::time::Instant,
    seq: AtomicU64,
    max_entries: usize,
}

impl EventLog {
    fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries),
            start_time: std::time::Instant::now(),
            seq: AtomicU64::new(0),
            max_entries,
        }
    }

    /// 记录一个UI事件
    pub fn log(&mut self, event: UiEvent) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let timestamp_ms = self.start_time.elapsed().as_millis() as u64;
        let entry = LogEntry { seq, timestamp_ms, event };
        tracing::info!("[UI_EVENT] #{seq} {:?}", entry.event);
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// 导出所有日志为JSON
    pub fn export_json(&self) -> String {
        serde_json::to_string_pretty(&self.entries.iter().collect::<Vec<_>>())
            .unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
    }

    /// 导出日志到文件
    pub fn export_to_file(&self, path: &str) -> std::io::Result<()> {
        std::fs::write(path, self.export_json())
    }

    /// 按类别筛选事件
    pub fn filter_by_category(&self, category: &str) -> Vec<&LogEntry> {
        self.entries.iter()
            .filter(|e| e.event.category() == category)
            .collect()
    }

    /// 获取最近N条事件
    pub fn recent(&self, n: usize) -> Vec<&LogEntry> {
        self.entries.iter().rev().take(n).collect()
    }

    /// 清空日志
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 事件总数
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// 全局事件日志实例
static EVENT_LOG: Mutex<Option<EventLog>> = Mutex::new(None);

/// 初始化事件日志系统
pub fn init_event_log(max_entries: usize) {
    let mut log = EVENT_LOG.lock().unwrap();
    *log = Some(EventLog::new(max_entries));
    tracing::info!("[UI_TEST] 事件日志系统已初始化，最大条目数: {}", max_entries);
}

/// 记录UI事件（便捷函数）
pub fn log_event(event: UiEvent) {
    let mut log = EVENT_LOG.lock().unwrap();
    if let Some(log) = log.as_mut() {
        log.log(event);
    } else {
        // 如果未初始化，自动初始化
        let mut new_log = EventLog::new(10000);
        new_log.log(event);
        *log = Some(new_log);
    }
}

/// 导出事件日志到文件
pub fn export_log(path: &str) -> std::io::Result<()> {
    let log = EVENT_LOG.lock().unwrap();
    if let Some(log) = log.as_ref() {
        log.export_to_file(path)
    } else {
        Ok(())
    }
}

/// 获取事件日志摘要（用于AI分析）
pub fn get_log_summary() -> String {
    let log = EVENT_LOG.lock().unwrap();
    if let Some(log) = log.as_ref() {
        let mut summary = String::new();
        summary.push_str(&format!("事件总数: {}\n", log.len()));

        // 按类别统计
        let categories = ["playback", "playlist", "menu", "panel", "lyric", "view", "error", "engine"];
        for cat in &categories {
            let count = log.filter_by_category(cat).len();
            if count > 0 {
                summary.push_str(&format!("  {}: {} 条\n", cat, count));
            }
        }

        // 最近10条事件
        summary.push_str("\n最近10条事件:\n");
        for entry in log.recent(10) {
            summary.push_str(&format!("  #{} [{:?}] {:?}\n",
                entry.seq, entry.timestamp_ms, entry.event));
        }

        summary
    } else {
        "事件日志未初始化".to_string()
    }
}

// ============================================================================
// 2. 截图对比工具
// ============================================================================

/// 截图元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotMeta {
    pub timestamp: String,
    pub description: String,
    pub panel: u8,
    pub is_playing: bool,
    pub track_title: String,
}

/// 保存截图描述（实际截图由外部工具完成）
pub fn log_screenshot(description: &str, panel: u8, is_playing: bool, track_title: &str) {
    let meta = ScreenshotMeta {
        timestamp: chrono::Utc::now().to_rfc3339(),
        description: description.to_string(),
        panel,
        is_playing,
        track_title: track_title.to_string(),
    };
    tracing::info!("[SCREENSHOT] {:?}", meta);
    log_event(UiEvent::Custom {
        category: "screenshot".to_string(),
        message: serde_json::to_string(&meta).unwrap_or_default(),
    });
}

// ============================================================================
// 3. 测试断言工具
// ============================================================================

/// 测试断言结果
#[derive(Debug, Clone)]
pub struct AssertResult {
    pub passed: bool,
    pub message: String,
    pub expected: String,
    pub actual: String,
}

/// 验证播放状态
pub fn assert_playing(expected: bool, actual: bool) -> AssertResult {
    AssertResult {
        passed: expected == actual,
        message: format!("播放状态应为 {}", if expected { "播放中" } else { "已停止" }),
        expected: format!("{}", expected),
        actual: format!("{}", actual),
    }
}

/// 验证音量
pub fn assert_volume(expected: u32, actual: u32, tolerance: u32) -> AssertResult {
    let diff = actual.abs_diff(expected);
    AssertResult {
        passed: diff <= tolerance,
        message: format!("音量应为 {} ± {}", expected, tolerance),
        expected: format!("{}", expected),
        actual: format!("{}", actual),
    }
}

/// 验证播放列表长度
pub fn assert_playlist_len(expected: usize, actual: usize) -> AssertResult {
    AssertResult {
        passed: expected == actual,
        message: format!("播放列表长度应为 {}", expected),
        expected: format!("{}", expected),
        actual: format!("{}", actual),
    }
}

/// 验证当前曲目索引
pub fn assert_current_track(expected: Option<usize>, actual: Option<usize>) -> AssertResult {
    AssertResult {
        passed: expected == actual,
        message: format!("当前曲目索引应为 {:?}", expected),
        expected: format!("{:?}", expected),
        actual: format!("{:?}", actual),
    }
}

// ============================================================================
// 4. 测试用例定义（供AI执行）
// ============================================================================

/// 测试步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStep {
    pub action: String,
    pub expected: String,
    pub verify_command: String,
}

/// 测试用例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<TestStep>,
}

/// 预定义测试用例
pub fn default_test_cases() -> Vec<TestCase> {
    vec![
        TestCase {
            id: "tc_playback_basic".to_string(),
            name: "基础播放控制".to_string(),
            description: "测试播放/暂停/停止/上一曲/下一曲".to_string(),
            steps: vec![
                TestStep {
                    action: "点击播放按钮".to_string(),
                    expected: "音频开始播放，状态栏显示播放中".to_string(),
                    verify_command: "assert_playing(true, player.is_playing())".to_string(),
                },
                TestStep {
                    action: "点击暂停按钮".to_string(),
                    expected: "音频暂停，状态栏显示已暂停".to_string(),
                    verify_command: "assert_playing(false, player.is_playing())".to_string(),
                },
                TestStep {
                    action: "点击下一曲按钮".to_string(),
                    expected: "切换到下一首曲目，标题更新".to_string(),
                    verify_command: "assert_current_track(Some(idx+1), player.playlist().current_index())".to_string(),
                },
                TestStep {
                    action: "点击上一曲按钮".to_string(),
                    expected: "切换到上一首曲目，标题更新".to_string(),
                    verify_command: "assert_current_track(Some(idx), player.playlist().current_index())".to_string(),
                },
                TestStep {
                    action: "点击停止按钮".to_string(),
                    expected: "停止播放，进度归零".to_string(),
                    verify_command: "assert_playing(false, player.is_playing())".to_string(),
                },
            ],
        },
        TestCase {
            id: "tc_volume_control".to_string(),
            name: "音量控制".to_string(),
            description: "测试音量滑块和静音".to_string(),
            steps: vec![
                TestStep {
                    action: "拖动音量滑块到50%".to_string(),
                    expected: "音量变为50".to_string(),
                    verify_command: "assert_volume(50, player.volume(), 5)".to_string(),
                },
                TestStep {
                    action: "点击静音按钮".to_string(),
                    expected: "音量变为0，图标变为静音".to_string(),
                    verify_command: "assert_volume(0, player.volume(), 0)".to_string(),
                },
                TestStep {
                    action: "再次点击静音按钮".to_string(),
                    expected: "恢复之前的音量".to_string(),
                    verify_command: "assert_volume(50, player.volume(), 5)".to_string(),
                },
            ],
        },
        TestCase {
            id: "tc_playlist_ops".to_string(),
            name: "播放列表操作".to_string(),
            description: "测试添加/删除/排序/搜索".to_string(),
            steps: vec![
                TestStep {
                    action: "通过菜单添加文件夹".to_string(),
                    expected: "播放列表添加多首曲目".to_string(),
                    verify_command: "assert_playlist_len(>0, player.playlist().len())".to_string(),
                },
                TestStep {
                    action: "在搜索框输入关键词".to_string(),
                    expected: "播放列表过滤显示匹配项".to_string(),
                    verify_command: "check filtered list".to_string(),
                },
                TestStep {
                    action: "点击标题列排序".to_string(),
                    expected: "播放列表按标题排序".to_string(),
                    verify_command: "check sort order".to_string(),
                },
                TestStep {
                    action: "右键删除选中曲目".to_string(),
                    expected: "曲目从列表中移除".to_string(),
                    verify_command: "assert_playlist_len(old-1, player.playlist().len())".to_string(),
                },
            ],
        },
        TestCase {
            id: "tc_progress_seek".to_string(),
            name: "进度条拖动".to_string(),
            description: "测试进度条拖动和跳转".to_string(),
            steps: vec![
                TestStep {
                    action: "播放一首曲目".to_string(),
                    expected: "进度条开始移动".to_string(),
                    verify_command: "assert_playing(true, player.is_playing())".to_string(),
                },
                TestStep {
                    action: "拖动进度条到中间位置".to_string(),
                    expected: "播放位置跳转到中间".to_string(),
                    verify_command: "check position changed".to_string(),
                },
                TestStep {
                    action: "快进5秒按钮".to_string(),
                    expected: "位置前进5秒".to_string(),
                    verify_command: "check position += 5s".to_string(),
                },
                TestStep {
                    action: "快退5秒按钮".to_string(),
                    expected: "位置后退5秒".to_string(),
                    verify_command: "check position -= 5s".to_string(),
                },
            ],
        },
        TestCase {
            id: "tc_panel_switch".to_string(),
            name: "面板切换".to_string(),
            description: "测试切换不同面板".to_string(),
            steps: vec![
                TestStep {
                    action: "点击播放列表面板按钮".to_string(),
                    expected: "显示播放列表面板".to_string(),
                    verify_command: "check ACTIVE_PANEL == 0".to_string(),
                },
                TestStep {
                    action: "点击媒体库面板按钮".to_string(),
                    expected: "显示媒体库面板".to_string(),
                    verify_command: "check ACTIVE_PANEL == 2".to_string(),
                },
                TestStep {
                    action: "通过菜单打开设置面板".to_string(),
                    expected: "显示设置面板".to_string(),
                    verify_command: "check ACTIVE_PANEL == 8".to_string(),
                },
                TestStep {
                    action: "通过菜单打开均衡器".to_string(),
                    expected: "显示均衡器面板".to_string(),
                    verify_command: "check ACTIVE_PANEL == 7".to_string(),
                },
            ],
        },
        TestCase {
            id: "tc_view_modes".to_string(),
            name: "视图模式切换".to_string(),
            description: "测试迷你模式/全屏/深色模式".to_string(),
            steps: vec![
                TestStep {
                    action: "点击迷你模式菜单".to_string(),
                    expected: "窗口缩小为迷你模式".to_string(),
                    verify_command: "check MINI_MODE == true".to_string(),
                },
                TestStep {
                    action: "再次点击迷你模式菜单".to_string(),
                    expected: "窗口恢复正常大小".to_string(),
                    verify_command: "check MINI_MODE == false".to_string(),
                },
                TestStep {
                    action: "点击深色模式菜单".to_string(),
                    expected: "切换为深色/浅色主题".to_string(),
                    verify_command: "check config dark_mode".to_string(),
                },
                TestStep {
                    action: "点击全屏菜单".to_string(),
                    expected: "窗口切换为全屏".to_string(),
                    verify_command: "check window fullscreen".to_string(),
                },
            ],
        },
    ]
}

/// 导出测试用例为JSON（供AI读取执行）
pub fn export_test_cases(path: &str) -> std::io::Result<()> {
    let cases = default_test_cases();
    let json = serde_json::to_string_pretty(&cases)?;
    std::fs::write(path, json)
}

// ============================================================================
// 5. 日志分析工具（供AI判断问题）
// ============================================================================

/// 分析事件日志，找出异常
pub fn analyze_log() -> String {
    let log = EVENT_LOG.lock().unwrap();
    if let Some(log) = log.as_ref() {
        let mut report = String::new();
        report.push_str("=== 事件日志分析报告 ===\n\n");

        // 1. 事件统计
        report.push_str(&format!("总事件数: {}\n", log.len()));

        let categories = ["playback", "playlist", "menu", "panel", "lyric", "view", "error", "engine"];
        for cat in &categories {
            let events = log.filter_by_category(cat);
            if !events.is_empty() {
                report.push_str(&format!("  {}: {} 条\n", cat, events.len()));
            }
        }

        // 2. 错误分析
        let errors = log.filter_by_category("error");
        if !errors.is_empty() {
            report.push_str(&format!("\n⚠️ 发现 {} 个错误事件:\n", errors.len()));
            for e in errors.iter().take(20) {
                if let UiEvent::Error { source, message } = &e.event {
                    report.push_str(&format!("  #{} [{}] {}\n", e.seq, source, message));
                }
            }
        } else {
            report.push_str("\n✓ 无错误事件\n");
        }

        // 3. 播放状态分析
        let playback_events = log.filter_by_category("playback");
        let play_count = playback_events.iter()
            .filter(|e| matches!(e.event, UiEvent::PlayPressed))
            .count();
        let pause_count = playback_events.iter()
            .filter(|e| matches!(e.event, UiEvent::PausePressed))
            .count();
        report.push_str(&format!("\n播放控制统计: 播放={}, 暂停={}\n", play_count, pause_count));

        // 4. 最近事件时间线
        report.push_str("\n最近20条事件时间线:\n");
        for entry in log.recent(20) {
            report.push_str(&format!("  [{:.3}s] #{} {:?}\n",
                entry.timestamp_ms as f64 / 1000.0,
                entry.seq,
                entry.event));
        }

        report
    } else {
        "事件日志未初始化".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_log_basic() {
        let mut log = EventLog::new(100);
        log.log(UiEvent::PlayPressed);
        log.log(UiEvent::VolumeChanged { volume: 50 });
        log.log(UiEvent::Error {
            source: "test".to_string(),
            message: "test error".to_string(),
        });

        assert_eq!(log.len(), 3);
        assert_eq!(log.filter_by_category("playback").len(), 2);
        assert_eq!(log.filter_by_category("error").len(), 1);
    }

    #[test]
    fn test_event_log_ring_buffer() {
        let mut log = EventLog::new(3);
        log.log(UiEvent::PlayPressed);
        log.log(UiEvent::PausePressed);
        log.log(UiEvent::StopPressed);
        log.log(UiEvent::NextPressed); // 应该挤掉第一条

        assert_eq!(log.len(), 3);
        // 第一条应该被挤掉
        assert!(log.recent(3).iter().all(|e| e.seq > 0));
    }

    #[test]
    fn test_assert_playing() {
        assert!(assert_playing(true, true).passed);
        assert!(!assert_playing(true, false).passed);
        assert!(assert_playing(false, false).passed);
    }

    #[test]
    fn test_assert_volume() {
        assert!(assert_volume(50, 52, 5).passed);
        assert!(!assert_volume(50, 60, 5).passed);
    }

    #[test]
    fn test_default_test_cases() {
        let cases = default_test_cases();
        assert!(cases.len() >= 5);
        assert!(cases.iter().all(|c| !c.steps.is_empty()));
    }
}
