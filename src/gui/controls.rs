//! Playback control bar + status bar systems.

use bevy::prelude::*;
use crate::gui::styles::*;
use crate::gui::PlayerResource;
use crate::gui::PlayerState;

// ---------------------------------------------------------------------------
// Marker components for control buttons
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct BtnPrev;
#[derive(Component)]
pub struct BtnPlayPause;
#[derive(Component)]
pub struct BtnStop;
#[derive(Component)]
pub struct BtnNext;
#[derive(Component)]
pub struct BtnVolDown;
#[derive(Component)]
pub struct BtnVolUp;
#[derive(Component)]
pub struct BtnRepeatMode;

// Volume slider
#[derive(Component)]
pub struct VolumeSlider;
#[derive(Component)]
pub struct VolumeSliderFill;

// Status bar
#[derive(Component)]
pub struct StatusFps;
#[derive(Component)]
pub struct StatusNextTrack;
#[derive(Component)]
pub struct StatusRepeatMode;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Handle playback control button clicks.
/// Uses a SINGLE combined query with Option markers to avoid B0001 conflicts.
pub fn handle_controls(
    player: Res<PlayerResource>,
    colors: Res<UiColors>,
    mut state: ResMut<PlayerState>,
    mut interaction_q: Query<(
        &Interaction,
        &mut BackgroundColor,
        Entity,
        Option<&BtnPrev>,
        Option<&BtnPlayPause>,
        Option<&BtnStop>,
        Option<&BtnNext>,
        Option<&BtnVolDown>,
        Option<&BtnVolUp>,
        Option<&BtnRepeatMode>,
    ), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut bg, _entity, prev, play, stop, next, vd, vu, rep) in interaction_q.iter_mut() {
        let pressed = match *interaction {
            Interaction::Pressed => true,
            Interaction::Hovered => { bg.0 = colors.button_hover; false }
            Interaction::None => { bg.0 = colors.button; false }
        };
        if !pressed { continue; }

        bg.0 = colors.button_press;

        if prev.is_some() { let _ = player.0.prev(); }
        else if play.is_some() { let _ = player.0.toggle_pause(); }
        else if stop.is_some() { let _ = player.0.stop(); }
        else if next.is_some() { let _ = player.0.next(); }
        else if vd.is_some() { let _ = player.0.volume_down(5); }
        else if vu.is_some() { let _ = player.0.volume_up(5); }
        else if rep.is_some() { cycle_repeat_mode(&player, &mut state); }
    }
}

fn cycle_repeat_mode(player: &PlayerResource, state: &mut PlayerState) {
    use crate::core::playlist::RepeatMode;
    let current = player.0.repeat_mode();
    let next = match current {
        RepeatMode::PlayOrder | RepeatMode::LoopPlaylist => RepeatMode::LoopTrack,
        RepeatMode::LoopTrack | RepeatMode::PlayTrack => RepeatMode::PlayRandom,
        RepeatMode::PlayRandom | RepeatMode::PlayShuffle => RepeatMode::LoopPlaylist,
    };
    player.0.set_repeat_mode(next);
    state.repeat_mode = format!("{next:?}");
}

/// Update play/pause button icon.
pub fn update_play_button(
    state: Res<PlayerState>,
    mut btn_q: Query<&mut Text, With<BtnPlayPause>>,
) {
    if let Ok(mut text) = btn_q.single_mut() {
        text.0 = if state.is_playing { "\u{23F8}" } else { "\u{25B6}" }.to_string();
    }
}

/// Update status bar: FPS, next track, repeat mode.
pub fn update_status_bar(
    _state: Res<PlayerState>,
    player: Res<PlayerResource>,
    time: Res<Time>,
    mut fps_q: Query<&mut Text, (With<StatusFps>, Without<StatusNextTrack>, Without<StatusRepeatMode>)>,
    mut next_q: Query<&mut Text, (With<StatusNextTrack>, Without<StatusFps>, Without<StatusRepeatMode>)>,
    mut repeat_q: Query<&mut Text, (With<StatusRepeatMode>, Without<StatusFps>, Without<StatusNextTrack>)>,
) {
    // FPS
    if let Ok(mut text) = fps_q.single_mut() {
        text.0 = format!("FPS: {:.0}", 1.0 / time.delta_secs());
    }

    // Next track
    if let Ok(mut text) = next_q.single_mut() {
        let pl = player.0.playlist();
        let idx = pl.current_index();
        let next = idx.and_then(|i| pl.get(i + 1)).or_else(|| {
            if pl.len() > 0 { pl.get(0) } else { None }
        });
        text.0 = if let Some(t) = next {
            let title = if t.title.is_empty() { "Unknown" } else { &t.title };
            format!("\u{4E0B}\u{4E00}\u{9996}: {}", title)
        } else {
            "\u{4E0B}\u{4E00}\u{9996}: --".to_string()
        };
    }

    // Repeat mode
    if let Ok(mut text) = repeat_q.single_mut() {
        use crate::core::playlist::RepeatMode;
        let mode = player.0.repeat_mode();
        let (icon, label) = match mode {
            RepeatMode::LoopPlaylist => ("\u{1F503}", "\u{5FAA}\u{73AF}"),
            RepeatMode::LoopTrack => ("\u{1F501}", "\u{5355}\u{66F2}"),
            RepeatMode::PlayRandom => ("\u{1F500}", "\u{968F}\u{673A}"),
            RepeatMode::PlayShuffle => ("\u{1F500}", "\u{968F}\u{673A}"),
            RepeatMode::PlayOrder => ("\u{1F503}", "\u{987A}\u{5E8F}"),
            RepeatMode::PlayTrack => ("\u{1F501}", "\u{5355}\u{66F2}"),
        };
        text.0 = format!("{icon} {label}");
    }
}