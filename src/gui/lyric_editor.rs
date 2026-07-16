//! Lyric editor panel — edit LRC lyrics with time tags, save/load, and timing adjustment.
//!
//! The editor state and pure logic functions are defined here. The actual GPUI
//! rendering lives in `MusicPlayer::render_lyric_editor_panel` in `gui/mod.rs`
//! (which needs `&mut self` access to modify the editor state).

use crate::lyric::Lyrics;
use crate::lyric::editor::{format_time_ms, parse_time_ms};

/// A single editable lyric row.
#[derive(Clone)]
pub struct LyricEditRow {
    pub timestamp: String, // [MM:SS.xx]
    pub text: String,
    pub original_index: usize,
}

/// Editor state for lyrics editing.
#[derive(Clone)]
pub struct LyricEditorState {
    /// The rows being edited
    pub rows: Vec<LyricEditRow>,
    /// Current lyrics being edited (if any)
    pub lyrics: Option<Lyrics>,
    /// Source file path (for save)
    pub file_path: String,
    /// Whether there are unsaved changes
    pub dirty: bool,
    /// Currently selected row index
    pub selected_row: Option<usize>,
    /// Pending UI event (set by button callbacks, processed in poll/render)
    pub pending_event: Option<String>,
}

impl LyricEditorState {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            lyrics: None,
            file_path: String::new(),
            dirty: false,
            selected_row: None,
            pending_event: None,
        }
    }

    /// Queue a UI event for processing.
    pub fn queue_event(&mut self, event: &str) {
        self.pending_event = Some(event.to_string());
    }

    /// Process the pending event if any, then clear it.
    pub fn process_pending_event(&mut self) -> Option<Result<(), String>> {
        if let Some(event) = self.pending_event.take() {
            let result = apply_event(self, &event);
            match result {
                Some(r) => Some(r),
                None => Some(Ok(())),  // Event was processed without needing a result
            }
        } else {
            None
        }
    }

    /// Load lyrics into the editor.
    pub fn load(&mut self, lyrics: Lyrics, file_path: String) {
        self.lyrics = Some(lyrics.clone());
        self.file_path = file_path;
        self.rows = lyrics
            .lines
            .iter()
            .enumerate()
            .map(|(i, line)| LyricEditRow {
                timestamp: format_time_ms(line.time_ms),
                text: line.text.clone(),
                original_index: i,
            })
            .collect();
        self.dirty = false;
        self.selected_row = None;
    }

    /// Load from a file path.
    pub fn load_from_file(&mut self, path: &str) -> Result<(), String> {
        match crate::lyric::load_lyric_file(path) {
            Ok(lyrics) => {
                self.load(lyrics, path.to_string());
                Ok(())
            }
            Err(e) => Err(format!("Failed to load: {}", e)),
        }
    }

    /// Insert a new row at the given index.
    pub fn insert_row(&mut self, at: usize) {
        let insert_idx = if at >= self.rows.len() {
            self.rows.len()
        } else {
            at + 1
        };
        let ts = if insert_idx > 0 && insert_idx <= self.rows.len() {
            self.rows[insert_idx - 1].timestamp.clone()
        } else if insert_idx + 1 < self.rows.len() {
            self.rows[insert_idx].timestamp.clone()
        } else {
            "[00:00.00]".to_string()
        };
        self.rows.insert(
            insert_idx,
            LyricEditRow {
                timestamp: ts,
                text: String::new(),
                original_index: 0,
            },
        );
        self.dirty = true;
        self.selected_row = Some(insert_idx);
    }

    /// Delete the row at the given index.
    pub fn delete_row(&mut self, idx: usize) {
        if idx < self.rows.len() {
            self.rows.remove(idx);
            self.dirty = true;
            self.selected_row = if self.rows.is_empty() {
                None
            } else {
                Some(idx.min(self.rows.len() - 1))
            };
        }
    }

    /// Adjust timing of a row by delta milliseconds.
    pub fn adjust_timing(&mut self, idx: usize, delta_ms: i64) {
        if idx >= self.rows.len() {
            return;
        }
        if let Some(ms) = parse_time_ms(&self.rows[idx].timestamp) {
            let new_ms = if delta_ms >= 0 {
                ms.saturating_add(delta_ms as u64)
            } else {
                ms.saturating_sub((-delta_ms) as u64)
            };
            self.rows[idx].timestamp = format_time_ms(new_ms);
            self.dirty = true;
        }
    }

    /// Shift all timestamps by a delta.
    pub fn shift_all_timing(&mut self, delta_ms: i64) {
        for row in &mut self.rows {
            if let Some(ms) = parse_time_ms(&row.timestamp) {
                let new_ms = if delta_ms >= 0 {
                    ms.saturating_add(delta_ms as u64)
                } else {
                    ms.saturating_sub((-delta_ms) as u64)
                };
                row.timestamp = format_time_ms(new_ms);
            }
        }
        self.dirty = true;
    }

    /// Update text of a row.
    pub fn update_text(&mut self, idx: usize, text: String) {
        if idx < self.rows.len() {
            self.rows[idx].text = text;
            self.dirty = true;
        }
    }

    /// Update timestamp of a row.
    pub fn update_timestamp(&mut self, idx: usize, ts: String) {
        if idx < self.rows.len() {
            self.rows[idx].timestamp = ts;
            self.dirty = true;
        }
    }

    /// Generate the LRC output string from the current rows.
    pub fn to_lrc_string(&self) -> String {
        lrc_string_from_state(self)
    }

    /// Save to the current file path.
    pub fn save(&mut self) -> Result<(), String> {
        if self.file_path.is_empty() {
            return Err("No file path set".to_string());
        }
        let content = self.to_lrc_string();
        match std::fs::write(&self.file_path, content) {
            Ok(()) => {
                self.dirty = false;
                Ok(())
            }
            Err(e) => Err(format!("Save failed: {}", e)),
        }
    }

    /// Save to a new file path (sets file_path and saves).
    pub fn save_as(&mut self, new_path: &str) -> Result<(), String> {
        let content = self.to_lrc_string();
        match std::fs::write(new_path, content) {
            Ok(()) => {
                self.file_path = new_path.to_string();
                self.dirty = false;
                Ok(())
            }
            Err(e) => Err(format!("Save failed: {}", e)),
        }
    }

    /// Clear the editor.
    pub fn clear(&mut self) {
        self.rows.clear();
        self.lyrics = None;
        self.file_path.clear();
        self.dirty = false;
        self.selected_row = None;
    }
}

