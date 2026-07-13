pub mod playback;
pub mod audio;
pub mod effects;
pub mod playlist;
pub mod track;
pub mod media;
pub mod system;

use crate::cli::Commands;
use crate::error::Result;
use std::sync::Arc;
use crate::core::player::Player;

/// Global player reference for command handlers
static PLAYER: std::sync::OnceLock<Arc<Player>> = std::sync::OnceLock::new();

pub fn init_player(player: Arc<Player>) {
    PLAYER.set(player).ok();
}

pub fn get_player() -> &'static Arc<Player> {
    PLAYER.get().expect("Player not initialized")
}

/// Execute a command by dispatching to the appropriate handler
pub fn dispatch(command: &Commands) -> Result<()> {
    match command {
        Commands::Play(args) => playback::cmd_play(args),
        Commands::Pause => playback::cmd_pause(),
        Commands::Stop => playback::cmd_stop(),
        Commands::Next => playback::cmd_next(),
        Commands::Prev => playback::cmd_prev(),
        Commands::Seek(args) => playback::cmd_seek(args),
        Commands::Jump(args) => playback::cmd_jump(args),
        Commands::Volume(args) => audio::cmd_volume(args),
        Commands::Speed(args) => audio::cmd_speed(args),
        Commands::Pitch(args) => audio::cmd_pitch(args),
        Commands::Repeat(args) => audio::cmd_repeat(args),
        Commands::Eq(args) => effects::cmd_eq(args),
        Commands::Reverb(args) => effects::cmd_reverb(args),
        Commands::Ab(args) => effects::cmd_ab(args),
        Commands::Playlist(args) => playlist::cmd_playlist(args),
        Commands::Rate(args) => track::cmd_rate(args),
        Commands::Fav(args) => track::cmd_fav(args),
        Commands::Lyric(args) => track::cmd_lyric(args),
        Commands::Cover(args) => track::cmd_cover(args),
        Commands::Tag(args) => track::cmd_tag(args),
        Commands::Media(args) => media::cmd_media(args),
        Commands::Device(args) => system::cmd_device(args),
        Commands::Lastfm(args) => system::cmd_lastfm(args),
        Commands::Midi(args) => system::cmd_midi(args),
        Commands::Status => system::cmd_status(),
        Commands::Nowplaying(args) => system::cmd_nowplaying(args),
        Commands::Info(args) => system::cmd_info(args),
        Commands::Config(args) => system::cmd_config(args),
        Commands::Daemon(args) => system::cmd_daemon(args),
        Commands::Stats(args) => system::cmd_stats(args),
        Commands::Convert(args) => system::cmd_convert(args),
        Commands::Osu(args) => system::cmd_osu(args),
        Commands::Recent => system::cmd_recent(),
        Commands::Plugin(args) => system::cmd_plugin(args),
        Commands::Cue(_) => unreachable!(), // handled early in main.rs
        Commands::FileAssoc(args) => system::cmd_file_assoc(args),
        Commands::Musicbrainz(args) => track::cmd_musicbrainz(args),
        Commands::OpenLocation(args) => system::cmd_open_location(args),
        Commands::Completion(args) => system::cmd_completion(args),
}
}
