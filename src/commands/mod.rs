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
        Commands::SearchLyrics(args) => media::cmd_search_lyrics(args),
        Commands::RebuildIndex => cmd_rebuild_index(),
}
}

/// Rebuild vector index: embed all tracks in media library via third-party API.
fn cmd_rebuild_index() -> Result<()> {
    use crate::vecdb;

    println!("Loading media library...");
    let lib = crate::media::MediaLib::load();
    let entries: Vec<_> = lib.entries.iter().collect();
    println!("Found {} tracks", entries.len());

    if entries.is_empty() {
        println!("No tracks to index.");
        return Ok(());
    }

    // Initialize vecdb (shared config-dir location, same as the GUI).
    let db_path = vecdb::default_db_path();
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    vecdb::init_vecdb(&db_path).map_err(|e| {
        crate::error::PlayerError::BassError(format!("Failed to init vecdb: {e}"))
    })?;
    let conn = vecdb::try_global_db().ok_or_else(|| {
        crate::error::PlayerError::BassError("vecdb not initialised".into())
    })?;

    let rt = crate::runtime();
    rt.block_on(async {
        vecdb::clear(conn).await.map_err(|e| {
            crate::error::PlayerError::BassError(format!("Failed to clear index: {e}"))
        })?;
        println!("Cleared old index.");

        let mut indexed = 0u32;
        let mut failed = 0u32;
        for (i, entry) in entries.iter().enumerate() {
            let path = &entry.file_path;
            let fname = std::path::Path::new(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            print!("\r[{}/{}] Indexing {}...", i + 1, entries.len(), fname);

            let lyric_text = crate::core::player::load_lyric_text(path, "");
            if lyric_text.trim().is_empty() {
                eprintln!("\n  No lyrics found for: {}", path);
                failed += 1;
                continue;
            }
            let emb = match crate::ai::embed_lyrics_text(&lyric_text) {
                Some(e) => e,
                None => {
                    eprintln!("\n  Embedding API failed for: {}", path);
                    failed += 1;
                    continue;
                }
            };
            if let Err(e) = vecdb::index_track(conn, path, &entry.title, &entry.artist, &entry.album, &lyric_text, &emb, &[]).await {
                eprintln!("\n  Failed to index {}: {}", path, e);
                failed += 1;
            } else {
                indexed += 1;
            }
        }
        println!("\nDone! Indexed: {}, Failed: {}", indexed, failed);
        Ok(())
    })
}
