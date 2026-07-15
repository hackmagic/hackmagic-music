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
pub fn handle_controls(
    player: Res<PlayerResource>,
    mut interaction_q: Query<
        (&Interaction, &mut BackgroundColor, Entity),
        (Changed<Interaction>, With<Button>),
    >,
    colors: Res<UiColors>,
    mut state: ResMut<PlayerState>,
    // Use a single marker query to avoid conflicts
    marker_q: Query<(
        Entity,
        Option<&BtnPrev>,
        Option<&BtnPlayPause>,
        Option<&BtnStop>,
        Option<&BtnNext>,
        Option<&BtnVolDown>,
        Option<&BtnVolUp>,
        Option<&BtnRepeatMode>,
    )>,
) {
    // Build a quick lookup: entity -> button type
    use std::collections::HashMap;
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum BtnType { Prev, PlayPause, Stop, Next, VolDown, VolUp, Repeat }
    let mut btn_map: HashMap<Entity, BtnType> = HashMap::new();
    for (entity, prev, play, stop, next, vd, vu, rep) in marker_q.iter() {
        if prev.is_some() { btn_map.insert(entity, BtnType::Prev); }
        else if play.is_some() { btn_map.insert(entity, BtnType::PlayPause); }
        else if stop.is_some() { btn_map.insert(entity, BtnType::Stop); }
        else if next.is_some() { btn_map.insert(entity, BtnType::Next); }
        else if vd.is_some() { btn_map.insert(entity, BtnType::VolDown); }
        else if vu.is_some() { btn_map.insert(entity, BtnType::VolUp); }
        else if rep.is_some() { btn_map.insert(entity, BtnType::Repeat); }
    }

    for (interaction, mut bg, entity) in interaction_q.iter_mut() {
        let pressed = match *interaction {
            Interaction::Pressed => true,
            Interaction::Hovered => { bg.0 = colors.button_hover; false }
            Interaction::None => { bg.0 = colors.button; false }
        };
        if !pressed { continue; }

        bg.0 = colors.button_press;

        match btn_map.get(&entity) {
            Some(BtnType::Prev) => { let _ = player.0.prev(); }
            Some(BtnType::PlayPause) => { let _ = player.0.toggle_pause(); }
            Some(BtnType::Stop) => { let _ = player.0.stop(); }
            Some(BtnType::Next) => { let _ = player.0.next(); }
            Some(BtnType::VolDown) => { let _ = player.0.volume_down(5); }
            Some(BtnType::VolUp) => { let _ = player.0.volume_up(5); }
            Some(BtnType::Repeat) => { cycle_repeat_mode(&player, &mut state); }
            None => {}
        }
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
    mut fps_q: Query<&mut Text, (With<StatusFps>, Without<StatusNextTrack>, Without<StatusRepeatMode>)>,
    mut next_q: Query<&mut Text, (With<StatusNextTrack>, Without<StatusFps>, Without<StatusRepeatMode>)>,
    mut repeat_q: Query<&mut Text, (With<StatusRepeatMode>, Without<StatusFps>, Without<StatusNextTrack>)>,
    state: Res<PlayerState>,
    player: Res<PlayerResource>,
    time: Res<Time>,
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