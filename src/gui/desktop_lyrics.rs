//! Desktop lyrics panel — displays karaoke-synced lyrics in the main GPUI window.
//! Supports dual-line original+translation, inline translation, and karaoke progress.

use gpui::*;
use gpui_component::{h_flex, v_flex};
use crate::gui::theme::UiColors;
use crate::lyric::{Lyrics, TranslateMode};

/// Lyrics display state tracked by the GUI.
#[derive(Clone)]
pub struct LyricsState {
    pub lyrics: Option<Lyrics>,
    pub current_index: Option<usize>,
    pub progress: u32, // 0-1000 karaoke progress
    pub visible: bool,
    pub error: Option<String>,
}

impl LyricsState {
    pub fn new() -> Self {
        Self {
            lyrics: None,
            current_index: None,
            progress: 0,
            visible: false,
            error: None,
        }
    }

    /// Update the lyrics data and compute which line is current + karaoke progress.
    pub fn update(&mut self, lyrics: Option<Lyrics>, position_ms: u64) {
        self.lyrics = lyrics;
        self.error = None;
        self.visible = true;
        self.recompute(position_ms);
    }

    /// Recompute current line index and progress for the given position.
    pub fn recompute(&mut self, position_ms: u64) {
        if let Some(ref lyrics) = self.lyrics {
            self.current_index = lyrics.current_line_index(position_ms);
            self.progress = lyrics.karaoke_progress(position_ms);
        } else {
            self.current_index = None;
            self.progress = 0;
        }
    }

    pub fn clear(&mut self) {
        self.lyrics = None;
        self.current_index = None;
        self.progress = 0;
        self.visible = false;
        self.error = None;
    }
}

impl Default for LyricsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a desktop lyrics panel showing karaoke-highlighted synced lyrics.
pub fn render_lyrics_panel(
    state: &LyricsState,
    c: &UiColors,
) -> impl IntoElement {
    if !state.visible {
        return v_flex().size_full();
    }

    let empty_text = "暂无歌词";
    let hint_text = "打开音乐文件以显示歌词";

    let Some(ref lyrics) = state.lyrics else {
        return v_flex()
            .size_full()
            .justify_center()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_size(px(16.0))
                    .text_color(c.text_dim)
                    .child(empty_text.to_string())
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(c.text_dim)
                    .child(hint_text.to_string())
            );
    };

    if lyrics.is_empty() {
        return v_flex()
            .size_full()
            .justify_center()
            .items_center()
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(c.text_dim)
                    .child("歌词文件为空".to_string())
            );
    }

    // Show track info at top
    let track_info = if !lyrics.title.is_empty() {
        format!("{} - {}", lyrics.title, lyrics.artist)
    } else {
        String::new()
    };

    let current_idx = state.current_index.unwrap_or(0);
    let progress = state.progress;
    let translate_mode = lyrics.translate_mode;

    // Display: show surrounding lines with the current one highlighted
    let total_lines = lyrics.len();
    let start = current_idx.saturating_sub(2);
    let end = (current_idx + 4).min(total_lines);

    let mut line_elements: Vec<gpui::AnyElement> = Vec::new();

    // Track info header
    if !track_info.is_empty() {
        line_elements.push(
            div()
                .w_full()
                .text_center()
                .py_2()
                .text_size(px(12.0))
                .text_color(c.text_dim)
                .child(track_info)
                .into_any_element()
        );
        line_elements.push(
            div()
                .w_full()
                .h(px(1.0))
                .bg(c.divider)
                .into_any_element()
        );
    }

    for i in start..end {
        let line = &lyrics.lines[i];
        let is_current = i == current_idx;
        let is_past = i < current_idx;

        let text_color = if is_current {
            c.text_title
        } else if is_past {
            c.text_dim
        } else {
            c.text
        };

        let font_size = if is_current { 16.0 } else { 13.0 };
        let font_weight = if is_current { FontWeight::BOLD } else { FontWeight::NORMAL };

        let display_text = lyrics.display_text(line);

        // Current line with karaoke progress highlight
        if is_current && progress > 0 && progress < 1000 {
            // Use gradient overlay for karaoke-style highlight
            line_elements.push(
                karaoke_line(&display_text, progress, font_size, font_weight, c)
                    .into_any_element()
            );
        } else {
            line_elements.push(
                div()
                    .w_full()
                    .text_center()
                    .py_1()
                    .text_size(px(font_size))
                    .text_color(text_color)
                    .font_weight(font_weight)
                    .child(display_text)
                    .into_any_element()
            );
        }

        // Show translation on separate line when mode is Separate
        if translate_mode == TranslateMode::Separate && !line.translate.is_empty() {
            let trans_color = if is_current { c.text } else { c.text_dim };
            line_elements.push(
                div()
                    .w_full()
                    .text_center()
                    .text_size(px(if is_current { 13.0 } else { 11.0 }))
                    .text_color(trans_color)
                    .child(line.translate.clone())
                    .into_any_element()
            );
        }
    }

    v_flex()
        .size_full()
        .flex_grow()
        .p_4()
        .gap_1()
        .children(line_elements)
}

