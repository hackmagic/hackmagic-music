use crate::cli::{DeviceArgs, DeviceAction, LastfmArgs, LastfmAction, PluginArgs, PluginAction, MidiArgs, MidiAction, NowplayingArgs, InfoArgs, InfoAction, ConfigArgs, ConfigAction, CueArgs, DaemonArgs, DaemonAction, OsuArgs, OsuAction, ConvertArgs, ConvertAction, StatsArgs, StatsAction, FileAssocArgs, FileAssocAction, OpenLocationArgs, CompletionArgs};
use crate::commands::get_player;
use crate::config::Config;
use crate::core::engine_trait::EngineState;
use crate::error::{PlayerError, Result};
use std::time::Duration;

pub fn cmd_device(args: &DeviceArgs) -> Result<()> {
    match &args.action {
        #[cfg(windows)]
        DeviceAction::List => {
            let count = crate::bass::sys::BASS_GetDeviceCount();
            for i in 0..count {
                if let Some(name) = crate::bass::sys::get_device_name(i) {
                    println!("{i}: {name}");
                }
            }
        }
        #[cfg(not(windows))]
        DeviceAction::List => {
            println!("Audio device listing requires BASS engine (Windows only)");
        }
        #[cfg(windows)]
        DeviceAction::Set(v) => {
            let target = if let Ok(idx) = v.name_or_index.parse::<i32>() {
                idx
            } else {
                let count = crate::bass::sys::BASS_GetDeviceCount();
                let mut found = -1;
                for i in 0..count {
                    if let Some(name) = crate::bass::sys::get_device_name(i) {
                        if name.to_lowercase().contains(&v.name_or_index.to_lowercase()) {
                            found = i as i32;
                            break;
                        }
                    }
                }
                found
            };

            if target >= 0 && crate::bass::sys::BASS_SetDevice(target as u32) {
                println!("Switched to device {target}");
            } else {
                eprintln!("Device not found: {}", v.name_or_index);
            }
        }
        #[cfg(not(windows))]
        DeviceAction::Set(v) => {
            eprintln!("Audio device switching requires BASS engine (Windows only): {}", v.name_or_index);
        }
        #[cfg(windows)]
        DeviceAction::Bluetooth => {
            let monitor = crate::smtc::BluetoothMonitor::new();
            let devices = monitor.devices();
            if devices.is_empty() {
                println!("No Bluetooth audio devices found.");
                println!("  Note: Only BASS-recognized Bluetooth (A2DP) devices are listed.");
            } else {
                println!("Bluetooth audio devices:");
                println!("  ID  Status     Name");
                for dev in &devices {
                    let status = if dev.enabled { "enabled" } else { "disabled" };
                    let marker = if dev.is_default { " *" } else { "  " };
                    println!("  {:>2}  {:<9} {}{}", dev.device_id, status, dev.name, marker);
                }
                if devices.iter().any(|d| d.is_default) {
                    println!("  (*) = currently active output device");
                }
            }
        }
        #[cfg(not(windows))]
        DeviceAction::Bluetooth => {
            println!("Bluetooth device listing requires BASS engine (Windows only)");
        }
    }
    Ok(())
}

pub fn cmd_lastfm(args: &LastfmArgs) -> Result<()> {
    match &args.action {
        LastfmAction::Status => {
            let cfg = Config::load();
            if cfg.lastfm.enabled && !cfg.lastfm.api_key.is_empty() {
                println!("Last.fm: enabled");
                println!("  User:       {}", cfg.lastfm.username);
                println!("  Session:    {}", if cfg.lastfm.session_key.is_empty() { "not authenticated" } else { "authenticated" });
                println!("  Auto-scrobble: {}", cfg.lastfm.auto_scrobble);
                println!("  Min duration:  {}s (or {}%)", cfg.lastfm.least_dur, cfg.lastfm.least_perdur);
            } else {
                println!("Last.fm: not configured");
                println!("  Register at https://www.last.fm/api/ to get API credentials");
                println!("  Then use: lastfm login <username> <password> <api_key> <shared_secret>");
            }
        }
        LastfmAction::Login(v) => {
            let parts: Vec<&str> = v.password.split_whitespace().collect();
            if parts.len() >= 2 {
                Config::set("lastfm.enabled", "true").ok();
                Config::set("lastfm.username", &v.username).ok();
                Config::set("lastfm.api_key", parts[0]).ok();
                Config::set("lastfm.shared_secret", parts[1]).ok();

                if parts.len() >= 3 {
                    let token = parts[2];
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    match rt.block_on(crate::lastfm::LastfmApi::get_session(parts[0], parts[1], token)) {
                        Ok((session_key, name)) => {
                            Config::set("lastfm.session_key", &session_key).ok();
                            if !name.is_empty() {
                                Config::set("lastfm.username", &name).ok();
                            }
                            println!("Last.fm: authenticated as '{name}'");
                        }
                        Err(e) => {
                            eprintln!("Last.fm auth failed: {e}");
                            eprintln!("Make sure you authorized the token at:");
                            eprintln!("  https://www.last.fm/api/auth/?api_key={}&token={}", parts[0], token);
                        }
                    }
                } else {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    match rt.block_on(crate::lastfm::LastfmApi::authenticate(parts[0], parts[1])) {
                        Ok(token) => {
                            println!("After authorizing, run with the token as 4th argument:");
                            println!("  lastfm login {} {} {} {}", v.username, parts[0], parts[1], token);
                        }
                        Err(e) => eprintln!("Last.fm auth failed: {e}"),
                    }
                }
            } else {
                Config::set("lastfm.enabled", "true").ok();
                Config::set("lastfm.username", &v.username).ok();
                Config::set("lastfm.password", &v.password).ok();
                println!("Last.fm login saved: {}", v.username);
            }
        }
        LastfmAction::Love => {
            let pl = get_player().playlist_mut();
            if let Some(track) = pl.current_track() {
                if let Some(api) = crate::lastfm::LastfmApi::from_config() {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    match rt.block_on(api.love(track)) {
                        Ok(()) => println!("\u{2665} Loved on Last.fm: {}", track.display_name("title")),
                        Err(e) => eprintln!("Last.fm love failed: {e}"),
                    }
                } else {
                    println!("\u{2665} Loved (local only): {}", track.display_name("title"));
                }
            }
        }
        LastfmAction::Unlove => {
            let pl = get_player().playlist_mut();
            if let Some(track) = pl.current_track() {
                if let Some(api) = crate::lastfm::LastfmApi::from_config() {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    match rt.block_on(api.unlove(track)) {
                        Ok(()) => println!("\u{2661} Un-loved on Last.fm: {}", track.display_name("title")),
                        Err(e) => eprintln!("Last.fm unlove failed: {e}"),
                    }
                } else {
                    println!("\u{2661} Un-loved (local only): {}", track.display_name("title"));
                }
            }
        }
        LastfmAction::Scrobble => {
            let pl = get_player().playlist_mut();
            if let Some(track) = pl.current_track() {
                let pos = get_player().position();
                if let Some(api) = crate::lastfm::LastfmApi::from_config() {
                    if crate::lastfm::LastfmApi::should_scrobble(
                        track, pos.as_secs_f64(),
                        Config::load().lastfm.least_perdur,
                        Config::load().lastfm.least_dur,
                    ) {
                        let ts = chrono::Utc::now().timestamp();
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        match rt.block_on(api.scrobble(track, ts)) {
                            Ok(()) => println!("Scrobbled to Last.fm: {} (at {:.0}s)", track.display_name("title"), pos.as_secs()),
                            Err(e) => eprintln!("Scrobble failed: {e}"),
                        }
                    } else {
                        println!("Track not eligible for scrobble (needs >={}s and >={}% played)",
                            Config::load().lastfm.least_dur, Config::load().lastfm.least_perdur);
                    }
                } else {
                    println!("Scrobble requires Last.fm authentication");
                }
            }
        }
    }
    Ok(())
}

