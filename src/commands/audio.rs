use crate::cli::{VolumeArgs, VolumeAction, SpeedArgs, SpeedAction, PitchArgs, PitchAction, RepeatArgs};
use crate::commands::get_player;
use crate::error::Result;

pub fn cmd_volume(args: &VolumeArgs) -> Result<()> {
    let player = get_player();
    match &args.action {
        VolumeAction::Get => {
            println!("{}", player.volume());
        }
        VolumeAction::Set(v) => {
            player.set_volume(v.value)?;
        }
        VolumeAction::Up(v) => {
            let step = v.step.unwrap_or(2);
            player.volume_up(step)?;
            println!("{}", player.volume());
        }
        VolumeAction::Down(v) => {
            let step = v.step.unwrap_or(2);
            player.volume_down(step)?;
            println!("{}", player.volume());
        }
    }
    Ok(())
}

pub fn cmd_speed(args: &SpeedArgs) -> Result<()> {
    let player = get_player();
    match &args.action {
        SpeedAction::Get => println!("{:.2}", player.speed()),
        SpeedAction::Set(v) => player.set_speed(v.value)?,
        SpeedAction::Up => player.speed_up()?,
        SpeedAction::Down => player.speed_down()?,
        SpeedAction::Reset => player.reset_speed()?,
    }
    Ok(())
}

pub fn cmd_pitch(args: &PitchArgs) -> Result<()> {
    let player = get_player();
    match &args.action {
        PitchAction::Get => println!("{}", player.pitch()),
        PitchAction::Set(v) => player.set_pitch(v.value)?,
        PitchAction::Up => player.pitch_up()?,
        PitchAction::Down => player.pitch_down()?,
        PitchAction::Reset => player.reset_pitch()?,
    }
    Ok(())
}

pub fn cmd_repeat(args: &RepeatArgs) -> Result<()> {
    let player = get_player();
    if args.status {
        let mode = player.repeat_mode();
        println!("{} ({})", mode.description(), mode.to_str());
        return Ok(());
    }
    if let Some(mode_str) = &args.mode {
        let mode = crate::core::playlist::RepeatMode::from_str(mode_str);
        player.set_repeat_mode(mode);
        println!("Repeat: {} ({})", mode.description(), mode.to_str());
    }
    Ok(())
}