/// Render a karaoke-highlighted text line where the first `progress/1000` fraction
/// is highlighted with the accent color and the rest uses the normal text color.
fn karaoke_line(
    text: &str,
    progress: u32,
    font_size: f32,
    _font_weight: FontWeight,
    c: &UiColors,
) -> impl IntoElement {
    if text.is_empty() || progress == 0 {
        return div()
            .w_full()
            .text_center()
            .py_1()
            .text_size(px(font_size))
            .text_color(c.text_title)
            .child(text.to_string());
    }

    if progress >= 1000 {
        return div()
            .w_full()
            .text_center()
            .py_1()
            .text_size(px(font_size))
            .text_color(c.accent)
            .child(text.to_string());
    }

    // Split text by progress ratio
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    let highlighted = (total * progress as usize / 1000).max(1).min(total);

    let highlighted_text: String = chars[..highlighted].iter().collect();
    let remaining_text: String = chars[highlighted..].iter().collect();

    div()
        .w_full()
        .text_center()
        .py_1()
        .child(
            h_flex()
                .justify_center()
                .child(
                    span(highlighted_text, font_size, c.accent)
                )
                .child(
                    span(remaining_text, font_size, c.text_title)
                )
        )
}

/// Helper to create a styled text span.
fn span(text: String, font_size: f32, color: Hsla) -> impl IntoElement {
    div()
        .text_size(px(font_size))
        .text_color(color)
        .child(text)
}

/// Render a compact lyrics overlay (used when desktop lyrics is toggled in BIG mode).
pub fn render_lyrics_overlay(
    state: &LyricsState,
    c: &UiColors,
) -> impl IntoElement {
    if !state.visible {
        return div().size_full();
    }

    let Some(ref lyrics) = state.lyrics else {
        return div().size_full();
    };

    let current_idx = state.current_index.unwrap_or(0);
    if current_idx >= lyrics.len() {
        return div().size_full();
    }

    let line = &lyrics.lines[current_idx];
    let progress = state.progress;
    let display_text = lyrics.display_text(line);

    v_flex()
        .size_full()
        .justify_center()
        .items_center()
        .gap_1()
        .bg(c.panel)
        .child(
            div()
                .w(DefiniteLength::Fraction(0.9))
                .child(karaoke_line(&display_text, progress, 18.0, FontWeight::BOLD, c))
        )
        .child({
            // Show translation below if separate mode and non-empty
            if lyrics.translate_mode == TranslateMode::Separate && !line.translate.is_empty() {
                div()
                    .w_full()
                    .text_center()
                    .pb_2()
                    .text_size(px(13.0))
                    .text_color(c.text_dim)
                    .child(line.translate.clone())
            } else {
                div()
            }
        })
}