pub fn cmd_plugin(args: &PluginArgs) -> Result<()> {
    match &args.action {
        PluginAction::Load { path } => {
            let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            match crate::bass::sys::BASS_PluginLoad(path_wide.as_ptr(), 0) {
                Ok(handle) => println!("Plugin loaded successfully (handle: {handle})"),
                Err(e) => eprintln!("Failed to load plugin '{path}': {e}"),
            }
        }
        PluginAction::List => {
            println!("BASS plugin listing not directly supported via BASS API.");
            println!("Use `plugin load <path>` to load a plugin DLL.");
        }
    }
    Ok(())
}

pub fn cmd_midi(args: &MidiArgs) -> Result<()> {
    match &args.action {
        MidiAction::Soundfont(v) => {
            let sf_path = &v.path;
            let abs_path = std::path::Path::new(sf_path);
            if !abs_path.exists() {
                eprintln!("SoundFont file not found: {sf_path}");
                return Ok(());
            }
            // Save soundfont path to config
            Config::set("midi.soundfont", sf_path).ok();
            Config::set("midi.enabled", "true").ok();

            // Try to load BASS_MIDI plugin (platform-specific lib name)
            let plugin_name = if cfg!(target_os = "windows") {
                "bassmidi.dll"
            } else if cfg!(target_os = "macos") {
                "libbassmidi.dylib"
            } else {
                "libbassmidi.so"
            };
            let plugin_wide: Vec<u16> = plugin_name.encode_utf16().chain(std::iter::once(0)).collect();
            match crate::bass::sys::BASS_PluginLoad(plugin_wide.as_ptr(), 0) {
                Ok(handle) => {
                    println!("\u{2705} SoundFont set and BASS_MIDI loaded: {sf_path}");
                    println!("   Plugin handle: {handle}");
                }
                Err(e) => {
                    println!("\u{2705} SoundFont path saved: {sf_path}");
                    println!("   BASS_MIDI plugin not loaded: {e}");
                    println!("   Make sure {plugin_name} is in the same directory as the executable.");
                }
            }
        }
        MidiAction::Lyric => {
            let cfg = Config::load();
            if cfg.midi.enabled && !cfg.midi.soundfont.is_empty() {
                println!("MIDI lyric display");
                println!("  SoundFont: {}", cfg.midi.soundfont);
                println!("  (MIDI lyric display requires bassmidi.dll and active MIDI playback)");
            } else {
                println!("MIDI not configured. Set a SoundFont with: midi soundfont <path>");
            }
        }
    }
    Ok(())
}

