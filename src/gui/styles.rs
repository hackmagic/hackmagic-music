//! UI colour palette — inspired by MusicPlayer2's theming system.
//! Supports dark/light mode and theme-color overrides.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Full colour palette
// ---------------------------------------------------------------------------

/// All colours used by the UI, including extras not in the legacy UiColors.
#[derive(Resource)]
pub struct UiColors {
    // Backgrounds
    pub bg: Color,
    pub panel: Color,
    pub panel_alt: Color,
    pub titlebar_bg: Color,
    pub menubar_bg: Color,
    pub statusbar_bg: Color,

    // Text
    pub text: Color,
    pub text_dim: Color,
    pub text_title: Color,
    pub text_selected: Color,

    // Controls
    pub button: Color,
    pub button_hover: Color,
    pub button_press: Color,
    pub button_accent: Color,
    pub button_accent_hover: Color,
    pub button_disabled: Color,

    // Progress / slider
    pub progress_track: Color,
    pub progress_fill: Color,
    pub progress_thumb: Color,

    // Spectrum
    pub spectrum_bar: Color,
    pub spectrum_peak: Color,

    // Playlist
    pub playlist_item: Color,
    pub playlist_item_hover: Color,
    pub playlist_item_selected: Color,
    pub playlist_playing: Color,

    // Borders / dividers
    pub border: Color,
    pub divider: Color,

    // Window controls
    pub btn_close: Color,
    pub btn_close_hover: Color,
    pub btn_minmax: Color,
    pub btn_minmax_hover: Color,

    // Misc
    pub shadow: Color,
    pub scrollbar: Color,
    pub scrollbar_hover: Color,
}

impl UiColors {
    /// Dark theme (default).
    pub fn dark() -> Self {
        Self {
            bg:              Color::srgb(0.12, 0.12, 0.14),
            panel:           Color::srgb(0.16, 0.16, 0.19),
            panel_alt:       Color::srgb(0.19, 0.19, 0.22),
            titlebar_bg:     Color::srgb(0.08, 0.08, 0.10),
            menubar_bg:      Color::srgb(0.13, 0.13, 0.16),
            statusbar_bg:    Color::srgb(0.09, 0.09, 0.11),

            text:            Color::srgb(0.92, 0.92, 0.95),
            text_dim:        Color::srgb(0.55, 0.55, 0.62),
            text_title:      Color::srgb(0.98, 0.98, 1.0),
            text_selected:   Color::srgb(0.30, 0.60, 1.0),

            button:          Color::srgb(0.22, 0.22, 0.27),
            button_hover:    Color::srgb(0.30, 0.30, 0.35),
            button_press:    Color::srgb(0.38, 0.38, 0.44),
            button_accent:   Color::srgb(0.25, 0.55, 1.0),
            button_accent_hover: Color::srgb(0.35, 0.65, 1.0),
            button_disabled: Color::srgb(0.15, 0.15, 0.18),

            progress_track:  Color::srgb(0.25, 0.25, 0.30),
            progress_fill:   Color::srgb(0.25, 0.55, 1.0),
            progress_thumb:  Color::srgb(0.50, 0.75, 1.0),

            spectrum_bar:    Color::srgb(0.25, 0.55, 1.0),
            spectrum_peak:   Color::srgb(0.60, 0.80, 1.0),

            playlist_item:       Color::srgb(0.16, 0.16, 0.19),
            playlist_item_hover: Color::srgb(0.22, 0.22, 0.27),
            playlist_item_selected: Color::srgb(0.20, 0.35, 0.55),
            playlist_playing:    Color::srgb(0.20, 0.50, 0.90),

            border:          Color::srgb(0.28, 0.28, 0.32),
            divider:         Color::srgb(0.20, 0.20, 0.24),

            btn_close:       Color::srgb(0.22, 0.22, 0.27),
            btn_close_hover: Color::srgb(0.80, 0.20, 0.20),
            btn_minmax:      Color::srgb(0.22, 0.22, 0.27),
            btn_minmax_hover: Color::srgb(0.30, 0.30, 0.35),

            shadow:          Color::srgb(0.0, 0.0, 0.0),
            scrollbar:       Color::srgb(0.28, 0.28, 0.32),
            scrollbar_hover: Color::srgb(0.35, 0.35, 0.40),
        }
    }

