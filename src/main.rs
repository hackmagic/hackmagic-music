use hm::cli::{Cli, Commands, DaemonArgs, DaemonAction, InfoArgs, InfoAction};
use hm::config::Config;
use hm::core::engine_trait::EngineType;
use hm::core::player::Player;
use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

// These wrappers avoid the `static_mut_refs` warning from Rust 2024.
// The accesses are in `unsafe fn` callers; the `static mut` is a deliberate
// single-threaded choice for lock-at-most-once usage.
#[allow(static_mut_refs)]
mod instance_lock_static {
    use super::InstanceLock;
    static mut LOCK: Option<InstanceLock> = None;

    /// Returns true if the lock has already been acquired.
    pub(crate) fn is_held() -> bool {
        unsafe { LOCK.is_some() }
    }

    /// Store an acquired lock. Call only once, from `check_single_instance`.
    pub(crate) fn set(lock: InstanceLock) {
        unsafe {
            LOCK = Some(lock);
        }
    }
}

/// On all platforms we store the lock file handle / mutex in a static so it
/// lives for the process lifetime, ensuring automatic cleanup on exit.
#[allow(unused)]
static mut INSTANCE_LOCK: Option<InstanceLock> = None;

/// Platform-specific instance lock abstraction.
#[cfg(target_os = "windows")]
struct InstanceLock(());