pub fn cmd_status() -> Result<()> {
    let player = get_player();
    let state = player.state();
    let state_str = match state {
        EngineState::Playing => "\u{25b6} Playing",
        EngineState::Paused => "\u{23f8} Paused",
        EngineState::Stopped => "\u{23f9} Stopped",
    };

    let pos = player.position();
    let dur = player.duration();
    let vol = player.volume();
    let speed = player.speed();
    let pitch = player.pitch();

    println!("\u{250c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("\u{2502} HackMagic Music Player v{}              \u{2502}", env!("CARGO_PKG_VERSION"));
    println!("\u{251c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
    println!("\u{2502} Engine: {}                         \u{2502}", player.engine_name());
    println!("\u{2502} State:  {state_str}                         \u{2502}");
    println!("\u{2502} Volume: {vol}%                        \u{2502}");
    if dur > Duration::ZERO {
        let pct = if dur.as_secs() > 0 { (pos.as_secs_f64() / dur.as_secs_f64()) * 100.0 } else { 0.0 };
        println!("\u{2502} Time:   {:02}:{:02} / {:02}:{:02} ({:.0}%) \u{2502}",
            pos.as_secs() / 60, pos.as_secs() % 60,
            dur.as_secs() / 60, dur.as_secs() % 60,
            pct);
    }
    println!("\u{2502} Speed:  {speed:.2}x                        \u{2502}");
    println!("\u{2502} Pitch:  {pitch}                            \u{2502}");

    let pl = player.playlist_mut();
    if let Some(track) = pl.current_track() {
        let fmt = player.display_format();
        println!("\u{251c}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");
        println!("\u{2502} {}", track.display_name(&fmt));
        if !track.artist.is_empty() || !track.album.is_empty() {
            println!("\u{2502} {} - {}", track.artist, track.album);
        }
        if let Some(idx) = pl.current_index() {
            println!("\u{2502} Track {}/{}", idx + 1, pl.len());
        }
    }
    let repeat = pl.repeat_mode();
    println!("\u{2502} Repeat: {}                  \u{2502}", repeat.description());
    println!("\u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}");

    Ok(())
}

pub fn cmd_nowplaying(args: &NowplayingArgs) -> Result<()> {
    use crate::color::{GREEN, YELLOW, DIM, colorize, BOLD, CYAN, BRIGHT_GREEN, BRIGHT_BLUE, BRIGHT_WHITE};
    let player = get_player();

    let state = match player.state() {
        EngineState::Playing => "playing",
        EngineState::Paused => "paused",
        EngineState::Stopped => "stopped",
    };

    let pl = player.playlist_mut();
    let track = pl.current_track();

    if args.progress {
        // Show a compact one-line colored progress bar
        let pos = player.position();
        let dur = player.duration();
        let pct = if dur > Duration::ZERO {
            (pos.as_secs_f64() / dur.as_secs_f64() * 100.0).min(100.0)
        } else {
            0.0
        };

        if let Some(track) = track {
            let fmt = player.display_format();
            let name = track.display_name(&fmt);
            // Truncate song name to ~30 chars so the bar fits on one line
            let name = if name.len() > 30 {
                format!("{}…", &name[..29])
            } else {
                format!("{name:30}")
            };
            let icon = match player.state() {
                EngineState::Playing => "\u{25b6}",   // ▶
                EngineState::Paused => "\u{23f8}",    // ⏸
                EngineState::Stopped => "\u{23f9}",   // ⏹
            };

            // Build the bar
            let bar_width = 24usize;
            let filled = if dur > Duration::ZERO {
                ((pos.as_secs_f64() / dur.as_secs_f64()) * bar_width as f64).round() as usize
            } else {
                0
            };
            let filled = filled.min(bar_width);
            let empty = bar_width - filled;
            let pct_str = format!("{pct:.0}%");
            let pos_str = format!("{:02}:{:02}", pos.as_secs() / 60, pos.as_secs() % 60);
            let dur_str = format!("{:02}:{:02}", dur.as_secs() / 60, dur.as_secs() % 60);

            use std::iter::repeat_n;
            let bar_filled: String = repeat_n('█', filled).collect();
            let bar_empty: String = repeat_n('░', empty).collect();
            // Green bar when playing, yellow when paused, gray when stopped
            let bar_color = match player.state() {
                EngineState::Playing => GREEN,
                EngineState::Paused => YELLOW,
                EngineState::Stopped => DIM,
            };

            println!(
                "{} {} {}{}{}{} {} {}/{}",
                colorize(icon, BOLD),
                colorize(&name, CYAN),
                colorize("[", DIM),
                colorize(&bar_filled, bar_color),
                colorize(&bar_empty, DIM),
                colorize("]", DIM),
                colorize(&pct_str, match pct {
                    _ if pct >= 90.0 => BRIGHT_GREEN,
                    _ if pct >= 60.0 => GREEN,
                    _ if pct >= 30.0 => YELLOW,
                    _ => BRIGHT_BLUE,
                }),
                colorize(&pos_str, BRIGHT_WHITE),
                colorize(&dur_str, DIM),
            );
        } else {
            println!("{} No track loaded", colorize("\u{23f9}", DIM));
        }
        return Ok(());
    }

    if args.json {
        let json = serde_json::json!({
            "status": state,
            "title": track.map_or(&String::new(), |t| &t.title),
            "artist": track.map_or(&String::new(), |t| &t.artist),
            "album": track.map_or(&String::new(), |t| &t.album),
            "position": player.position().as_secs(),
            "length": player.duration().as_secs(),
            "volume": player.volume(),
            "speed": player.speed(),
            "pitch": player.pitch(),
            "repeat": pl.repeat_mode().to_str(),
            "playlist_index": pl.current_index(),
            "playlist_count": pl.len(),
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else if let Some(track) = track {
        let fmt = player.display_format();
        let pos = player.position();
        let dur = player.duration();
        println!("\u{25b6} {}", track.display_name(&fmt));
        if !track.artist.is_empty() || !track.album.is_empty() {
            println!("   {} - {}", track.artist, track.album);
        }
        println!("   {:02}:{:02} / {:02}:{:02}",
            pos.as_secs() / 60, pos.as_secs() % 60,
            dur.as_secs() / 60, dur.as_secs() % 60);
    } else {
        println!("{}", serde_json::json!({ "status": state }));
    }

    Ok(())
}

pub fn cmd_info(args: &InfoArgs) -> Result<()> {
    match &args.action {
        InfoAction::Version => {
            println!("HackMagic Music Player v{}", env!("CARGO_PKG_VERSION"));
            if let Ok(exe) = std::env::current_exe() {
                println!("Executable: {}", exe.display());
            }
        }
        InfoAction::Stats => {
            println!("--- Playback Statistics ---");
            let player = get_player();
            println!("State:    {:?}", player.state());
            println!("Volume:   {}%", player.volume());
            println!("Speed:    {:.2}x", player.speed());
            println!("Pitch:    {}", player.pitch());
            let total = player.playlist_mut().len();
            let pos = player.position();
            let dur = player.duration();
            println!("Tracks:   {total} in playlist");
            if dur > Duration::ZERO {
                println!("Position: {:02}:{:02} / {:02}:{:02}",
                    pos.as_secs() / 60, pos.as_secs() % 60,
                    dur.as_secs() / 60, dur.as_secs() % 60);
            }
        }
        InfoAction::CheckUpdate { download } => {
            check_for_update(*download)?;
        }
        InfoAction::Formats => {
            let exts = crate::audio_common::supported_extensions();
            println!("Supported audio formats ({}):", exts.len());
            for ext in exts {
                print!(" .{ext}");
            }
            println!();
        }
    }
    Ok(())
}

pub fn cmd_config(args: &ConfigArgs) -> Result<()> {
    match &args.action {
        ConfigAction::Get(v) => {
            if let Some(val) = Config::get(&v.key) {
                println!("{} = {}", v.key, val);
            } else {
                println!("Config key '{}' not found", v.key);
            }
        }
        ConfigAction::Set(v) => {
            Config::set(&v.key, &v.value)?;
            println!("{} = {}", v.key, v.value);
        }
        ConfigAction::List => {
            let cfg = Config::load();
            let content = toml::to_string_pretty(&cfg).unwrap_or_default();
            print!("{content}");
        }
        ConfigAction::Import(v) => {
            let content = std::fs::read_to_string(&v.path)?;
            let cfg: Config = toml::from_str(&content)
                .map_err(|e| PlayerError::Config(e.to_string()))?;
            cfg.save()?;
            println!("Config imported from {}", v.path);
        }
        ConfigAction::Export(v) => {
            let cfg = Config::load();
            let content = toml::to_string_pretty(&cfg)
                .map_err(|e| PlayerError::Config(e.to_string()))?;
            std::fs::write(&v.path, content)?;
            println!("Config exported to {}", v.path);
        }
        ConfigAction::Reset => {
            let cfg = Config::default();
            cfg.save()?;
            println!("Config reset to defaults");
        }
    }
    Ok(())
}

pub fn cmd_cue(args: &CueArgs) -> Result<()> {
    use crate::cuesheet::parse_cue_file;

    let sheet = parse_cue_file(&args.path)
        .map_err(PlayerError::Other)?;

    println!("CUE Sheet: {}", args.path);
    println!("Album: {}", sheet.album);
    println!("Artist: {}", sheet.album_artist);
    if !sheet.genre.is_empty() { println!("Genre: {}", sheet.genre); }
    if !sheet.year.is_empty() { println!("Year: {}", sheet.year); }
    if !sheet.comment.is_empty() { println!("Comment: {}", sheet.comment); }
    println!("Tracks: {}", sheet.tracks.len());

    if args.verbose {
        for track in &sheet.tracks {
            let duration_secs = if track.end_pos == Duration::ZERO {
                "-".to_string()
            } else {
                let d = track.end_pos.as_secs_f64() - track.start_pos.as_secs_f64();
                format!("{d:.0}s")
            };
            println!("  {:02}. {} - {} ({})", track.track, track.artist, track.title, duration_secs);
        }
    } else {
        for track in &sheet.tracks {
            println!("  {:02}. {} - {}", track.track, track.artist, track.title);
        }
    }

    Ok(())
}

pub fn cmd_daemon(args: &DaemonArgs) -> Result<()> {
    match &args.action {
        DaemonAction::Start => {
            println!("Daemon mode starting... (Ctrl+C to stop)");
            let player = get_player();

            // Start RPC server in background thread
            let rpc_player = player.clone();
            std::thread::spawn(move || {
                crate::rpc::start_rpc_server(rpc_player, 10280);
            });
            println!("  Web UI: http://127.0.0.1:{}", 10280u16);
            // Open browser (best-effort, cross-platform)
            let webui_url = format!("http://127.0.0.1:{}", 10280u16);
            #[cfg(windows)]
            let _ = std::process::Command::new("cmd")
                .args(["/c", "start", "", &webui_url])
                .spawn();
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open")
                .arg(&webui_url)
                .spawn();
            #[cfg(not(any(windows, target_os = "macos")))]
            let _ = std::process::Command::new("xdg-open")
                .arg(&webui_url)
                .spawn();

            // Start global hotkey listener
            let hotkey_player = player.clone();
            crate::hotkey::init(hotkey_player);
            println!("  Global hotkeys active (Ctrl+Alt+P/N/B/S, multimedia keys)");

            // SMTC initialization (Windows only, best-effort)
            #[cfg(target_os = "windows")]
            let smtc: Option<crate::smtc::SmtcManager> = match crate::smtc::SmtcManager::new() {
                Ok(s) => { println!("  SMTC active"); Some(s) }
                Err(e) => { tracing::debug!("SMTC skipped: {}", e); None }
            };
            #[cfg(not(target_os = "windows"))]
            let smtc: Option<crate::smtc::SmtcManager> = None;

            // Tray icon initialization (Windows only, best-effort)
            #[cfg(target_os = "windows")]
            let mut tray: Option<crate::tray::TrayManager> = match crate::tray::TrayManager::new() {
                Ok(t) => { println!("  System tray icon active"); Some(t) }
                Err(e) => { tracing::debug!("Tray icon skipped: {}", e); None }
            };
            #[cfg(not(target_os = "windows"))]
            let mut tray: Option<crate::tray::TrayManager> = None;

            let mut last_save = std::time::Instant::now();
            let mut last_stats = std::time::Instant::now();
            let mut last_scrobble_check = std::time::Instant::now();
            let mut last_device_check = std::time::Instant::now();
            let mut last_smtc_update = std::time::Instant::now();
            let mut scrobbled_tracks: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut prev_path: Option<String> = None;
            let mut prev_pos_secs: f64 = 0.0;
            let mut prev_tray_state: Option<String> = None;
            loop {
                std::thread::sleep(Duration::from_millis(200));
                let state = player.state();
                if state == EngineState::Stopped {
                    if player.playlist_mut().is_empty() {
                        std::thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                    if let Err(e) = player.next() {
                        tracing::error!("Auto-next failed: {}", e);
                        std::thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                    crate::play_stats::track_started(
                        player.playlist_mut().current_track().map(|t| &*t.file_path)
                    );
                    prev_path = player.playlist_mut().current_track().map(|t| t.file_path.clone());
                    prev_pos_secs = 0.0;
                    player.save_playback_state();
                }

                #[cfg(windows)]
                // Check device state every 2 seconds (BASS-only, Windows)
                if last_device_check.elapsed().as_secs() >= 2 {
                    last_device_check = std::time::Instant::now();
                    if state != EngineState::Stopped {
                        let dev = crate::bass::sys::BASS_GetDevice();
                        let mut info = crate::bass::sys::BASS_DEVICEINFO {
                            name: std::ptr::null(),
                            driver: std::ptr::null(),
                            flags: 0,
                        };
                        if crate::bass::sys::BASS_GetDeviceInfo(dev, &raw mut info)
                            && info.flags & crate::bass::sys::BASS_DEVICE_ENABLED == 0 {
                                tracing::warn!("Audio device removed, stopping playback");
                                player.save_playback_state();
                                let _ = player.stop();
                                continue;
                            }
                    }
                }

                if last_stats.elapsed().as_secs() >= 1 && state == EngineState::Playing {
                    let current_pos = player.position().as_secs_f64();
                    let current_path = player.playlist_mut().current_track().map(|t| t.file_path.clone());
                    if current_path.is_some() && current_path == prev_path {
                        let delta = current_pos - prev_pos_secs;
                        if delta > 0.0 && delta < 5.0 {
                            crate::play_stats::record_playback(
                                current_path.as_deref(),
                                delta.round() as u64,
                            );
                        }
                    }
                    prev_path = current_path;
                    prev_pos_secs = current_pos;
                    last_stats = std::time::Instant::now();
                }

                if last_save.elapsed().as_secs() >= 5 {
                    player.save_playback_state();
                    crate::play_stats::save_stats();
                    last_save = std::time::Instant::now();
                }

                // Update SMTC every 1 second
                if last_smtc_update.elapsed().as_secs_f64() >= 1.0 {
                    let _ = smtc.as_ref().map(|s| s.update(player));
                    last_smtc_update = std::time::Instant::now();
                }

                // Handle SMTC button press events (multimedia keys)
                #[cfg(target_os = "windows")]
                if let Some(ref smtc) = smtc {
                    while let Some(event) = smtc.poll_event() {
                        match event {
                            crate::smtc::SmtcButtonEvent::Play => {
                                if player.is_paused() {
                                    let _ = player.toggle_pause();
                                }
                            }
                            crate::smtc::SmtcButtonEvent::Pause => {
                                if player.is_playing() {
                                    let _ = player.toggle_pause();
                                }
                            }
                            crate::smtc::SmtcButtonEvent::Stop => {
                                let _ = player.stop();
                            }
                            crate::smtc::SmtcButtonEvent::Next => {
                                let _ = player.next();
                                player.save_playback_state();
                            }
                            crate::smtc::SmtcButtonEvent::Previous => {
                                let _ = player.prev();
                                player.save_playback_state();
                            }
                        }
                    }
                }

                // Handle tray icon events (Windows only)
                #[cfg(target_os = "windows")]
                if let Some(ref mut tray) = tray {
                    // Pump Windows messages for tray callbacks
                    tray.poll_messages();
                    // Process tray commands
                    while let Ok(cmd) = tray.try_recv() {
                        match cmd {
                            crate::tray::TrayCommand::TogglePlayPause => {
                                let _ = player.toggle_pause();
                                player.save_playback_state();
                            }
                            crate::tray::TrayCommand::Next => {
                                let _ = player.next();
                                player.save_playback_state();
                            }
                            crate::tray::TrayCommand::Previous => {
                                let _ = player.prev();
                                player.save_playback_state();
                            }
                            crate::tray::TrayCommand::Stop => {
                                let _ = player.stop();
                            }
                            crate::tray::TrayCommand::Exit => {
                                tracing::info!("Tray Exit requested, shutting down");
                                player.save_playback_state();
                                crate::play_stats::save_stats();
                                std::process::exit(0);
                            }
                        }
                    }
                    // Update tray tooltip when playback state changes
                    let tray_state = match player.state() {
                        EngineState::Playing => Some("playing".to_string()),
                        EngineState::Paused => Some("paused".to_string()),
                        EngineState::Stopped => Some("stopped".to_string()),
                    };
                    if tray_state != prev_tray_state {
                        if let Some(ref state_str) = tray_state {
                            match state_str.as_str() {
                                "playing" => tray.set_playing(true),
                                "paused" => tray.set_playing(false),
                                _ => tray.set_stopped(),
                            }
                        }
                        prev_tray_state = tray_state;
                    }
                }

                if last_scrobble_check.elapsed().as_secs() >= 15 {
                    if crate::lastfm::LastfmApi::from_config().is_some() {
                        let pl = player.playlist_mut();
                        if let Some(track) = pl.current_track() {
                            let key = track.file_path.clone();
                            if !scrobbled_tracks.contains(&key) {
                                let pos = player.position().as_secs_f64();
                                let cfg = crate::config::Config::load();
                                if crate::lastfm::LastfmApi::should_scrobble(track, pos, cfg.lastfm.least_perdur, cfg.lastfm.least_dur) {
                                    let ts = chrono::Utc::now().timestamp();
                                    let track_data = track.clone();
                                    std::thread::spawn(move || {
                                        if let Some(api) = crate::lastfm::LastfmApi::from_config() {
                                            let rt = tokio::runtime::Runtime::new().unwrap();
                                            match rt.block_on(api.scrobble(&track_data, ts)) {
                                                Ok(()) => tracing::info!("Auto-scrobbled: {}", track_data.display_name("title")),
                                                Err(e) => tracing::warn!("Auto-scrobble failed: {}", e),
                                            }
                                        }
                                    });
                                    scrobbled_tracks.insert(key);
                                    let track_data2 = track.clone();
                                    std::thread::spawn(move || {
                                        if let Some(api) = crate::lastfm::LastfmApi::from_config() {
                                            let rt = tokio::runtime::Runtime::new().unwrap();
                                            let _ = rt.block_on(api.now_playing(&track_data2));
                                        }
                                    });
                                }
                            }
                        }
                    }
                    last_scrobble_check = std::time::Instant::now();
                }
            }
        }
        DaemonAction::Stop => {
            get_player().stop()?;
            println!("Daemon stopped");
        }
        DaemonAction::Restart => {
            get_player().stop()?;
            get_player().save_playback_state();
            tracing::info!("Daemon restarted (playback stopped)");
        }
        DaemonAction::Status => {
            let state = get_player().state();
            match state {
                EngineState::Playing => println!("Daemon is running and playing"),
                EngineState::Paused => println!("Daemon is running (paused)"),
                EngineState::Stopped => println!("Daemon is running (stopped)"),
            }
        }
    }
    Ok(())
}

/// Check for newer version on GitHub (background, no Result)
pub fn check_update_background() {
    let current_ver = env!("CARGO_PKG_VERSION");
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return,
    };
    let info = rt.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent("hm")
            .timeout(std::time::Duration::from_secs(10))
            .build().ok()?;
        let resp = client
            .get("https://api.github.com/repos/anomalyco/hm/releases/latest")
            .send().await.ok()?;
        let json: serde_json::Value = resp.json().await.ok()?;
        Some(json)
    });

    if let Some(json) = info {
        let tag = json["tag_name"].as_str().unwrap_or("unknown");
        let is_newer = tag.trim_start_matches('v') > current_ver;
        if is_newer {
            tracing::info!(
                "Update available: v{} -> {}. Run 'info check-update' for details.",
                current_ver, tag
            );
        }
    }
}

/// Check for newer version on GitHub, optionally download and install
fn check_for_update(download: bool) -> Result<()> {
    let current_ver = env!("CARGO_PKG_VERSION");
    println!("  Current version: v{current_ver}");

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| PlayerError::Other(format!("Cannot create runtime: {e}")))?;
    let info = rt.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent("hm")
            .timeout(std::time::Duration::from_secs(10))
            .build().ok()?;
        let resp = client
            .get("https://api.github.com/repos/anomalyco/hm/releases/latest")
            .send().await.ok()?;
        let json: serde_json::Value = resp.json().await.ok()?;
        Some(json)
    });

    match info {
        Some(json) => {
            let tag = json["tag_name"].as_str().unwrap_or("unknown");
            let body = json["body"].as_str().unwrap_or("");
            let html_url = json["html_url"].as_str().unwrap_or("");
            let is_newer = tag.trim_start_matches('v') > current_ver;
            if is_newer {
                println!("\n  ╔══════════════════════════════════════╗");
                println!("  ║    Update available!                  ║");
                println!("  ╠══════════════════════════════════════╣");
                println!("  ║  Current: v{current_ver:<23} ║");
                println!("  ║  Latest:  {tag:<23} ║");
                println!("  ╚══════════════════════════════════════╝");
                println!("\n  Release notes:");
                for line in body.lines().take(10) {
                    println!("    {line}");
                }
                println!("\n  Download: {html_url}");

                if download {
                    auto_download_and_replace(&rt, &json)?;
                }
            } else {
                println!("  You have the latest version (v{current_ver})");
            }
        }
        None => {
            println!("  Could not check for updates (no network or GitHub API unavailable).");
        }
    }
    Ok(())
}

/// Find the best matching release asset URL from the GitHub API response for the current platform
fn find_asset_url(json: &serde_json::Value) -> Option<(String, String)> {
    let assets = json["assets"].as_array()?;
    // Determine target triple for the current build
    let target = if cfg!(target_os = "windows") {
        if cfg!(target_arch = "x86_64") {
            "x86_64-pc-windows-msvc"
        } else {
            "i686-pc-windows-msvc"
        }
    } else if cfg!(target_os = "linux") {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(target_os = "macos") {
        "x86_64-apple-darwin"
    } else {
        return None;
    };

    // Look for an asset matching our target triple (zip file)
    for asset in assets {
        let name = asset["name"].as_str().unwrap_or("");
        if name.contains(target) && name.ends_with(".zip") {
            let url = asset["browser_download_url"].as_str()?;
            return Some((url.to_string(), name.to_string()));
        }
    }
    None
}

/// Download the release zip from a URL and extract the executable
fn download_release_zip(rt: &tokio::runtime::Runtime, url: &str, asset_name: &str) -> Result<std::path::PathBuf> {
    let temp_dir = std::env::temp_dir().join("hm_update");
    let _ = std::fs::create_dir_all(&temp_dir);
    let zip_path = temp_dir.join(asset_name);

    println!("  ⬇ Downloading {asset_name}...");

    let result = rt.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent("hm")
            .timeout(std::time::Duration::from_secs(120))
            .build().map_err(|e| PlayerError::Other(format!("Cannot build client: {e}")))?;

        let response = client.get(url)
            .send().await
            .map_err(|e| PlayerError::Other(format!("Download failed: {e}")))?;

        let bytes = response.bytes().await
            .map_err(|e| PlayerError::Other(format!("Download read failed: {e}")))?;

        std::fs::write(&zip_path, &bytes)
            .map_err(|e| PlayerError::Other(format!("Cannot write zip to temp: {e}")))?;

        Ok::<_, PlayerError>(())
    });

    result?;
    println!("  ✅ Downloaded to: {}", zip_path.display());
    Ok(zip_path)
}

