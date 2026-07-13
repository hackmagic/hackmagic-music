use crate::cli::{PlayArgs, SeekArgs, JumpArgs};
use crate::color;
use crate::commands::get_player;
use crate::config::RecentHistory;
use crate::error::Result;
use std::path::Path;
use std::time::Duration;

pub fn cmd_play(args: &PlayArgs) -> Result<()> {
    let player = get_player();

    if let Some(index) = args.index {
        if args.next {
            player.push_next_track(index);
            println!("Track #{index} queued for next play");
            return Ok(());
        }
        player.play_at_index(index)?;
        crate::play_stats::track_started(player.playlist_mut().current_track().map(|t| &*t.file_path));
        player.save_playback_state();
        return Ok(());
    }

    if args.next {
        eprintln!("Use --next with --index to queue a specific track");
        return Ok(());
    }

    if args.paths.is_empty() {
        if player.is_paused() {
            return player.toggle_pause();
        }
        if player.restore_playback_state() {
            println!("▶ Resumed from saved position");
            crate::play_stats::track_started(player.playlist_mut().current_track().map(|t| &*t.file_path));
        } else {
            eprintln!("No files specified. Use: 1028mp play <file> [files...]");
        }
        return Ok(());
    }

    let path = &args.paths[0];
    player.play_file(path)?;
    println!("▶ Playing: {path}");

    // Record recent folder/playlist
    let p = Path::new(path);
    if p.is_dir() {
        let mut recent = RecentHistory::load();
        recent.add_folder(path);
    } else if let Some(ext) = p.extension().and_then(|e| e.to_str()).map(str::to_lowercase) {
        if matches!(ext.as_str(), "playlist" | "m3u" | "m3u8" | "wpl" | "ttpl" | "csv") {
            let mut recent = RecentHistory::load();
            recent.add_playlist(path);
        }
    }

    if let Some(seek_secs) = args.seek {
        player.seek(Duration::from_secs(seek_secs))?;
    }

    crate::play_stats::track_started(Some(path));
    player.save_playback_state();

    // Show playback progress bar if --progress flag is set
    if args.progress {
        show_progress_bar(player);
    }

    Ok(())
}

/// Display a live-updating playback progress bar.
/// Runs until the player stops, Ctrl+C is pressed, or the track ends.
fn show_progress_bar(player: &crate::core::player::Player) {
    let bar_width: usize = 30;
    let interrupted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sig = interrupted.clone();
    // Set Ctrl+C handler to set the flag instead of causing a panic
    if ctrlc::set_handler(move || {
        sig.store(true, std::sync::atomic::Ordering::SeqCst);
    })
    .is_err()
    {
        tracing::debug!("Failed to set Ctrl+C handler for progress display");
    }

    loop {
        let state = player.state();
        if state == crate::core::engine_trait::EngineState::Stopped {
            println!();
            break;
        }

        let pos = player.position();
        let dur = player.duration();
        let pos_secs = pos.as_secs_f64();
        let dur_secs = dur.as_secs_f64();
        let pct = if dur_secs > 0.0 {
            (pos_secs / dur_secs) * 100.0
        } else {
            0.0
        };

        let state_icon = match state {
            crate::core::engine_trait::EngineState::Playing => "\u{25b6}", // ▶
            crate::core::engine_trait::EngineState::Paused => "\u{23f8}",  // ⏸
            crate::core::engine_trait::EngineState::Stopped => "\u{23f9}", // ⏹
        };

        let pos_str = format!("{:02}:{:02}", pos.as_secs() / 60, pos.as_secs() % 60);
        let dur_str = format!("{:02}:{:02}", dur.as_secs() / 60, dur.as_secs() % 60);

        // Build the progress bar
        let filled = if dur_secs > 0.0 {
            ((pos_secs / dur_secs) * bar_width as f64).round() as usize
        } else {
            0
        };
        let filled = filled.min(bar_width);
        let empty = bar_width - filled;
        let bar: String = std::iter::repeat_n('\u{2588}', filled)
            .chain(std::iter::repeat_n('\u{2591}', empty))
            .collect();

        // Get current track info
        let pl = player.playlist_mut();
        let track_name = pl.current_track()
            .map(|t| t.display_name(&player.display_format()))
            .unwrap_or_default();

        // Print progress line (carriage return to overwrite) with colours
        let state_color = match state {
            crate::core::engine_trait::EngineState::Playing => color::BRIGHT_GREEN,
            crate::core::engine_trait::EngineState::Paused => color::BRIGHT_YELLOW,
            crate::core::engine_trait::EngineState::Stopped => color::BRIGHT_RED,
        };
        let line = format!(
            "\r{state_color}{state_icon}{reset} {bold}{track_name}{reset}  \
             [{green}{bar}{reset}] \
             {yellow}{pct:5.1}%{reset}  \
             {cyan}{pos_str}/{dur_str}{reset}  ",
            state_color = state_color,
            reset = color::RESET,
            bold = color::BOLD,
            green = color::BRIGHT_GREEN,
            yellow = color::BRIGHT_YELLOW,
            cyan = color::BRIGHT_CYAN,
        );
        print!("{line}");
        use std::io::Write;
        let _ = std::io::stdout().flush();

        // Check for Ctrl+C
        if interrupted.load(std::sync::atomic::Ordering::SeqCst) {
            println!("\n⏹ Progress display stopped");
            break;
        }

        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

pub fn cmd_pause() -> Result<()> {
    let player = get_player();
    player.toggle_pause()?;
    match player.state() {
        crate::core::engine_trait::EngineState::Playing => println!("▶ Resumed"),
        crate::core::engine_trait::EngineState::Paused => println!("⏸ Paused"),
        _ => println!("⏹ Stopped"),
    }
    Ok(())
}

pub fn cmd_stop() -> Result<()> {
    let player = get_player();
    player.save_playback_state();
    player.stop()?;
    println!("⏹ Stopped");
    Ok(())
}

pub fn cmd_next() -> Result<()> {
    let player = get_player();
    player.next()?;
    crate::play_stats::track_started(player.playlist_mut().current_track().map(|t| &*t.file_path));
    player.save_playback_state();
    Ok(())
}

pub fn cmd_prev() -> Result<()> {
    let player = get_player();
    player.prev()?;
    crate::play_stats::track_started(player.playlist_mut().current_track().map(|t| &*t.file_path));
    player.save_playback_state();
    Ok(())
}

pub fn cmd_seek(args: &SeekArgs) -> Result<()> {
    let player = get_player();
    if args.relative {
        let pos = player.position();
        let delta: i64 = args.position.parse().unwrap_or(0);
        let new_pos = (pos.as_secs() as i64 + delta).max(0) as u64;
        player.seek(Duration::from_secs(new_pos))?;
    } else {
        let secs: f64 = args.position.parse().unwrap_or(0.0);
        player.seek(Duration::from_secs_f64(secs))?;
    }
    Ok(())
}

pub fn cmd_jump(args: &JumpArgs) -> Result<()> {
    let player = get_player();
    player.play_at_index(args.index)?;
    crate::play_stats::track_started(player.playlist_mut().current_track().map(|t| &*t.file_path));
    player.save_playback_state();
    Ok(())
}