impl Default for LyricEditorState {
    fn default() -> Self {
        Self::new()
    }
}

// Helper for LRC string generation directly from editor state.
fn lrc_string_from_state(state: &LyricEditorState) -> String {
    let rows: Vec<(String, String)> = state.rows.iter()
        .map(|r| (r.timestamp.clone(), r.text.clone()))
        .collect();
    let title = state.lyrics.as_ref().map(|l| l.title.as_str()).filter(|s| !s.is_empty());
    let artist = state.lyrics.as_ref().map(|l| l.artist.as_str()).filter(|s| !s.is_empty());
    let album = state.lyrics.as_ref().map(|l| l.album.as_str()).filter(|s| !s.is_empty());
    let offset = state.lyrics.as_ref().map(|l| l.offset_ms).filter(|&o| o != 0);
    crate::lyric::editor::to_lrc_string(&rows, title, artist, album, offset)
}

/// Apply a string command to the editor state.
/// Used by the UI to interpret button events.
pub fn apply_event(state: &mut LyricEditorState, event: &str) -> Option<Result<(), String>> {
    match event {
        "save" => Some(state.save()),
        "save_as" => Some(Err("需要文件对话框选择保存路径".to_string())),
        "open" => None,
        "insert_row" => {
            let at = state.selected_row.unwrap_or(state.rows.len().saturating_sub(1));
            state.insert_row(at);
            None
        }
        "delete_row" => {
            if let Some(idx) = state.selected_row {
                state.delete_row(idx);
            }
            None
        }
        "back_500" => {
            if let Some(idx) = state.selected_row {
                state.adjust_timing(idx, -500);
            }
            None
        }
        "back_100" => {
            if let Some(idx) = state.selected_row {
                state.adjust_timing(idx, -100);
            }
            None
        }
        "fwd_100" => {
            if let Some(idx) = state.selected_row {
                state.adjust_timing(idx, 100);
            }
            None
        }
        "fwd_500" => {
            if let Some(idx) = state.selected_row {
                state.adjust_timing(idx, 500);
            }
            None
        }
        "shift_all" => {
            state.shift_all_timing(100);
            None
        }
        _ => {
            if event.starts_with("select_row:") {
                if let Some(idx_str) = event.strip_prefix("select_row:") {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        if idx < state.rows.len() {
                            state.selected_row = Some(idx);
                        }
                    }
                }
            }
            None
        }
    }
}

/// Events emitted by the lyric editor UI (legacy - kept for compatibility).
#[derive(Debug, Clone)]
pub enum LyricEditorEvent {
    Save,
    SaveAs,
    InsertRow,
    DeleteRow,
    AdjustTiming(i64),
    ShiftAll(i64),
    SelectRow(usize),
    UpdateText(usize, String),
    UpdateTimestamp(usize, String),
    OpenFile,
    JumpToCurrent,
    TextChanged(usize, String),
}

/// Legacy convenience function: apply an editor event to the state.
pub fn apply_editor_event(state: &mut LyricEditorState, event: &LyricEditorEvent) -> Option<Result<(), String>> {
    match event {
        LyricEditorEvent::Save => Some(state.save()),
        LyricEditorEvent::SaveAs => Some(Err("需要文件对话框选择保存路径".to_string())),
        LyricEditorEvent::OpenFile => None,
        LyricEditorEvent::InsertRow => {
            let at = state.selected_row.unwrap_or(state.rows.len().saturating_sub(1));
            state.insert_row(at);
            None
        }
        LyricEditorEvent::DeleteRow => {
            if let Some(idx) = state.selected_row {
                state.delete_row(idx);
            }
            None
        }
        LyricEditorEvent::AdjustTiming(delta) => {
            if let Some(idx) = state.selected_row {
                state.adjust_timing(idx, *delta);
            }
            None
        }
        LyricEditorEvent::ShiftAll(delta) => {
            state.shift_all_timing(*delta);
            None
        }
        LyricEditorEvent::SelectRow(idx) => {
            if *idx < state.rows.len() {
                state.selected_row = Some(*idx);
            }
            None
        }
        LyricEditorEvent::UpdateText(idx, text) => {
            state.update_text(*idx, text.clone());
            None
        }
        LyricEditorEvent::UpdateTimestamp(idx, ts) => {
            state.update_timestamp(*idx, ts.clone());
            None
        }
        LyricEditorEvent::JumpToCurrent => None,
        LyricEditorEvent::TextChanged(idx, text) => {
            state.update_text(*idx, text.clone());
            None
        }
    }
}
