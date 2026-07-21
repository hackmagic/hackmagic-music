use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemeName {
    Default,
    Ocean,
    Forest,
    Lavender,
    Sunset,
    Midnight,
    Autumn,
    Spring,
}

impl ThemeName {
    pub fn from_config(s: &str) -> Self {
        match s {
            "ocean" => ThemeName::Ocean,
            "forest" => ThemeName::Forest,
            "lavender" => ThemeName::Lavender,
            "sunset" => ThemeName::Sunset,
            "midnight" => ThemeName::Midnight,
            "autumn" => ThemeName::Autumn,
            "spring" => ThemeName::Spring,
            _ => ThemeName::Default,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            ThemeName::Default => "default",
            ThemeName::Ocean => "ocean",
            ThemeName::Forest => "forest",
            ThemeName::Lavender => "lavender",
            ThemeName::Sunset => "sunset",
            ThemeName::Midnight => "midnight",
            ThemeName::Autumn => "autumn",
            ThemeName::Spring => "spring",
        }
    }
}

impl Default for ThemeName {
    fn default() -> Self {
        ThemeName::Default
    }
}

impl std::fmt::Display for ThemeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[derive(Clone)]
pub struct UiColors {
    pub bg: Hsla,
    pub panel: Hsla,
    pub panel_alt: Hsla,
    pub titlebar_bg: Hsla,
    pub menubar_bg: Hsla,
    pub statusbar_bg: Hsla,
    pub control_bar_bg: Hsla,
    pub accent: Hsla,
    pub text: Hsla,
    pub text_dim: Hsla,
    pub text_title: Hsla,
    pub text_selected: Hsla,
    pub button: Hsla,
    pub button_hover: Hsla,
    pub button_press: Hsla,
    pub button_accent: Hsla,
    pub button_accent_hover: Hsla,
    pub button_disabled: Hsla,
    pub progress_track: Hsla,
    pub progress_fill: Hsla,
    pub progress_thumb: Hsla,
    pub spectrum_bar: Hsla,
    pub spectrum_peak: Hsla,
    pub playlist_item: Hsla,
    pub playlist_item_hover: Hsla,
    pub playlist_item_selected: Hsla,
    pub playlist_playing: Hsla,
    pub border: Hsla,
    pub divider: Hsla,
    pub btn_close: Hsla,
    pub btn_close_hover: Hsla,
    pub btn_minmax: Hsla,
    pub btn_minmax_hover: Hsla,
    pub shadow: Hsla,
    pub scrollbar: Hsla,
    pub scrollbar_hover: Hsla,
}

type Hsla = gpui::Hsla;

fn hsla(h: f32, s: f32, l: f32) -> Hsla {
    Hsla { h, s, l, a: 1.0 }
}

fn rgba(r: u8, g: u8, b: u8) -> Hsla {
    gpui::Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
    .into()
}

fn lighten(c: Hsla, factor: f32) -> Hsla {
    Hsla {
        l: (c.l + factor).min(1.0),
        ..c
    }
}

fn darken(c: Hsla, factor: f32) -> Hsla {
    Hsla {
        l: (c.l - factor).max(0.0),
        ..c
    }
}

impl UiColors {
    pub fn build(dark_mode: bool, theme: &ThemeName) -> Self {
        if dark_mode { Self::dark_base(theme) } else { Self::light_base(theme) }
    }

    fn dark_base(theme: &ThemeName) -> Self {
        let accent = dark_accent(theme);
        Self {
            // Original MusicPlayer2 dark mode: dark gray background with amber/gold accents
            // ColorTable: dark2 = luminance(orig*0.3 + 20)
            bg: rgba(32, 32, 32),
            panel: rgba(40, 40, 40),
            panel_alt: rgba(50, 50, 50),
            titlebar_bg: rgba(38, 38, 38),
            menubar_bg: rgba(42, 42, 42),
            statusbar_bg: rgba(35, 35, 35),
            control_bar_bg: rgba(36, 36, 36),
            accent,
            text: rgba(240, 240, 240),
            text_dim: rgba(160, 160, 160),
            text_title: rgba(255, 255, 255),
            text_selected: accent,
            button: rgba(60, 60, 60),
            button_hover: rgba(75, 75, 75),
            button_press: rgba(90, 90, 90),
            button_accent: accent,
            button_accent_hover: lighten(accent, 0.1),
            button_disabled: rgba(45, 45, 45),
            progress_track: rgba(60, 60, 60),
            progress_fill: accent,
            progress_thumb: lighten(accent, 0.2),
            spectrum_bar: accent,
            spectrum_peak: lighten(accent, 0.3),
            playlist_item: rgba(40, 40, 40),
            playlist_item_hover: rgba(55, 55, 55),
            playlist_item_selected: darken(accent, 0.3),
            playlist_playing: darken(accent, 0.5),
            border: rgba(65, 65, 65),
            divider: rgba(55, 55, 55),
            btn_close: rgba(60, 60, 60),
            btn_close_hover: rgba(204, 51, 51),
            btn_minmax: rgba(60, 60, 60),
            btn_minmax_hover: rgba(75, 75, 75),
            shadow: hsla(0.0, 0.0, 0.0),
            scrollbar: rgba(65, 65, 65),
            scrollbar_hover: rgba(80, 80, 80),
        }
    }