/// Auto-download the update and replace the current executable
fn auto_download_and_replace(rt: &tokio::runtime::Runtime, json: &serde_json::Value) -> Result<()> {
    let (asset_url, asset_name) = find_asset_url(json)
        .ok_or_else(|| PlayerError::Other("No matching release asset found for this platform".to_string()))?;

    let zip_path = download_release_zip(rt, &asset_url, &asset_name)?;

    #[cfg(target_os = "windows")]
    {
        replace_exe_windows(&zip_path, &asset_name)?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("  ℹ  Auto-replace not yet implemented for this platform.");
        println!("     The update zip is saved at: {}", zip_path.display());
        println!("     Please extract it manually and replace the executable.");
    }

    Ok(())
}

/// On Windows, extract the exe from the zip and create a batch script to replace and restart
#[cfg(target_os = "windows")]
fn replace_exe_windows(zip_path: &std::path::Path, _asset_name: &str) -> Result<()> {
    // Locate current executable
    let current_exe = std::env::current_exe()
        .map_err(|e| PlayerError::Other(format!("Cannot locate current executable: {e}")))?;
    let exe_dir = current_exe.parent()
        .ok_or_else(|| PlayerError::Other("Cannot determine executable directory".to_string()))?;

    // Extract the zip to a temp directory
    let extract_dir = std::env::temp_dir().join("hm_update_extracted");
    let _ = std::fs::create_dir_all(&extract_dir);

    // Use PowerShell to extract the zip (available on all modern Windows)
    let ps_script = format!(
        r#"Expand-Archive -Path "{}" -DestinationPath "{}" -Force"#,
        zip_path.display(),
        extract_dir.display()
    );
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output()
        .map_err(|e| PlayerError::Other(format!("Failed to run PowerShell: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PlayerError::Other(format!("Zip extraction failed: {stderr}")));
    }

    // The zip contains: hm.exe + bass.dll + bass_fx.dll + gui/
    let new_exe = extract_dir.join("hm.exe");
    if !new_exe.exists() {
        return Err(PlayerError::Other(format!(
            "Extracted exe not found at: {}",
            new_exe.display()
        )));
    }

    // Install path: same directory as current exe
    let install_dir = exe_dir.to_path_buf();

    // Create a batch script that:
    // 1. Waits for current process to exit
    // 2. Copies the new exe and DLLs to the install directory
    // 3. Relaunches the new exe
    let bat_path = std::env::temp_dir().join("hm_update.bat");
    let bat_content = format!(
        r#"@echo off
title hm Updater
echo Waiting for hm to exit...
:wait
ping -n 2 127.0.0.1 > nul
tasklist /FI "IMAGENAME eq hm.exe" 2>nul | find /I "hm.exe" >nul
if not errorlevel 1 goto wait

echo Installing update...
copy /Y "{}" "{}"
copy /Y "{}" "{}"
copy /Y "{}" "{}"

if exist "{}" (
    mkdir "{}" 2>nul
    xcopy /E /Y "{}" "{}"
)

echo Starting hm...
start "" "{}"
del "%~f0"
"#,
        new_exe.display(),
        install_dir.join("hm.exe").display(),
        extract_dir.join("bass.dll").display(),
        install_dir.join("bass.dll").display(),
        extract_dir.join("bass_fx.dll").display(),
        install_dir.join("bass_fx.dll").display(),
        extract_dir.join("gui").display(),
        install_dir.join("gui").display(),
        extract_dir.join("gui").display(),
        install_dir.join("gui").display(),
        install_dir.join("hm.exe").display(),
    );

    std::fs::write(&bat_path, bat_content)
        .map_err(|e| PlayerError::Other(format!("Cannot write update script: {e}")))?;

    println!();
    println!("  ╔══════════════════════════════════════╗");
    println!("  ║    Update ready!                      ║");
    println!("  ╠══════════════════════════════════════╣");
    println!("  ║  The update script will:              ║");
    println!("  ║  1. Wait for this process to exit     ║");
    println!("  ║  2. Copy new executable + DLLs        ║");
    println!("  ║  3. Restart hm                    ║");
    println!("  ╚══════════════════════════════════════╝");
    println!();
    println!("  Launching updater...");

    // Launch the batch script hidden (invisible to user, runs in background)
    let _ = std::process::Command::new("cmd")
        .args(["/c", "start", "/MIN", "", bat_path.to_str().unwrap_or_default()])
        .spawn();

    // Exit current process so the updater can replace the exe
    println!("  Exiting current process to apply update...");
    std::process::exit(0);
}

pub fn cmd_osu(args: &OsuArgs) -> Result<()> {
    match &args.action {
        OsuAction::Info(v) => {
            match crate::osu::parse_osu_file(&v.path) {
                Ok(beatmap) => {
                    println!("OSU! Beatmap Info");
                    println!("{}", "─".repeat(50));
                    println!("  Title:    {}", beatmap.display_name());
                    if !beatmap.title_unicode.is_empty() && beatmap.title_unicode != beatmap.title {
                        println!("  Unicode:  {} - {}", beatmap.artist_unicode, beatmap.title_unicode);
                    }
                    println!("  Creator:  {}", beatmap.creator);
                    println!("  Mode:     {}", match beatmap.mode {
                        0 => "Standard",
                        1 => "Taiko",
                        2 => "Catch the Beat",
                        3 => "Mania",
                        _ => "Unknown",
                    });
                    if beatmap.bpm_min > 0.0 {
                        if beatmap.bpm_min == beatmap.bpm_max {
                            println!("  BPM:      {:.0}", beatmap.bpm_min);
                        } else {
                            println!("  BPM:      {:.0} - {:.0}", beatmap.bpm_min, beatmap.bpm_max);
                        }
                    }
                    if !beatmap.audio_file.is_empty() {
                        println!("  Audio:    {}", beatmap.audio_file);
                    }
                    println!("  HitObjects: {}", beatmap.hit_objects);
                    let audio_dir = std::path::Path::new(&v.path).parent();
                    let audio_path = audio_dir.map(|d| d.join(&beatmap.audio_file));
                    if let Some(ref ap) = audio_path {
                        if ap.exists() {
                            println!("  Audio file found: {}", ap.display());
                        }
                    }
                }
                Err(e) => eprintln!("Error parsing .osu file: {e}"),
            }
        }
        OsuAction::Search(v) => {
            println!("Searching for .osu beatmaps in: {} ...", v.dir);
            match crate::osu::search_beatmaps(&v.dir, v.keyword.as_deref()) {
                Ok(beatmaps) => {
                    if beatmaps.is_empty() {
                        println!("  (no beatmaps found)");
                    } else {
                        println!("Found {} beatmap(s):", beatmaps.len());
                        for (i, b) in beatmaps.iter().enumerate() {
                            let mode_char = match b.mode {
                                0 => 'S', 1 => 'T', 2 => 'C', 3 => 'M', _ => '?',
                            };
                            let bpm_str = if b.bpm_min > 0.0 {
                                if b.bpm_min == b.bpm_max {
                                    format!("{:.0}bpm", b.bpm_min)
                                } else {
                                    format!("{:.0}-{:.0}bpm", b.bpm_min, b.bpm_max)
                                }
                            } else {
                                String::new()
                            };
                            println!("  {:2}. [{}] {}  {}", i + 1, mode_char, b.display_name(), bpm_str);
                        }
                    }
                }
                Err(e) => eprintln!("Error searching beatmaps: {e}"),
            }
        }
    }
    Ok(())
}

pub fn cmd_recent() -> Result<()> {
    let history = crate::config::RecentHistory::load();
    if history.folders.is_empty() && history.playlists.is_empty() {
        println!("No recent history.");
        return Ok(());
    }

    if !history.folders.is_empty() {
        println!("Recently browsed folders:");
        for (i, f) in history.folders.iter().enumerate() {
            println!("  {:2}. {}", i + 1, f);
        }
    }

    if !history.playlists.is_empty() {
        if !history.folders.is_empty() {
            println!();
        }
        println!("Recently loaded playlists:");
        for (i, p) in history.playlists.iter().enumerate() {
            println!("  {:2}. {}", i + 1, p);
        }
    }
    Ok(())
}

pub fn cmd_convert(args: &ConvertArgs) -> Result<()> {
    match &args.action {
        ConvertAction::Simplify { text } => {
            let input = text.join(" ");
            let result = crate::charset::to_simplified_chinese(&input);
            if input == result {
                println!("{input} (already simplified or no conversion needed)");
            } else {
                println!("{result}");
            }
        }
        ConvertAction::Traditionalize { text } => {
            let input = text.join(" ");
            let result = crate::charset::to_traditional_chinese(&input);
            if input == result {
                println!("{input} (already traditional or no conversion needed)");
            } else {
                println!("{result}");
            }
        }
    }
    Ok(())
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m:02}m {s:02}s")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

fn print_bar(value: f64, max_value: f64, width: usize) {
    let filled = if max_value > 0.0 {
        ((value / max_value) * width as f64).round() as usize
    } else {
        0
    };
    let filled = filled.min(width);
    let empty = width - filled;
    let bar: String = std::iter::repeat_n('█', filled)
        .chain(std::iter::repeat_n('░', empty))
        .collect();
    print!("[{bar}]");
}

pub fn cmd_stats(args: &StatsArgs) -> Result<()> {
    match &args.action {
        StatsAction::Show => {
            let total = crate::play_stats::total_listen_secs();
            let play_count = crate::play_stats::total_play_count();
            let track_count = crate::play_stats::total_track_count();
            println!("\n  Playback Statistics");
            println!("  {}", "─".repeat(50));
            println!("  Total listening time: {:>12}", format_duration(total));
            println!("  Total plays:          {play_count:>12}");
            println!("  Unique tracks played: {track_count:>12}");

            let (day_secs, week_secs, month_secs, all) = crate::play_stats::activity_breakdown();
            let max_activity = all.max(1);
            println!("\n  Activity:");
            print!("    Last 24h:  {:>12}  ", format_duration(day_secs));
            print_bar(day_secs as f64, max_activity as f64, 20);
            println!();
            print!("    Last 7d:   {:>12}  ", format_duration(week_secs));
            print_bar(week_secs as f64, max_activity as f64, 20);
            println!();
            print!("    Last 30d:  {:>12}  ", format_duration(month_secs));
            print_bar(month_secs as f64, max_activity as f64, 20);
            println!();
            print!("    All time:  {:>12}  ", format_duration(all));
            print_bar(all as f64, max_activity as f64, 20);
            println!();

            let top = crate::play_stats::top_stats(5);
            if top.is_empty() {
                println!("\n  (no track statistics recorded yet)");
            } else {
                let max_listen = top.first().map_or(1, |(_, e)| e.listen_secs).max(1);
                println!("\n  Top tracks:");
                for (i, (path, entry)) in top.iter().enumerate() {
                    let name = std::path::Path::new(path)
                        .file_stem().map(|s| s.to_string_lossy()).unwrap_or(std::borrow::Cow::Borrowed(path));
                    print!("  {:2}. {:30} ", i + 1, name.chars().take(28).collect::<String>());
                    print_bar(entry.listen_secs as f64, max_listen as f64, 15);
                    println!("  {} ({} plays)", format_duration(entry.listen_secs), entry.play_count);
                }
            }

            // Per-artist stats
            let artists = crate::play_stats::artist_stats(5);
            if !artists.is_empty() {
                let max_artist = artists.first().map_or(1, |(_, s, _)| *s).max(1);
                println!("\n  Top artists:");
                for (i, (name, secs, plays)) in artists.iter().enumerate() {
                    print!("  {:2}. {:30} ", i + 1, name.chars().take(28).collect::<String>());
                    print_bar(*secs as f64, max_artist as f64, 15);
                    println!("  {} ({} plays)", format_duration(*secs), plays);
                }
            }
        }
        StatsAction::Top(args) => {
            let count = args.count.unwrap_or(10);
            let top = crate::play_stats::top_stats(count);
            if top.is_empty() {
                println!("   (no statistics recorded yet)");
            } else {
                let max_listen = top.first().map_or(1, |(_, e)| e.listen_secs).max(1);
                println!("\n  Top {} Most Played Tracks", top.len());
                for (i, (path, entry)) in top.iter().enumerate() {
                    let name = std::path::Path::new(path)
                        .file_stem().map(|s| s.to_string_lossy()).unwrap_or(std::borrow::Cow::Borrowed(path));
                    print!("  {:2}. {:35} ", i + 1, name.chars().take(33).collect::<String>());
                    print_bar(entry.listen_secs as f64, max_listen as f64, 15);
                    println!("  {} ({} plays)", format_duration(entry.listen_secs), entry.play_count);
                }
            }
        }
        StatsAction::Clear => {
            crate::play_stats::clear_stats();
            println!("  All playback statistics cleared");
        }
    }
    Ok(())
}

pub fn cmd_file_assoc(args: &FileAssocArgs) -> Result<()> {
    match &args.action {
        FileAssocAction::Register => {
            register_file_assoc()?;
            println!("\u{2705} File associations registered for all supported audio formats");
        }
        FileAssocAction::Unregister => {
            unregister_file_assoc()?;
            println!("\u{2705} File associations unregistered");
        }
    }
    Ok(())
}

pub fn cmd_open_location(args: &OpenLocationArgs) -> Result<()> {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&args.file_path)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(&args.file_path)
            .spawn();
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let p = std::path::Path::new(&args.file_path);
        let _ = std::process::Command::new("xdg-open")
            .arg(p.parent().unwrap_or(p))
            .spawn();
    }
    Ok(())
}

