//! Responsive layout — switches between normal and narrow mode
//! based on window width, and small mode based on window height.

use bevy::prelude::*;
use crate::gui::layout::{InfoPanel, PlaylistPanel, MainContent};
use crate::gui::styles::NARROW_MODE_THRESHOLD;

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct LayoutMode {
    pub narrow: bool,   // < 600px → stack vertically
    pub small: bool,    // < 260px height → hide playlist
}

impl Default for LayoutMode {
    fn default() -> Self {
        Self { narrow: false, small: false }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Monitor window size and update layout mode accordingly.
pub fn update_layout_mode(
    mut layout_mode: ResMut<LayoutMode>,
    window_q: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut main_content_q: Query<&mut Node, With<MainContent>>,
    mut info_q: Query<&mut Node, (With<InfoPanel>, Without<PlaylistPanel>)>,
    mut playlist_q: Query<&mut Node, (With<PlaylistPanel>, Without<InfoPanel>)>,
) {
    let Ok(window) = window_q.single() else { return };

    let was_narrow = layout_mode.narrow;
    let was_small = layout_mode.small;

    layout_mode.narrow = window.width() < NARROW_MODE_THRESHOLD;
    layout_mode.small = window.height() < 260.0;

    // Only update layout if mode changed
    if layout_mode.narrow != was_narrow || layout_mode.small != was_small {
        // Switch main content flex direction
        if let Ok(mut content) = main_content_q.single_mut() {
            content.flex_direction = if layout_mode.narrow {
                FlexDirection::Column
            } else {
                FlexDirection::Row
            };
        }

        // Adjust panel widths
        if let Ok(mut info) = info_q.single_mut() {
            info.width = if layout_mode.narrow {
                Val::Percent(100.0)
            } else {
                Val::Percent(50.0)
            };
            info.height = if layout_mode.narrow {
                Val::Auto
            } else {
                Val::Percent(100.0)
            };
        }

        if let Ok(mut pl) = playlist_q.single_mut() {
            pl.width = if layout_mode.narrow {
                Val::Percent(100.0)
            } else {
                Val::Percent(50.0)
            };
            // Hide playlist in small mode
            pl.display = if layout_mode.small {
                Display::None
            } else {
                Display::Flex
            };
        }
    }
}