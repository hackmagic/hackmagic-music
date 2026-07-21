//! Responsive layout system - adapts UI based on window width/height
//! Mirrors the original MusicPlayer2's 3 layout modes: BIG / NARROW / SMALL

/// Layout mode based on window dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Full layout: left panel (album+lyrics+spectrum+controls) + right panel (playlist) (width >= 600, height >= 260)
    Big,
    /// Narrow layout: stacked elements, no separate panels (width < 600, height >= 260)
    Narrow,
    /// Small layout: minimal UI (height < 260)
    Small,
}

impl LayoutMode {
    /// Determine layout mode from window dimensions
    /// Original MusicPlayer2: width_threshold=600, height_threshold=260
    pub fn from_dimensions(width: f32, height: f32) -> Self {
        if width >= 600.0 && height >= 260.0 {
            LayoutMode::Big
        } else if height >= 260.0 {
            LayoutMode::Narrow
        } else {
            LayoutMode::Small
        }
    }

    /// Whether to show the menu bar
    pub fn show_menubar(&self) -> bool {
        !matches!(self, LayoutMode::Small)
    }

    /// Whether to show the status bar
    pub fn show_statusbar(&self) -> bool {
        matches!(self, LayoutMode::Big)
    }

    /// Whether to show album art
    pub fn show_album_art(&self) -> bool {
        matches!(self, LayoutMode::Big)
    }

    /// Whether to show spectrum visualizer
    pub fn show_spectrum(&self) -> bool {
        matches!(self, LayoutMode::Big)
    }

    /// Whether to show the search box
    pub fn show_search_box(&self) -> bool {
        matches!(self, LayoutMode::Big)
    }

    /// Whether to show the right panel (playlist dock)
    pub fn show_right_panel(&self) -> bool {
        matches!(self, LayoutMode::Big)
    }

    /// Control bar height
    pub fn control_bar_height(&self) -> f32 {
        match self {
            LayoutMode::Big => 48.0,
            LayoutMode::Narrow => 42.0,
            LayoutMode::Small => 36.0,
        }
    }

    /// Progress bar height
    pub fn progress_bar_height(&self) -> f32 {
        match self {
            LayoutMode::Big => 4.0,
            LayoutMode::Narrow => 3.0,
            LayoutMode::Small => 2.0,
        }
    }

    /// Button size for control buttons
    pub fn button_size(&self) -> f32 {
        match self {
            LayoutMode::Big => 36.0,
            LayoutMode::Narrow => 32.0,
            LayoutMode::Small => 28.0,
        }
    }

    /// Volume slider width
    pub fn volume_slider_width(&self) -> f32 {
        match self {
            LayoutMode::Big => 96.0,
            LayoutMode::Narrow => 72.0,
            LayoutMode::Small => 60.0,
        }
    }

    /// Font size for title text
    pub fn title_font_size(&self) -> f32 {
        match self {
            LayoutMode::Big => 13.0,
            LayoutMode::Narrow => 12.0,
            LayoutMode::Small => 11.0,
        }
    }

    /// Font size for artist text
    pub fn artist_font_size(&self) -> f32 {
        match self {
            LayoutMode::Big => 11.0,
            LayoutMode::Narrow => 10.0,
            LayoutMode::Small => 9.0,
        }
    }

    /// Left panel width percentage (0.0 - 1.0)
    pub fn left_panel_ratio(&self) -> f32 {
        match self {
            LayoutMode::Big => 0.5,
            LayoutMode::Narrow => 1.0, // full width
            LayoutMode::Small => 1.0,
        }
    }
}

/// Track responsive layout state for a window
pub struct ResponsiveState {
    pub mode: LayoutMode,
    pub window_width: f32,
    pub window_height: f32,
}

impl ResponsiveState {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            mode: LayoutMode::from_dimensions(width, height),
            window_width: width,
            window_height: height,
        }
    }

    pub fn update(&mut self, width: f32, height: f32) {
        self.window_width = width;
        self.window_height = height;
        self.mode = LayoutMode::from_dimensions(width, height);
    }
}

/// Helper to create responsive visibility conditions
pub fn responsive_visible(mode: LayoutMode, min_mode: LayoutMode) -> bool {
    let mode_level = match mode {
        LayoutMode::Small => 0,
        LayoutMode::Narrow => 1,
        LayoutMode::Big => 2,
    };
    let min_level = match min_mode {
        LayoutMode::Small => 0,
        LayoutMode::Narrow => 1,
        LayoutMode::Big => 2,
    };
    mode_level >= min_level
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_mode_big() {
        let mode = LayoutMode::from_dimensions(1200.0, 800.0);
        assert_eq!(mode, LayoutMode::Big);
        assert!(mode.show_menubar());
        assert!(mode.show_statusbar());
        assert!(mode.show_album_art());
        assert!(mode.show_right_panel());
    }

    #[test]
    fn test_layout_mode_narrow() {
        let mode = LayoutMode::from_dimensions(500.0, 500.0);
        assert_eq!(mode, LayoutMode::Narrow);
        assert!(mode.show_menubar());
        assert!(!mode.show_statusbar());
        assert!(!mode.show_album_art());
        assert!(!mode.show_right_panel());
    }

    #[test]
    fn test_layout_mode_small() {
        let mode = LayoutMode::from_dimensions(400.0, 200.0);
        assert_eq!(mode, LayoutMode::Small);
        assert!(!mode.show_menubar());
        assert!(!mode.show_statusbar());
        assert!(!mode.show_album_art());
    }

    #[test]
    fn test_boundary_values() {
        assert_eq!(LayoutMode::from_dimensions(600.0, 260.0), LayoutMode::Big);
        assert_eq!(LayoutMode::from_dimensions(599.9, 260.0), LayoutMode::Narrow);
        assert_eq!(LayoutMode::from_dimensions(600.0, 259.9), LayoutMode::Small);
        assert_eq!(LayoutMode::from_dimensions(0.0, 0.0), LayoutMode::Small);
    }

    #[test]
    fn test_responsive_state() {
        let mut state = ResponsiveState::new(1200.0, 800.0);
        assert_eq!(state.mode, LayoutMode::Big);

        state.update(500.0, 500.0);
        assert_eq!(state.mode, LayoutMode::Narrow);

        state.update(400.0, 200.0);
        assert_eq!(state.mode, LayoutMode::Small);
    }
}