    fn light_base(theme: &ThemeName) -> Self {
        let accent = light_accent(theme);
        Self {
            bg: rgba(240, 240, 240),
            panel: rgba(250, 250, 250),
            panel_alt: rgba(230, 230, 230),
            titlebar_bg: rgba(220, 220, 220),
            menubar_bg: rgba(235, 235, 235),
            statusbar_bg: rgba(225, 225, 225),
            control_bar_bg: rgba(245, 245, 245),
            accent,
            text: rgba(40, 40, 40),
            text_dim: rgba(120, 120, 120),
            text_title: rgba(20, 20, 20),
            text_selected: accent,
            button: rgba(220, 220, 220),
            button_hover: rgba(200, 200, 200),
            button_press: rgba(180, 180, 180),
            button_accent: accent,
            button_accent_hover: lighten(accent, 0.1),
            button_disabled: rgba(200, 200, 200),
            progress_track: rgba(200, 200, 200),
            progress_fill: accent,
            progress_thumb: lighten(accent, 0.1),
            spectrum_bar: accent,
            spectrum_peak: lighten(accent, 0.15),
            playlist_item: rgba(250, 250, 250),
            playlist_item_hover: rgba(238, 238, 238),
            playlist_item_selected: lighten(accent, 0.55),
            playlist_playing: lighten(accent, 0.4),
            border: rgba(180, 180, 180),
            divider: rgba(200, 200, 200),
            btn_close: rgba(220, 220, 220),
            btn_close_hover: rgba(204, 51, 51),
            btn_minmax: rgba(220, 220, 220),
            btn_minmax_hover: rgba(200, 200, 200),
            shadow: hsla(0.0, 0.0, 0.0),
            scrollbar: rgba(180, 180, 180),
            scrollbar_hover: rgba(160, 160, 160),
        }
    }
}

impl Default for UiColors {
    fn default() -> Self {
        Self::build(true, &ThemeName::Default)
    }
}

/// Dark mode accent colors — original MusicPlayer2 default is RGB(255, 168, 59) = gold/amber
fn dark_accent(theme: &ThemeName) -> Hsla {
    match theme {
        ThemeName::Default => rgba(255, 168, 59),   // Gold/Amber (original default)
        ThemeName::Ocean   => rgba(0, 170, 200),     // Teal
        ThemeName::Forest  => rgba(80, 180, 80),     // Green
        ThemeName::Lavender => rgba(160, 130, 230),  // Lavender
        ThemeName::Sunset  => rgba(230, 110, 60),    // Orange
        ThemeName::Midnight => rgba(100, 100, 220),  // Blue
        ThemeName::Autumn  => rgba(210, 130, 50),    // Warm orange
        ThemeName::Spring  => rgba(100, 190, 120),   // Fresh green
    }
}

/// Light mode accent colors
fn light_accent(theme: &ThemeName) -> Hsla {
    match theme {
        ThemeName::Default => rgba(220, 140, 30),    // Deeper gold for light mode
        ThemeName::Ocean   => rgba(0, 140, 170),
        ThemeName::Forest  => rgba(50, 150, 60),
        ThemeName::Lavender => rgba(130, 100, 200),
        ThemeName::Sunset  => rgba(200, 90, 30),
        ThemeName::Midnight => rgba(70, 70, 180),
        ThemeName::Autumn  => rgba(180, 110, 20),
        ThemeName::Spring  => rgba(60, 160, 80),
    }
}

// ── Layout constants ────────────────────────────────────────────────
pub const TITLEBAR_HEIGHT: f32 = 28.0;
pub const MENUBAR_HEIGHT: f32 = 24.0;
pub const STATUSBAR_HEIGHT: f32 = 22.0;

/// Left panel default width (album + lyrics + controls)
pub const LEFT_PANEL_WIDTH: f32 = 420.0;
/// Right panel default width (playlist)
pub const RIGHT_PANEL_WIDTH: f32 = 380.0;

pub const WINDOW_MIN_WIDTH: f32 = 600.0;
pub const WINDOW_MIN_HEIGHT: f32 = 400.0;
pub const NARROW_MODE_THRESHOLD: f32 = 600.0;

pub const BUTTON_SIZE: f32 = 32.0;
pub const BUTTON_ICON_SIZE: f32 = 18.0;
pub const BORDER_RADIUS: f32 = 4.0;

/// Playlist toolbar height (search + add/delete/sort row)
pub const PLAYLIST_TOOLBAR_HEIGHT: f32 = 28.0;

/// Left navigation rail width (for nav_rail method, kept for compatibility)
pub const NAV_RAIL_WIDTH: f32 = 140.0;

/// Right-docked playlist column width in Big mode
pub const PLAYLIST_DOCK_WIDTH: f32 = 380.0;
