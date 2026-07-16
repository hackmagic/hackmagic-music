use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use crate::gui::styles::*;
use crate::gui::PlayerResource;
use crate::gui::PlayerState;
use crate::gui::i18n::Locale;

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

#[derive(Component)]
pub struct VolumeSlider;
#[derive(Component)]
pub struct VolumeSliderFill;

#[derive(Component)]
pub struct StatusFps;
#[derive(Component)]
pub struct StatusNextTrack;
#[derive(Component)]
pub struct StatusRepeatMode;

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

        if prev.is_some() {
            tracing::info!("[Controls] Previous");
            let _ = player.0.prev();
        } else if play.is_some() {
            tracing::info!("[Controls] Toggle play/pause");
            let _ = player.0.toggle_pause();
        } else if stop.is_some() {
            tracing::info!("[Controls] Stop");
            let _ = player.0.stop();
        } else if next.is_some() {
            tracing::info!("[Controls] Next");
            let _ = player.0.next();
        } else if vd.is_some() {
            let vol = player.0.volume().saturating_sub(5);
            tracing::info!("[Controls] Volume down to {}", vol);
            let _ = player.0.volume_down(5);
        } else if vu.is_some() {
            let vol = (player.0.volume() + 5).min(100);
            tracing::info!("[Controls] Volume up to {}", vol);
            let _ = player.0.volume_up(5);
        } else if rep.is_some() {
            tracing::info!("[Controls] Cycle repeat mode");
            cycle_repeat_mode(&player, &mut state);
        }
    }
}

pub fn handle_sliders(
    player: Res<PlayerResource>,
    progress_q: Query<(&Interaction, &RelativeCursorPosition), (Changed<Interaction>, With<crate::gui::player_info::ProgressTrack>)>,
    volume_q: Query<(&Interaction, &RelativeCursorPosition), (Changed<Interaction>, With<VolumeSlider>)>,
) {
    for (interaction, cursor) in &progress_q {
        if *interaction == Interaction::Pressed {
            if let Some(position) = cursor.normalized {
                tracing::info!("[Controls] Seek to {:.1}%", position.x * 100.0);
                let _ = player.0.seek_percent(position.x as f64);
            }
        }
    }
    for (interaction, cursor) in &volume_q {
        if *interaction == Interaction::Pressed {
            if let Some(position) = cursor.normalized {
                let vol = (position.x * 100.0).round() as u32;
                tracing::info!("[Controls] Volume slider to {}%", vol);
                let _ = player.0.set_volume(vol);
            }
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
    tracing::info!("[Controls] Repeat mode: {:?} -> {:?}", current, next);
    player.0.set_repeat_mode(next);
    state.repeat_mode = format!("{next:?}");
}

pub fn update_status_dynamic(
    state: Res<PlayerState>,
    locale: Res<Locale>,
    mut play_btn_q: Query<&mut Text, (With<BtnPlayPause>, Without<StatusFps>, Without<StatusNextTrack>, Without<StatusRepeatMode>)>,
    mut fps_q: Query<&mut Text, (With<StatusFps>, Without<BtnPlayPause>, Without<StatusNextTrack>, Without<StatusRepeatMode>)>,
    mut next_q: Query<&mut Text, (With<StatusNextTrack>, Without<BtnPlayPause>, Without<StatusFps>, Without<StatusRepeatMode>)>,
    mut repeat_q: Query<&mut Text, (With<StatusRepeatMode>, Without<BtnPlayPause>, Without<StatusFps>, Without<StatusNextTrack>)>,
    time: Res<Time>,
) {
    let tr = locale.tr;
    if let Ok(mut text) = play_btn_q.single_mut() {
        text.0 = if state.is_playing { tr.ctrl_pause } else { tr.ctrl_play }.to_string();
    }
    if let Ok(mut text) = fps_q.single_mut() {
        text.0 = format!("{:.0} FPS", 1.0 / time.delta_secs());
    }
    if let Ok(mut text) = next_q.single_mut() {
        text.0 = tr.status_next_empty.to_string();
    }
    if let Ok(mut text) = repeat_q.single_mut() {
        let label = match state.repeat_mode.as_str() {
            "LoopPlaylist" | "PlayOrder" => tr.repeat_loop_pl,
            "LoopTrack" | "PlayTrack" => tr.repeat_single,
            "PlayRandom" => tr.repeat_random,
            "PlayShuffle" => tr.repeat_shuffle,
            _ => tr.repeat_loop_pl,
        };
        text.0 = label.to_string();
    }
}