#[cfg(target_os = "windows")]
impl InstanceLock {
    fn acquire() -> Option<Self> {
        extern "system" {
            fn CreateMutexW(
                lpMutexAttributes: *const std::ffi::c_void,
                bInitialOwner: i32,
                lpName: *const u16,
            ) -> isize;
            fn GetLastError() -> u32;
        }
        const ERROR_ALREADY_EXISTS: u32 = 183;
        let name: Vec<u16> = "HackMagicMusic\0".encode_utf16().collect();
        unsafe {
            let mutex = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
            if mutex == 0 || GetLastError() == ERROR_ALREADY_EXISTS {
                None
            } else {
                Some(Self(()))
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
struct InstanceLock {
    _file: std::fs::File,
    path: std::path::PathBuf,
}

#[cfg(not(target_os = "windows"))]
impl InstanceLock {
    /// Try to acquire the lock. Returns `Some` if we are the sole instance.
    fn acquire() -> Option<Self> {
        let lock_path = std::env::temp_dir().join("hm.pid");
        // Try to atomically create the lock file
        loop {
            match std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&lock_path)
            {
                Ok(file) => {
                    // We got the lock; write our PID
                    let pid_str = format!("{}\n", std::process::id());
                    // Ignore write errors — the file itself is the lock
                    let _ = file.set_len(0);
                    let _ = std::io::Write::write_all(
                        &mut &file,
                        pid_str.as_bytes(),
                    );
                    return Some(InstanceLock {
                        _file: file,
                        path: lock_path,
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    // Lock file exists — check if the owning process is alive
                    if let Ok(pid_str) = std::fs::read_to_string(&lock_path) {
                        if let Ok(pid) = pid_str.trim().parse::<u32>() {
                            // On Linux/macOS, `kill -0` checks if a PID exists
                            // without sending a signal
                            if !is_pid_alive(pid) {
                                // Stale lock file — remove it and retry
                                let _ = std::fs::remove_file(&lock_path);
                                continue;
                            }
                        }
                    }
                    // Another live instance is running
                    return None;
                }
                Err(_) => return None,
            }
        }
    }
}

impl Drop for InstanceLock {
    #[cfg(not(target_os = "windows"))]
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
    #[cfg(target_os = "windows")]
    fn drop(&mut self) {
        // Named mutex is automatically released when the handle closes
    }
}

/// Check if a process with the given PID is still alive (Unix only).
/// On non-Unix platforms always returns false so we treat the lock as stale.
#[cfg(not(target_os = "windows"))]
fn is_pid_alive(pid: u32) -> bool {
    // `kill -0` checks process existence without sending a signal
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Prevent multiple instances. Returns true if this is the only instance.
fn check_single_instance() -> bool {
    if instance_lock_static::is_held() {
        return true;
    }
    InstanceLock::acquire().is_some_and(|lock| {
        instance_lock_static::set(lock);
        true
    })
}

#[allow(clippy::too_many_lines)]
fn main() {
    // Enable ANSI colour support early (Windows needs VT processing)
    hm::color::enable_ansi_support();

    // Log panic to file for diagnosis
    let panic_log = std::env::temp_dir().join("hm_panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("Panic: {info}\n");
        let _ = std::fs::write(&panic_log, &msg);
        eprintln!("{msg}");
    }));

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();

    // No subcommand: auto-start daemon mode
    let command = cli.command.unwrap_or(Commands::Daemon(DaemonArgs {
        action: DaemonAction::Start,
    }));

    // Early commands that don't need BASS initialization
    if matches!(command, Commands::Info(InfoArgs { action: InfoAction::Version })) {
        println!("HackMagic Music Player v{}", env!("CARGO_PKG_VERSION"));
        if let Ok(exe) = std::env::current_exe() {
            println!("Executable: {}", exe.display());
        }
        return;
    }
    if let Commands::Cue(ref args) = command {
        if let Err(e) = hm::commands::system::cmd_cue(args) {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
        return;
    }

    // Single instance check (skip for info/version/cue/daemon)
    let skip_single = matches!(command, Commands::Info(InfoArgs { action: InfoAction::Version }))
        || matches!(command, Commands::Cue(_))
        || matches!(command, Commands::Daemon(_));
    if !skip_single && !check_single_instance() {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if hm::hotkey::try_forward_command(&args) {
            std::process::exit(0);
        }
        eprintln!("Error: Another instance is already running (command forwarding failed)");
        std::process::exit(1);
    }

    // Load config
    let cfg = Config::load();

    // Auto-check for updates (non-blocking, in background thread)
    if cfg.general.check_update_when_start {
        std::thread::spawn(|| {
            use hm::commands::system::check_update_background;
            check_update_background();
        });
    }

    // Auto-scan media library at startup (only if empty)
    if cfg.media_lib.auto_scan && !cfg.media_lib.media_dirs.is_empty() {
        let existing = hm::media::MediaLib::load();
        if existing.entries.is_empty() {
            let dirs = cfg.media_lib.media_dirs.clone();
            std::thread::spawn(move || {
                for dir in &dirs {
                    tracing::info!("Auto-scanning media directory: {}", dir);
                    match hm::media::scan_directory(dir, true, None) {
                        Ok(entries) => {
                            let mut lib = hm::media::MediaLib::load();
                            let before = lib.entries.len();
                            for e in entries {
                                lib.upsert(e);
                            }
                            let added = lib.entries.len() - before;
                            if added > 0 || before > 0 {
                                let _ = lib.save();
                                tracing::info!("Media library updated: {} tracks ({} new)", lib.entries.len(), added);
                            }
                        }
                        Err(e) => tracing::warn!("Auto-scan failed for {}: {}", dir, e),
                    }
                }
            });
        } else {
            tracing::info!("Skipping auto-scan: library already has {} entries", existing.entries.len());
        }
    }

    // Create player
    let engine_type = EngineType::from_str(&cfg.play.engine);
    let player = Arc::new(Player::new(engine_type));

    player.set_volume(cfg.play.default_volume).ok();
    player.set_volume_map(cfg.play.volume_map);
    player.set_spectrum_config(cfg.appearance.spectrum_columns as usize, cfg.appearance.fft_size as usize);

    hm::commands::init_player(player.clone());

    // Initialize engine — fallback to FFmpeg if BASS fails
    if let Err(e) = player.init() {
        if cfg.play.engine == "bass" {
            tracing::warn!("BASS init failed ({}), trying FFmpeg engine", e);
            let engine_type = EngineType::Ffmpeg;
            let fallback_player = Arc::new(Player::new(engine_type));
            fallback_player.set_volume(cfg.play.default_volume).ok();
            fallback_player.set_volume_map(cfg.play.volume_map);
            fallback_player.set_spectrum_config(cfg.appearance.spectrum_columns as usize, cfg.appearance.fft_size as usize);
            hm::commands::init_player(fallback_player.clone());
            match fallback_player.init() {
                Ok(()) => {
                    tracing::info!("Fell back to FFmpeg engine successfully");
                    // Update config so subsequent runs try FFmpeg first
                    let mut new_cfg = cfg.clone();
                    new_cfg.play.engine = "ffmpeg".to_string();
                    if let Err(e) = new_cfg.save() {
                        tracing::warn!("Failed to save engine fallback config: {e}");
                    }
                    let _ = fallback_player;
                }
                Err(e2) => {
                    eprintln!("Error: Failed to initialize player: {e2}");
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("Error: Failed to initialize player: {e}");
            std::process::exit(1);
        }
    }

    // Restore last playlist (tracks) without starting playback
    let is_daemon = matches!(command, Commands::Daemon(_));
    if is_daemon {
        use hm::config::PlaybackState;
        let state = PlaybackState::load();
        let pl_dir = hm::config::get_config_dir().join("playlists");
        let pl_path = pl_dir.join(format!("{}.playlist", state.last_playlist));
        if pl_path.exists() {
            if let Err(e) = player.switch_playlist(&state.last_playlist) {
                tracing::warn!("Failed to restore playlist '{}': {}", state.last_playlist, e);
            } else {
                tracing::info!("Restored playlist '{}'", state.last_playlist);
            }
            player.set_volume(state.volume).ok();
            let mode = hm::core::playlist::RepeatMode::from_str(&state.repeat_mode);
            player.set_repeat_mode(mode);
        }
    }

    // Execute command via dispatcher
    if let Err(e) = hm::commands::dispatch(&command) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}