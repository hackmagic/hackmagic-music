use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemeName {
    Default,
    Ocean,
    Forest,
    Lavender,
}

impl ThemeName {
    pub fn from_config(s: &str) -> Self {
        match s {
            "ocean" => ThemeName::Ocean,
            "forest" => ThemeName::Forest,
            "lavender" => ThemeName::Lavender,
            _ => ThemeName::Default,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            ThemeName::Default => "default",
            ThemeName::Ocean => "ocean",
            ThemeName::Forest => "forest",
            ThemeName::Lavender => "lavender",
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
            bg: rgba(26, 28, 31),
            panel: rgba(36, 38, 41),
            panel_alt: rgba(46, 48, 51),
            titlebar_bg: rgba(18, 20, 23),
            menubar_bg: rgba(31, 33, 36),
            statusbar_bg: rgba(20, 23, 26),
            control_bar_bg: rgba(24, 26, 29),
            accent,
            text: rgba(235, 235, 242),
            text_dim: rgba(140, 140, 158),
            text_title: rgba(250, 250, 255),
            text_selected: accent,
            button: rgba(56, 56, 69),
            button_hover: rgba(76, 76, 89),
            button_press: rgba(97, 97, 112),
            button_accent: accent,
            button_accent_hover: lighten(accent, 0.1),
            button_disabled: rgba(38, 38, 46),
            progress_track: rgba(64, 64, 76),
            progress_fill: accent,
            progress_thumb: lighten(accent, 0.25),
            spectrum_bar: accent,
            spectrum_peak: lighten(accent, 0.35),
            playlist_item: rgba(41, 41, 48),
            playlist_item_hover: rgba(56, 56, 69),
            playlist_item_selected: darken(accent, 0.3),
            playlist_playing: darken(accent, 0.5),
            border: rgba(71, 71, 82),
            divider: rgba(51, 51, 61),
            btn_close: rgba(56, 56, 69),
            btn_close_hover: rgba(204, 51, 51),
            btn_minmax: rgba(56, 56, 69),
            btn_minmax_hover: rgba(76, 76, 89),
            shadow: hsla(0.0, 0.0, 0.0),
            scrollbar: rgba(71, 71, 82),
            scrollbar_hover: rgba(89, 89, 102),
        }
    }

    fn light_base(theme: &ThemeName) -> Self {
        let accent = light_accent(theme);
        Self {
            bg: rgba(240, 240, 245),
            panel: rgba(250, 250, 255),
            panel_alt: rgba(230, 230, 237),
            titlebar_bg: rgba(217, 217, 224),
            menubar_bg: rgba(230, 230, 237),
            statusbar_bg: rgba(217, 217, 224),
            control_bar_bg: rgba(245, 245, 250),
            accent,
            text: rgba(31, 31, 36),
            text_dim: rgba(115, 115, 128),
            text_title: rgba(13, 13, 20),
            text_selected: accent,
            button: rgba(217, 217, 224),
            button_hover: rgba(199, 199, 209),
            button_press: rgba(178, 178, 191),
            button_accent: accent,
            button_accent_hover: lighten(accent, 0.1),
            button_disabled: rgba(191, 191, 199),
            progress_track: rgba(199, 199, 209),
            progress_fill: accent,
            progress_thumb: lighten(accent, 0.15),
            spectrum_bar: accent,
            spectrum_peak: lighten(accent, 0.15),
            playlist_item: rgba(250, 250, 255),
            playlist_item_hover: rgba(235, 235, 245),
            playlist_item_selected: lighten(accent, 0.6),
            playlist_playing: lighten(accent, 0.4),
            border: rgba(178, 178, 188),
            divider: rgba(204, 204, 214),
            btn_close: rgba(217, 217, 224),
            btn_close_hover: rgba(204, 51, 51),
            btn_minmax: rgba(217, 217, 224),
            btn_minmax_hover: rgba(199, 199, 209),
            shadow: hsla(0.0, 0.0, 0.0),
            scrollbar: rgba(178, 178, 188),
            scrollbar_hover: rgba(158, 158, 168),
        }
    }
}

impl Default for UiColors {
    fn default() -> Self {
        Self::build(true, &ThemeName::Default)
    }
}

fn dark_accent(theme: &ThemeName) -> Hsla {
    match theme {
        ThemeName::Default => rgba(0, 120, 215),
        ThemeName::Ocean   => rgba(0, 150, 170),
        ThemeName::Forest  => rgba(60, 160, 80),
        ThemeName::Lavender => rgba(150, 120, 220),
    }
}

fn light_accent(theme: &ThemeName) -> Hsla {
    match theme {
        ThemeName::Default => rgba(0, 100, 200),
        ThemeName::Ocean   => rgba(0, 130, 150),
        ThemeName::Forest  => rgba(50, 140, 70),
        ThemeName::Lavender => rgba(130, 100, 200),
    }
}

pub const TITLEBAR_HEIGHT: f32 = 32.0;
pub const MENUBAR_HEIGHT: f32 = 24.0;
pub const STATUSBAR_HEIGHT: f32 = 22.0;
pub const CONTROL_BAR_HEIGHT: f32 = 96.0;
pub const SPECTRUM_HEIGHT: f32 = 60.0;
pub const PROGRESS_BAR_HEIGHT: f32 = 6.0;
pub const ALBUM_COVER_SIZE: f32 = 180.0;

pub const WINDOW_MIN_WIDTH: f32 = 760.0;
pub const WINDOW_MIN_HEIGHT: f32 = 480.0;
pub const NARROW_MODE_THRESHOLD: f32 = 600.0;

pub const BUTTON_SIZE: f32 = 32.0;
pub const BUTTON_ICON_SIZE: f32 = 18.0;
pub const BORDER_RADIUS: f32 = 4.0;