    /// Light theme.
    pub fn light() -> Self {
        Self {
            bg:              Color::srgb(0.94, 0.94, 0.96),
            panel:           Color::srgb(0.98, 0.98, 1.0),
            panel_alt:       Color::srgb(0.90, 0.90, 0.93),
            titlebar_bg:     Color::srgb(0.85, 0.85, 0.88),
            menubar_bg:      Color::srgb(0.90, 0.90, 0.93),
            statusbar_bg:    Color::srgb(0.85, 0.85, 0.88),

            text:            Color::srgb(0.12, 0.12, 0.14),
            text_dim:        Color::srgb(0.45, 0.45, 0.50),
            text_title:      Color::srgb(0.05, 0.05, 0.08),
            text_selected:   Color::srgb(0.20, 0.50, 1.0),

            button:          Color::srgb(0.85, 0.85, 0.88),
            button_hover:    Color::srgb(0.78, 0.78, 0.82),
            button_press:    Color::srgb(0.70, 0.70, 0.75),
            button_accent:   Color::srgb(0.25, 0.55, 1.0),
            button_accent_hover: Color::srgb(0.35, 0.65, 1.0),
            button_disabled: Color::srgb(0.75, 0.75, 0.78),

            progress_track:  Color::srgb(0.78, 0.78, 0.82),
            progress_fill:   Color::srgb(0.25, 0.55, 1.0),
            progress_thumb:  Color::srgb(0.40, 0.70, 1.0),

            spectrum_bar:    Color::srgb(0.25, 0.55, 1.0),
            spectrum_peak:   Color::srgb(0.40, 0.65, 1.0),

            playlist_item:       Color::srgb(0.98, 0.98, 1.0),
            playlist_item_hover: Color::srgb(0.92, 0.92, 0.96),
            playlist_item_selected: Color::srgb(0.80, 0.90, 1.0),
            playlist_playing:    Color::srgb(0.70, 0.85, 1.0),

            border:          Color::srgb(0.70, 0.70, 0.74),
            divider:         Color::srgb(0.80, 0.80, 0.84),

            btn_close:       Color::srgb(0.85, 0.85, 0.88),
            btn_close_hover: Color::srgb(0.80, 0.20, 0.20),
            btn_minmax:      Color::srgb(0.85, 0.85, 0.88),
            btn_minmax_hover: Color::srgb(0.78, 0.78, 0.82),

            shadow:          Color::srgb(0.0, 0.0, 0.0),
            scrollbar:       Color::srgb(0.70, 0.70, 0.74),
            scrollbar_hover: Color::srgb(0.62, 0.62, 0.66),
        }
    }
}

impl Default for UiColors {
    fn default() -> Self {
        Self::dark()
    }
}

// ---------------------------------------------------------------------------
// Sizing constants
// ---------------------------------------------------------------------------

pub const TITLEBAR_HEIGHT: f32 = 32.0;
pub const MENUBAR_HEIGHT: f32 = 24.0;
pub const STATUSBAR_HEIGHT: f32 = 22.0;
pub const CONTROL_BAR_HEIGHT: f32 = 56.0;
pub const SPECTRUM_HEIGHT: f32 = 60.0;
pub const PROGRESS_BAR_HEIGHT: f32 = 6.0;
pub const ALBUM_COVER_SIZE: f32 = 180.0;

pub const WINDOW_MIN_WIDTH: f32 = 760.0;
pub const WINDOW_MIN_HEIGHT: f32 = 480.0;
pub const NARROW_MODE_THRESHOLD: f32 = 600.0;

pub const BUTTON_SIZE: f32 = 32.0;
pub const BUTTON_ICON_SIZE: f32 = 18.0;
pub const BORDER_RADIUS: f32 = 4.0;