/// Generate shell completion scripts
pub fn cmd_completion(args: &CompletionArgs) -> Result<()> {
    use clap::CommandFactory;
    use clap_complete::{Generator, Shell};

    let shell: Shell = match args.shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" | "pwsh" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        other => {
            eprintln!("Unsupported shell: {other}");
            eprintln!("Supported shells: bash, zsh, fish, powershell, elvish");
            return Err(PlayerError::Other(format!("Unknown shell: {other}")));
        }
    };

    let cmd = crate::cli::Cli::command();
    let name = cmd.get_name().to_string();

    // Generate completion script to stdout via clap_complete
    shell.generate(&cmd, &mut std::io::stdout());

    // Extra shell-specific hints (stderr so they don't mix with the script)
    match shell {
        Shell::PowerShell => {
            eprintln!("\n# To install, add the above output to your PowerShell profile:");
            eprintln!("#   {name} completion powershell >> $PROFILE");
        }
        Shell::Bash => {
            eprintln!("\n# To install, add to ~/.bashrc:");
            eprintln!("#   source <({name} completion bash)");
        }
        Shell::Zsh => {
            eprintln!("\n# To install, add to ~/.zshrc:");
            eprintln!("#   source <({name} completion zsh)");
        }
        Shell::Fish => {
            eprintln!("\n# To install:");
            eprintln!("#   {name} completion fish > ~/.config/fish/completions/{name}.fish");
        }
        _ => {}
    }

    Ok(())
}

#[cfg(windows)]
fn register_file_assoc() -> Result<()> {
    let exe_path = std::env::current_exe()
        .map_err(|e| PlayerError::Other(format!("Cannot get exe path: {e}")))?;
    let exe_str = exe_path.to_str().unwrap_or_default().to_string();
    let cmd = format!("\"{exe_str}\" \"%1\"");

    let extensions = [
        "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "wma",
        "ape", "dsf", "dff", "mpc", "tta", "wv", "spx", "aiff", "aif",
    ];

    let key_path = "Software\\Classes\\hm.Audio\\shell\\open\\command";
    set_registry(key_path, &cmd)?;

    for ext in &extensions {
        let sub_key = format!("Software\\Classes\\.{ext}");
        set_registry(&sub_key, "hm.Audio")?;
    }

    Ok(())
}

#[cfg(windows)]
fn unregister_file_assoc() -> Result<()> {
    let extensions = [
        "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "wma",
        "ape", "dsf", "dff", "mpc", "tta", "wv", "spx", "aiff", "aif",
    ];

    for ext in &extensions {
        let sub_key = format!("Software\\Classes\\.{ext}");
        delete_registry(&sub_key)?;
    }
    delete_registry("Software\\Classes\\hm.Audio")?;
    Ok(())
}

#[cfg(windows)]
fn set_registry(key: &str, value: &str) -> Result<()> {
    let full_key = format!("HKCU\\{key}");
    let output = std::process::Command::new("reg")
        .args(["add", &full_key, "/ve", "/d", value, "/f"])
        .output()
        .map_err(|e| PlayerError::Other(format!("Cannot access registry: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PlayerError::Other(format!("Registry write failed: {stderr}")));
    }
    Ok(())
}

#[cfg(windows)]
fn delete_registry(key: &str) -> Result<()> {
    let full_key = format!("HKCU\\{key}");
    let output = std::process::Command::new("reg")
        .args(["delete", &full_key, "/f"])
        .output()
        .map_err(|e| PlayerError::Other(format!("Cannot access registry: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("does not exist") && !stderr.contains("not found") {
            return Err(PlayerError::Other(format!("Registry delete failed: {stderr}")));
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn register_file_assoc() -> Result<()> {
    println!("File association is only supported on Windows.");
    println!("On Linux/macOS, use xdg-mime or manually configure your desktop environment.");
    Ok(())
}

#[cfg(not(windows))]
fn unregister_file_assoc() -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::format_duration;

    #[test]
    fn test_format_duration_seconds_only() {
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(1), "1s");
        assert_eq!(format_duration(59), "59s");
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(60), "1m 00s");
        assert_eq!(format_duration(61), "1m 01s");
        assert_eq!(format_duration(3599), "59m 59s");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(3600), "1h 00m 00s");
        assert_eq!(format_duration(3661), "1h 01m 01s");
        assert_eq!(format_duration(86399), "23h 59m 59s");
    }

    #[test]
    fn test_format_duration_days() {
        assert_eq!(format_duration(86400), "24h 00m 00s");
        assert_eq!(format_duration(90061), "25h 01m 01s");
    }

    #[test]
    fn test_format_duration_large_values() {
        assert_eq!(format_duration(u64::MAX), format_duration(u64::MAX));
        // Verify the overflow-safe formula: h = secs / 3600
        let h = u64::MAX / 3600;
        let m = (u64::MAX % 3600) / 60;
        let s = u64::MAX % 60;
        assert_eq!(format_duration(u64::MAX), format!("{}h {:02}m {:02}s", h, m, s));
    }
}
