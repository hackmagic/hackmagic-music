//! CLI command definitions using clap derive API.

use clap::{Parser, Subcommand, Args};

/// HackMagic Music Player - Cross-platform CLI music player
#[derive(Parser, Debug)]
#[command(name = "hm", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    // === Playback ===

    /// Play audio file(s) or URL
    Play(PlayArgs),

    /// Toggle pause/resume
    Pause,

    /// Stop playback
    Stop,

    /// Next track
    Next,

    /// Previous track
    Prev,

    /// Seek to position
    Seek(SeekArgs),

    // === Volume ===

    /// Volume control
    Volume(VolumeArgs),

    // === Speed ===

    /// Speed control
    Speed(SpeedArgs),

    // === Pitch ===

    /// Pitch control
    Pitch(PitchArgs),

    // === Repeat mode ===

    /// Set repeat mode
    Repeat(RepeatArgs),

    // === Jump ===

    /// Jump to track at index
    Jump(JumpArgs),

    // === Equalizer ===

    /// Equalizer control
    Eq(EqualizerArgs),

    // === Reverb ===

    /// Reverb control
    Reverb(ReverbArgs),

    /// Set AB repeat points
    Ab(ABRepeatArgs),

    // === Playlist ===

    /// Playlist management
    Playlist(PlaylistArgs),

    // === Favourites ===

    /// Favourite tracks management
    Fav(FavArgs),

    // === Lyrics ===

    /// Lyrics operations
    Lyric(LyricArgs),

    // === MusicBrainz ===

    /// Look up metadata from `MusicBrainz`
    Musicbrainz(MusicBrainzArgs),

    // === Cover ===

    /// Album cover operations
    Cover(CoverArgs),

    // === Tags ===

    /// Tag editing
    Tag(TagArgs),

    // === Media Library ===

    /// Media library management
    Media(MediaArgs),

    // === Device ===

    /// Audio device management
    Device(DeviceArgs),

    // === Last.fm ===

    /// Last.fm scrobbling
    Lastfm(LastfmArgs),

    // === MIDI ===

    /// MIDI settings
    Midi(MidiArgs),

    /// BASS plugin management
    Plugin(PluginArgs),

    /// Rate current or specified track (1-5)
    Rate(RateArgs),

    // === Status ===

    /// Show current playback status
    Status,

    /// Show current track info
    Nowplaying(NowplayingArgs),

    /// Show system info
    Info(InfoArgs),

    // === Config ===

    /// Configuration management
    Config(ConfigArgs),

    /// Parse and display CUE sheet info
    Cue(CueArgs),

    // === Daemon ===

    /// Daemon mode (background playback)
    Daemon(DaemonArgs),

    // === Playback Statistics ===

    /// Playback statistics
    Stats(StatsArgs),

    // === Chinese Text Conversion ===

    /// Chinese text conversion (simplified/traditional)
    Convert(ConvertArgs),

    /// OSU! beatmap support
    Osu(OsuArgs),

    /// Show recent folders and playlists
    Recent,

    /// Open file location in system file manager (Explorer/Finder)
    OpenLocation(OpenLocationArgs),

    /// Register/Unregister file association for audio formats
    FileAssoc(FileAssocArgs),

    /// Generate shell completion scripts
    ///
    /// Outputs a shell completion script for the specified shell.
    /// Pipe the output to a file or source it directly.
    ///
    /// Supported shells: bash, zsh, fish, powershell, elvish
    Completion(CompletionArgs),
}

// ===== Play =====
#[derive(Args, Debug)]
pub struct PlayArgs {
    /// File paths or URLs to play
    pub paths: Vec<String>,

    /// Add to playlist instead of replacing
    #[arg(short, long)]
    pub add: bool,

    /// Play from playlist index
    #[arg(short, long)]
    pub index: Option<usize>,

    /// Seek to position (seconds)
    #[arg(long)]
    pub seek: Option<u64>,

    /// Add to "play next" queue
    #[arg(long)]
    pub next: bool,

    /// Show real-time playback progress bar
    #[arg(long)]
    pub progress: bool,
}

// ===== Seek =====
#[derive(Args, Debug)]
pub struct SeekArgs {
    /// Position in seconds (or +/- relative)
    pub position: String,

    /// Relative seek (forward/backward)
    #[arg(short, long)]
    pub relative: bool,
}

// ===== Volume =====
#[derive(Args, Debug)]
pub struct VolumeArgs {
    #[command(subcommand)]
    pub action: VolumeAction,
}

#[derive(Subcommand, Debug)]
pub enum VolumeAction {
    /// Get current volume
    Get,
    /// Set volume (0-100)
    Set(VolumeSetArgs),
    /// Volume up
    Up(VolumeStepArgs),
    /// Volume down
    Down(VolumeStepArgs),
}

#[derive(Args, Debug)]
pub struct VolumeSetArgs { pub value: u32 }

#[derive(Args, Debug)]
pub struct VolumeStepArgs { pub step: Option<u32> }

// ===== Speed =====
#[derive(Args, Debug)]
pub struct SpeedArgs {
    #[command(subcommand)]
    pub action: SpeedAction,
}

#[derive(Subcommand, Debug)]
pub enum SpeedAction {
    Get,
    Set(SpeedSetArgs),
    Up,
    Down,
    Reset,
}

#[derive(Args, Debug)]
pub struct SpeedSetArgs { pub value: f32 }

// ===== Pitch =====
#[derive(Args, Debug)]
pub struct PitchArgs {
    #[command(subcommand)]
    pub action: PitchAction,
}

#[derive(Subcommand, Debug)]
pub enum PitchAction {
    Get,
    Set(PitchSetArgs),
    Up,
    Down,
    Reset,
}

#[derive(Args, Debug)]
pub struct PitchSetArgs { pub value: i32 }

// ===== Repeat =====
#[derive(Args, Debug)]
pub struct RepeatArgs {
    /// Repeat mode: `order|shuffle|random|loop|track|play_track`
    pub mode: Option<String>,
    /// Show current mode
    #[arg(long)]
    pub status: bool,
}

// ===== Jump =====
#[derive(Args, Debug)]
pub struct JumpArgs { pub index: usize }

// ===== Equalizer =====
#[derive(Args, Debug)]
pub struct EqualizerArgs {
    #[command(subcommand)]
    pub action: EqualizerAction,
}

#[derive(Subcommand, Debug)]
pub enum EqualizerAction {
    /// Get all bands or a specific band
    Get(EqualizerGetArgs),
    /// Set band gain
    Set(EqualizerSetArgs),
    /// Apply preset
    Preset(EqualizerPresetArgs),
    /// Enable equalizer
    Enable,
    /// Disable equalizer
    Disable,
    /// Reset all bands
    Reset,
}

#[derive(Args, Debug)]
pub struct EqualizerGetArgs { pub band: Option<usize> }

#[derive(Args, Debug)]
pub struct EqualizerSetArgs { pub band: usize, pub gain: i32 }

#[derive(Args, Debug)]
pub struct EqualizerPresetArgs { pub style: String }

// ===== Reverb =====
#[derive(Args, Debug)]
pub struct ReverbArgs {
    #[command(subcommand)]
    pub action: ReverbAction,
}

#[derive(Subcommand, Debug)]
pub enum ReverbAction {
    Get,
    Mix(ReverbMixArgs),
    Time(ReverbTimeArgs),
    Enable,
    Disable,
}

#[derive(Args, Debug)]
pub struct ReverbMixArgs { pub mix: u32 }

#[derive(Args, Debug)]
pub struct ReverbTimeArgs { pub time: u32 }

// ===== AB Repeat =====
#[derive(Args, Debug)]
pub struct ABRepeatArgs {
    #[command(subcommand)]
    pub action: ABRepeatAction,
}

#[derive(Subcommand, Debug)]
pub enum ABRepeatAction {
    SetA,
    SetB,
    Reset,
    Continue,
    Status,
}

// ===== Playlist =====
#[derive(Args, Debug)]
pub struct PlaylistArgs {
    #[command(subcommand)]
    pub action: PlaylistAction,
}

#[derive(Subcommand, Debug)]
pub enum PlaylistAction {
    /// List all playlists
    List,
    /// Show current playlist
    Show(PlaylistShowArgs),
    /// Create a new playlist
    New(PlaylistNewArgs),
    /// Load a playlist
    Load(PlaylistLoadArgs),
    /// Save playlist to file
    Save(PlaylistSaveArgs),
    /// Add files to playlist
    Add(PlaylistAddArgs),
    /// Remove tracks by indices
    Remove(PlaylistRemoveArgs),
    /// Clear playlist
    Clear,
    /// Sort playlist
    Sort(PlaylistSortArgs),
    /// Move track
    Move(PlaylistMoveArgs),
    /// Shuffle playlist
    Shuffle,
    /// Search tracks
    Search(PlaylistSearchArgs),
    /// Export to file
    Export(PlaylistExportArgs),
    /// Import from file
    Import(PlaylistImportArgs),
    /// Set playlist mode
    Mode(PlaylistModeArgs),
    /// Deduplicate tracks (by file path)
    Dedup,
    /// Remove tracks whose files no longer exist
    Clean,
    /// Rename a playlist
    Rename(PlaylistRenameArgs),
    /// Delete a playlist
    Delete(PlaylistDeleteArgs),
    /// Create playlist from media library category
    FromMedia(PlaylistFromMediaArgs),
    /// Merge same-song different versions into single entries
    MergeVersions,
    /// List/show versions of a track at given index
    Versions(PlaylistVersionsArgs),
}

#[derive(Args, Debug)]
pub struct PlaylistShowArgs {
    /// JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct PlaylistNewArgs { pub name: String }

#[derive(Args, Debug)]
pub struct PlaylistLoadArgs { pub name: String }

#[derive(Args, Debug)]
pub struct PlaylistSaveArgs { pub path: Option<String> }

#[derive(Args, Debug)]
pub struct PlaylistAddArgs { pub files: Vec<String> }

#[derive(Args, Debug)]
pub struct PlaylistRemoveArgs {
    /// One or more indices to remove
    pub indices: Vec<usize>,
}

#[derive(Args, Debug)]
pub struct PlaylistSortArgs {
    pub field: String,
    /// Sort descending
    #[arg(long)]
    pub desc: bool,
}

#[derive(Args, Debug)]
pub struct PlaylistMoveArgs { pub from: usize, pub to: usize }

#[derive(Args, Debug)]
pub struct PlaylistSearchArgs {
    pub keyword: String,
    /// JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct MidiSoundfontArgs {
    pub path: String,
}

// ===== Plugin =====
#[derive(Args, Debug)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub action: PluginAction,
}

#[derive(Subcommand, Debug)]
pub enum PluginAction {
    /// Load a BASS plugin DLL
    Load { path: String },
    /// List loaded plugins
    List,
}

#[derive(Args, Debug)]
pub struct PlaylistExportArgs { pub path: String, #[arg(short, long)] pub format: Option<String> }
#[derive(Args, Debug)]
pub struct PlaylistImportArgs { pub path: String }

#[derive(Args, Debug)]
pub struct PlaylistModeArgs { pub mode: String }

#[derive(Args, Debug)]
pub struct PlaylistRenameArgs {
    pub old_name: String,
    pub new_name: String,
}

#[derive(Args, Debug)]
pub struct PlaylistDeleteArgs {
    pub name: String,
}

// ===== Favourite =====
#[derive(Args, Debug)]
pub struct FavArgs {
    #[command(subcommand)]
    pub action: FavAction,
}

#[derive(Subcommand, Debug)]
pub enum FavAction {
    Add(FavIndexArgs),
    Remove(FavIndexArgs),
    Toggle(FavIndexArgs),
    List,
}

#[derive(Args, Debug)]
pub struct FavIndexArgs { pub index: Option<usize> }

// ===== Lyric =====
#[derive(Args, Debug)]
pub struct LyricArgs {
    #[command(subcommand)]
    pub action: LyricAction,
}

#[derive(Subcommand, Debug)]
pub enum LyricAction {
    /// Show current lyrics
    Show,
    /// Search lyrics
    Search(LyricSearchArgs),
    /// Download lyrics
    Download(LyricDownloadArgs),
    /// Manually link lyric file
    Link(LyricLinkArgs),
    /// Adjust lyric offset
    Offset(LyricOffsetArgs),
    /// Clear lyric association
    Clear,
}

#[derive(Args, Debug)]
pub struct LyricSearchArgs { pub keyword: String }
#[derive(Args, Debug)]
pub struct LyricDownloadArgs { #[arg(long)] pub service: Option<String> }
#[derive(Args, Debug)]
pub struct LyricLinkArgs { pub file: String }
#[derive(Args, Debug)]
pub struct LyricOffsetArgs { pub offset: i64 }

// ===== MusicBrainz =====
#[derive(Args, Debug)]
pub struct MusicBrainzArgs {
    /// Song title or keyword to search
    pub keyword: Option<String>,
    /// Artist name filter
    #[arg(long)]
    pub artist: Option<String>,
    /// Index to apply (0..N), default: first result
    #[arg(long, default_value = "0")]
    pub apply: usize,
    /// Automatically write tags from best match
    #[arg(long)]
    pub auto: bool,
}

// ===== Cover =====
#[derive(Args, Debug)]
pub struct CoverArgs {
    #[command(subcommand)]
    pub action: CoverAction,
}

#[derive(Subcommand, Debug)]
pub enum CoverAction {
    /// Show album cover path
    Show,
    /// Extract cover to file
    Extract { output: Option<String> },
    /// Download album cover
    Download,
    /// Write cover image to audio file
    Write(CoverWriteArgs),
    /// Clear cover cache
    Clear,
}

#[derive(Args, Debug)]
pub struct CoverWriteArgs {
    /// Image file path (jpg/png/bmp)
    pub image: String,
    /// Target audio file (default: current playing track)
    pub file: Option<String>,
}

// ===== Tag =====
#[derive(Args, Debug)]
pub struct TagArgs {
    #[command(subcommand)]
    pub action: TagAction,
}

#[derive(Subcommand, Debug)]
pub enum TagAction {
    /// Show tags for a file
    Show(TagShowArgs),
    /// Set a tag field
    Set(TagSetArgs),
    /// Batch rename files
    Batch(TagBatchArgs),
    /// Convert audio format
    Format(TagFormatArgs),
    /// Parse tags from filename pattern
    FromName(TagFromNameArgs),
    /// Fetch tags from online service (Netease/QQ)
    Online(TagOnlineArgs),
}

#[derive(Args, Debug)]
pub struct TagShowArgs { pub file: String }
#[derive(Args, Debug)]
pub struct TagSetArgs {
    pub file: String,
    pub field: String,
    pub value: String,
}
#[derive(Args, Debug)]
pub struct TagBatchArgs {
    pub dir: String,
    pub pattern: String,
}
#[derive(Args, Debug)]
pub struct TagFormatArgs {
    pub src: String,
    pub dest: String,
    #[arg(short, long)]
    pub format: Option<String>,
    /// Encoding mode: cbr, abr, vbr (default: cbr)
    #[arg(long, default_value = "cbr")]
    pub mode: String,
    /// Bitrate in kbps for CBR/ABR (e.g. 128, 192, 320)
    #[arg(short = 'b', long)]
    pub bitrate: Option<u32>,
    /// Quality for VBR (0=best, 9=worst for mp3/libmp3lame)
    #[arg(short = 'q', long)]
    pub quality: Option<u32>,
    /// Show a real-time conversion progress bar
    #[arg(short, long)]
    pub progress: bool,
}

#[derive(Args, Debug)]
pub struct TagOnlineArgs {
    /// Audio file to update
    pub file: String,
    /// Service to use: netease, qq (default: netease)
    #[arg(short, long, default_value = "netease")]
    pub service: String,
    /// Auto-pick first result (no prompt)
    #[arg(short, long)]
    pub auto: bool,
    /// Also download and embed album cover
    #[arg(short = 'C', long)]
    pub cover: bool,
}

#[derive(Args, Debug)]
pub struct TagFromNameArgs {
    /// Directory containing audio files
    pub dir: String,
    /// Pattern with placeholders: {artist}, {title}, {album}, {track}
    #[arg(default_value = "{artist} - {title}")]
    pub pattern: String,
}

#[derive(Args, Debug)]
pub struct PlaylistVersionsArgs {
    /// Track index in playlist
    pub index: usize,
    /// Switch to this version index (omit to list)
    pub switch_to: Option<usize>,
}

#[derive(Args, Debug)]
pub struct PlaylistFromMediaArgs {
    /// Category type: artist, album, genre, year, rating
    pub category: String,
    /// Value filter
    pub value: String,
    /// Name for the new playlist
    pub name: String,
    /// Load asynchronously with progress display
    #[arg(short, long)]
    pub r#async: bool,
}

// ===== Media Library =====
#[derive(Args, Debug)]
pub struct MediaArgs {
    #[command(subcommand)]
    pub action: MediaAction,
}

#[derive(Subcommand, Debug)]
pub enum MediaAction {
    /// Scan directory into media library
    Scan(MediaScanArgs),
    /// Refresh media library
    Refresh(MediaRefreshArgs),
    /// Show statistics
    Stats,
    /// Search media library
    Search(MediaSearchArgs),
    /// Browse by artist
    Artist(MediaBrowseArgs),
    /// Browse by album
    Album(MediaBrowseArgs),
    /// Browse by genre
    Genre(MediaBrowseArgs),
    /// Browse by year
    Year(MediaBrowseArgs),
    /// Browse by bitrate
    Bitrate(MediaBrowseArgs),
    /// Browse by rating
    Rating(MediaBrowseArgs),
    /// Browse directory structure
    Browse(MediaBrowsePathArgs),
    /// Show all tracks
    All,
    /// Show recent played
    Recent(MediaRecentArgs),
    /// Play from category
    PlayFrom(MediaPlayArgs),
}

#[derive(Args, Debug)]
pub struct MediaScanArgs { pub path: String, #[arg(short, long)] pub recursive: bool }
#[derive(Args, Debug)]
pub struct MediaRefreshArgs { #[arg(long)] pub force: bool }
#[derive(Args, Debug)]
pub struct MediaSearchArgs { pub keyword: String }
#[derive(Args, Debug)]
pub struct MediaBrowseArgs { pub name: Option<String> }
#[derive(Args, Debug)]
pub struct MediaBrowsePathArgs {
    /// Directory path to browse (default: current directory)
    pub path: Option<String>,
    /// Show detailed file info (size, duration)
    #[arg(long)]
    pub details: bool,
    /// Recursively list files in subdirectories
    #[arg(short, long)]
    pub recursive: bool,
}
#[derive(Args, Debug)]
pub struct MediaRecentArgs { pub range: Option<String> }
#[derive(Args, Debug)]
pub struct MediaPlayArgs { pub r#type: String, pub name: String }

// ===== Device =====
#[derive(Args, Debug)]
pub struct DeviceArgs {
    #[command(subcommand)]
    pub action: DeviceAction,
}

#[derive(Subcommand, Debug)]
pub enum DeviceAction {
    List,
    Set(DeviceSetArgs),
    /// List Bluetooth audio devices
    Bluetooth,
}

#[derive(Args, Debug)]
pub struct DeviceSetArgs { pub name_or_index: String }

// ===== Last.fm =====
#[derive(Args, Debug)]
pub struct LastfmArgs {
    #[command(subcommand)]
    pub action: LastfmAction,
}

#[derive(Subcommand, Debug)]
pub enum LastfmAction {
    Status,
    Login(LastfmLoginArgs),
    Love,
    Unlove,
    Scrobble,
}

#[derive(Args, Debug)]
pub struct LastfmLoginArgs { pub username: String, pub password: String }

// ===== MIDI =====
#[derive(Args, Debug)]
pub struct MidiArgs {
    #[command(subcommand)]
    pub action: MidiAction,
}

#[derive(Subcommand, Debug)]
pub enum MidiAction {
    Soundfont(MidiSoundfontArgs),
    Lyric,
}

// ===== Nowplaying =====
#[derive(Args, Debug)]
pub struct NowplayingArgs {
    /// JSON output
    #[arg(long)]
    pub json: bool,
    /// Show a colored progress bar (compact single-line output)
    #[arg(short, long)]
    pub progress: bool,
}

// ===== Info =====
#[derive(Args, Debug)]
pub struct InfoArgs {
    #[command(subcommand)]
    pub action: InfoAction,
}

#[derive(Subcommand, Debug)]
pub enum InfoAction {
    Version,
    Stats,
    /// Check for updates on GitHub
    CheckUpdate {
        /// Download and install the update automatically (Windows only)
        #[arg(short, long)]
        download: bool,
    },
    /// List all supported audio formats
    Formats,
}

// ===== Config =====
#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Get config value
    Get(ConfigGetArgs),
    /// Set config value
    Set(ConfigSetArgs),
    /// List all config
    List,
    /// Import config
    Import(ConfigPathArgs),
    /// Export config
    Export(ConfigPathArgs),
    /// Reset to defaults
    Reset,
}

#[derive(Args, Debug)]
pub struct ConfigGetArgs { pub key: String }
#[derive(Args, Debug)]
pub struct ConfigSetArgs { pub key: String, pub value: String }
#[derive(Args, Debug)]
pub struct ConfigPathArgs { pub path: String }

// ===== OSU! =====
#[derive(Args, Debug)]
pub struct OsuArgs {
    #[command(subcommand)]
    pub action: OsuAction,
}

#[derive(Subcommand, Debug)]
pub enum OsuAction {
    /// Show info about an .osu beatmap file
    Info(OsuInfoArgs),
    /// Search for beatmaps in a directory
    Search(OsuSearchArgs),
}

#[derive(Args, Debug)]
pub struct OsuInfoArgs {
    /// Path to .osu file
    pub path: String,
}

#[derive(Args, Debug)]
pub struct OsuSearchArgs {
    /// Directory containing .osu files
    pub dir: String,
    /// Keyword to filter
    pub keyword: Option<String>,
}

// ===== Rate =====
#[derive(Args, Debug)]
pub struct RateArgs {
    /// Rating (1-5, 0 to clear)
    pub rating: u32,
    /// Track index (default: current)
    pub index: Option<usize>,
}

// ===== Cue =====
#[derive(Debug, Clone, Args)]
pub struct CueArgs {
    /// path to .cue file
    pub path: String,
    /// Verbose output (show all tracks)
    #[arg(short, long)]
    pub verbose: bool,
}

// ===== Daemon =====
#[derive(Args, Debug)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub action: DaemonAction,
}

#[derive(Subcommand, Debug)]
pub enum DaemonAction {
    Start,
    Stop,
    Restart,
    Status,
}

// ===== Stats =====

#[derive(Args, Debug)]
pub struct StatsArgs {
    #[command(subcommand)]
    pub action: StatsAction,
}

#[derive(Subcommand, Debug)]
pub enum StatsAction {
    /// Show playback statistics summary
    Show,
    /// Show top-N most played tracks
    Top(TopArgs),
    /// Clear all playback statistics
    Clear,
}

#[derive(Args, Debug)]
pub struct TopArgs {
    /// Number of top tracks to display (default: 10)
    pub count: Option<usize>,
}

// ===== File Association =====
#[derive(Args, Debug)]
pub struct FileAssocArgs {
    #[command(subcommand)]
    pub action: FileAssocAction,
}

#[derive(Subcommand, Debug)]
pub enum FileAssocAction {
    /// Register file association for supported audio formats
    Register,
    /// Unregister file association for supported audio formats
    Unregister,
}

// ===== Open Location =====
#[derive(Args, Debug)]
pub struct OpenLocationArgs {
    /// File path to reveal in system file manager
    pub file_path: String,
}

// ===== Shell Completion =====
#[derive(Args, Debug)]
pub struct CompletionArgs {
    /// Shell to generate completion for
    ///
    /// Supported: bash, zsh, fish, powershell, elvish
    pub shell: String,
}

// ===== Chinese Convert =====
#[derive(Args, Debug)]
pub struct ConvertArgs {
    #[command(subcommand)]
    pub action: ConvertAction,
}

#[derive(Subcommand, Debug)]
pub enum ConvertAction {
    /// Convert to Simplified Chinese
    Simplify {
        /// Text to convert
        text: Vec<String>,
    },
    /// Convert to Traditional Chinese
    Traditionalize {
        /// Text to convert
        text: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Basic argument parsing ===

    #[test]
    fn test_cli_no_subcommand() {
        // No args should parse successfully (optional subcommand)
        let cli = Cli::try_parse_from(["hm"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_unknown_subcommand() {
        let result = Cli::try_parse_from(["hm", "nonexistent"]);
        assert!(result.is_err());
    }

    // === Playback commands ===

    #[test]
    fn test_play_command_basic() {
        let cli = Cli::try_parse_from(["hm", "play", "song.flac"]).unwrap();
        match cli.command.unwrap() {
            Commands::Play(args) => {
                assert_eq!(args.paths, vec!["song.flac"]);
                assert!(!args.add);
                assert!(args.index.is_none());
            }
            _ => panic!("Expected Play command"),
        }
    }

    #[test]
    fn test_play_command_multiple_paths() {
        let cli = Cli::try_parse_from(["hm", "play", "song1.flac", "song2.mp3"]).unwrap();
        match cli.command.unwrap() {
            Commands::Play(args) => assert_eq!(args.paths, vec!["song1.flac", "song2.mp3"]),
            _ => panic!("Expected Play command"),
        }
    }

    #[test]
    fn test_play_command_with_flags() {
        let cli = Cli::try_parse_from(["hm", "play", "--add", "--index", "3", "--seek", "30", "--next", "file.mp3"]).unwrap();
        match cli.command.unwrap() {
            Commands::Play(args) => {
                assert!(args.add);
                assert_eq!(args.index, Some(3));
                assert_eq!(args.seek, Some(30));
                assert!(args.next);
                assert_eq!(args.paths, vec!["file.mp3"]);
            }
            _ => panic!("Expected Play command"),
        }
    }

    #[test]
    fn test_pause_command() {
        let cli = Cli::try_parse_from(["hm", "pause"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Pause));
    }

    #[test]
    fn test_stop_command() {
        let cli = Cli::try_parse_from(["hm", "stop"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Stop));
    }

    #[test]
    fn test_next_command() {
        let cli = Cli::try_parse_from(["hm", "next"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Next));
    }

    #[test]
    fn test_prev_command() {
        let cli = Cli::try_parse_from(["hm", "prev"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Prev));
    }

    #[test]
    fn test_status_command() {
        let cli = Cli::try_parse_from(["hm", "status"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Status));
    }

    #[test]
    fn test_seek_command_absolute() {
        let cli = Cli::try_parse_from(["hm", "seek", "120"]).unwrap();
        match cli.command.unwrap() {
            Commands::Seek(args) => {
                assert_eq!(args.position, "120");
                assert!(!args.relative);
            }
            _ => panic!("Expected Seek command"),
        }
    }

    #[test]
    fn test_seek_command_relative() {
        let cli = Cli::try_parse_from(["hm", "seek", "--relative", "+10"]).unwrap();
        match cli.command.unwrap() {
            Commands::Seek(args) => {
                assert_eq!(args.position, "+10");
                assert!(args.relative);
            }
            _ => panic!("Expected Seek command"),
        }
    }

    // === Volume commands ===

    #[test]
    fn test_volume_get() {
        let cli = Cli::try_parse_from(["hm", "volume", "get"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Volume(_)));
    }

    #[test]
    fn test_volume_set() {
        let cli = Cli::try_parse_from(["hm", "volume", "set", "75"]).unwrap();
        match cli.command.unwrap() {
            Commands::Volume(args) => match args.action {
                VolumeAction::Set(a) => assert_eq!(a.value, 75),
                _ => panic!("Expected Volume::Set"),
            },
            _ => panic!("Expected Volume command"),
        }
    }

    #[test]
    fn test_volume_up() {
        let cli = Cli::try_parse_from(["hm", "volume", "up"]).unwrap();
        match cli.command.unwrap() {
            Commands::Volume(args) => match args.action {
                VolumeAction::Up(a) => assert!(a.step.is_none()),
                _ => panic!("Expected Volume::Up"),
            },
            _ => panic!("Expected Volume command"),
        }
    }

    #[test]
    fn test_volume_up_with_step() {
        let cli = Cli::try_parse_from(["hm", "volume", "up", "10"]).unwrap();
        match cli.command.unwrap() {
            Commands::Volume(args) => match args.action {
                VolumeAction::Up(a) => assert_eq!(a.step, Some(10)),
                _ => panic!("Expected Volume::Up"),
            },
            _ => panic!("Expected Volume command"),
        }
    }

    #[test]
    fn test_volume_down() {
        let cli = Cli::try_parse_from(["hm", "volume", "down", "5"]).unwrap();
        match cli.command.unwrap() {
            Commands::Volume(args) => match args.action {
                VolumeAction::Down(a) => assert_eq!(a.step, Some(5)),
                _ => panic!("Expected Volume::Down"),
            },
            _ => panic!("Expected Volume command"),
        }
    }

    // === Speed commands ===

    #[test]
    fn test_speed_get() {
        let cli = Cli::try_parse_from(["hm", "speed", "get"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Speed(_)));
    }

    #[test]
    fn test_speed_set() {
        let cli = Cli::try_parse_from(["hm", "speed", "set", "1.5"]).unwrap();
        match cli.command.unwrap() {
            Commands::Speed(args) => match args.action {
                SpeedAction::Set(a) => assert!((a.value - 1.5).abs() < f32::EPSILON),
                _ => panic!("Expected Speed::Set"),
            },
            _ => panic!("Expected Speed command"),
        }
    }

    #[test]
    fn test_speed_reset() {
        let cli = Cli::try_parse_from(["hm", "speed", "reset"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Speed(_)));
    }

    // === Pitch commands ===

    #[test]
    fn test_pitch_set() {
        let cli = Cli::try_parse_from(["hm", "pitch", "set", "5"]).unwrap();
        match cli.command.unwrap() {
            Commands::Pitch(args) => match args.action {
                PitchAction::Set(a) => assert_eq!(a.value, 5),
                _ => panic!("Expected Pitch::Set"),
            },
            _ => panic!("Expected Pitch command"),
        }
    }

    // === Repeat mode ===

    #[test]
    fn test_repeat_mode() {
        let cli = Cli::try_parse_from(["hm", "repeat", "shuffle"]).unwrap();
        match cli.command.unwrap() {
            Commands::Repeat(args) => {
                assert_eq!(args.mode, Some("shuffle".to_string()));
                assert!(!args.status);
            }
            _ => panic!("Expected Repeat command"),
        }
    }

    #[test]
    fn test_repeat_status() {
        let cli = Cli::try_parse_from(["hm", "repeat", "--status"]).unwrap();
        match cli.command.unwrap() {
            Commands::Repeat(args) => {
                assert!(args.mode.is_none());
                assert!(args.status);
            }
            _ => panic!("Expected Repeat command"),
        }
    }

    // === Jump ===

    #[test]
    fn test_jump() {
        let cli = Cli::try_parse_from(["hm", "jump", "5"]).unwrap();
        match cli.command.unwrap() {
            Commands::Jump(args) => assert_eq!(args.index, 5),
            _ => panic!("Expected Jump command"),
        }
    }

    #[test]
    fn test_jump_zero() {
        let cli = Cli::try_parse_from(["hm", "jump", "0"]).unwrap();
        match cli.command.unwrap() {
            Commands::Jump(args) => assert_eq!(args.index, 0),
            _ => panic!("Expected Jump command"),
        }
    }

    // === Equalizer ===

    #[test]
    fn test_eq_get() {
        let cli = Cli::try_parse_from(["hm", "eq", "get"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Eq(_)));
    }

    #[test]
    fn test_eq_get_band() {
        let cli = Cli::try_parse_from(["hm", "eq", "get", "5"]).unwrap();
        match cli.command.unwrap() {
            Commands::Eq(args) => match args.action {
                EqualizerAction::Get(a) => assert_eq!(a.band, Some(5)),
                _ => panic!("Expected Eq::Get"),
            },
            _ => panic!("Expected Eq command"),
        }
    }

    #[test]
    fn test_eq_set() {
        let cli = Cli::try_parse_from(["hm", "eq", "set", "3", "5"]).unwrap();
        match cli.command.unwrap() {
            Commands::Eq(args) => match args.action {
                EqualizerAction::Set(a) => {
                    assert_eq!(a.band, 3);
                    assert_eq!(a.gain, 5);
                }
                _ => panic!("Expected Eq::Set"),
            },
            _ => panic!("Expected Eq command"),
        }
    }

    #[test]
    fn test_eq_enable() {
        let cli = Cli::try_parse_from(["hm", "eq", "enable"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Eq(_)));
    }

    #[test]
    fn test_eq_disable() {
        let cli = Cli::try_parse_from(["hm", "eq", "disable"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Eq(_)));
    }

    #[test]
    fn test_eq_reset() {
        let cli = Cli::try_parse_from(["hm", "eq", "reset"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Eq(_)));
    }

    #[test]
    fn test_eq_preset() {
        let cli = Cli::try_parse_from(["hm", "eq", "preset", "rock"]).unwrap();
        match cli.command.unwrap() {
            Commands::Eq(args) => match args.action {
                EqualizerAction::Preset(a) => assert_eq!(a.style, "rock"),
                _ => panic!("Expected Eq::Preset"),
            },
            _ => panic!("Expected Eq command"),
        }
    }

    // === Reverb ===

    #[test]
    fn test_reverb_get() {
        let cli = Cli::try_parse_from(["hm", "reverb", "get"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Reverb(_)));
    }

    #[test]
    fn test_reverb_mix() {
        let cli = Cli::try_parse_from(["hm", "reverb", "mix", "50"]).unwrap();
        match cli.command.unwrap() {
            Commands::Reverb(args) => match args.action {
                ReverbAction::Mix(a) => assert_eq!(a.mix, 50),
                _ => panic!("Expected Reverb::Mix"),
            },
            _ => panic!("Expected Reverb command"),
        }
    }

    #[test]
    fn test_reverb_time() {
        let cli = Cli::try_parse_from(["hm", "reverb", "time", "200"]).unwrap();
        match cli.command.unwrap() {
            Commands::Reverb(args) => match args.action {
                ReverbAction::Time(a) => assert_eq!(a.time, 200),
                _ => panic!("Expected Reverb::Time"),
            },
            _ => panic!("Expected Reverb command"),
        }
    }

    // === AB Repeat ===

    #[test]
    fn test_ab_repeat_set_a() {
        let cli = Cli::try_parse_from(["hm", "ab", "set-a"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Ab(_)));
    }

    #[test]
    fn test_ab_repeat_set_b() {
        let cli = Cli::try_parse_from(["hm", "ab", "set-b"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Ab(_)));
    }

    #[test]
    fn test_ab_repeat_status() {
        let cli = Cli::try_parse_from(["hm", "ab", "status"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Ab(_)));
    }

    // === Playlist commands ===

    #[test]
    fn test_playlist_list() {
        let cli = Cli::try_parse_from(["hm", "playlist", "list"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_show() {
        let cli = Cli::try_parse_from(["hm", "playlist", "show"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_show_json() {
        let cli = Cli::try_parse_from(["hm", "playlist", "show", "--json"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_new() {
        let cli = Cli::try_parse_from(["hm", "playlist", "new", "my_list"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_load() {
        let cli = Cli::try_parse_from(["hm", "playlist", "load", "favorites"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_add_files() {
        let cli = Cli::try_parse_from(["hm", "playlist", "add", "a.mp3", "b.flac"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_remove() {
        let cli = Cli::try_parse_from(["hm", "playlist", "remove", "1", "3", "5"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_clear() {
        let cli = Cli::try_parse_from(["hm", "playlist", "clear"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_shuffle() {
        let cli = Cli::try_parse_from(["hm", "playlist", "shuffle"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_dedup() {
        let cli = Cli::try_parse_from(["hm", "playlist", "dedup"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_sort() {
        let cli = Cli::try_parse_from(["hm", "playlist", "sort", "title"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_sort_desc() {
        let cli = Cli::try_parse_from(["hm", "playlist", "sort", "artist", "--desc"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_move() {
        let cli = Cli::try_parse_from(["hm", "playlist", "move", "2", "5"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_search() {
        let cli = Cli::try_parse_from(["hm", "playlist", "search", "jazz"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_search_json() {
        let cli = Cli::try_parse_from(["hm", "playlist", "search", "--json", "classical"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_export() {
        let cli = Cli::try_parse_from(["hm", "playlist", "export", "/path/to/file.m3u"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_import() {
        let cli = Cli::try_parse_from(["hm", "playlist", "import", "/path/to/file.m3u8"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_mode() {
        let cli = Cli::try_parse_from(["hm", "playlist", "mode", "loop"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_rename() {
        let cli = Cli::try_parse_from(["hm", "playlist", "rename", "old_name", "new_name"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_delete() {
        let cli = Cli::try_parse_from(["hm", "playlist", "delete", "my_list"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_from_media() {
        let cli = Cli::try_parse_from(["hm", "playlist", "from-media", "artist", "Beatles", "beatles_songs"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_merge_versions() {
        let cli = Cli::try_parse_from(["hm", "playlist", "merge-versions"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_versions() {
        let cli = Cli::try_parse_from(["hm", "playlist", "versions", "2"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_versions_switch() {
        let cli = Cli::try_parse_from(["hm", "playlist", "versions", "2", "0"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    #[test]
    fn test_playlist_clean() {
        let cli = Cli::try_parse_from(["hm", "playlist", "clean"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Playlist(_)));
    }

    // === Favourites ===

    #[test]
    fn test_fav_add() {
        let cli = Cli::try_parse_from(["hm", "fav", "add", "3"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Fav(_)));
    }

    #[test]
    fn test_fav_list() {
        let cli = Cli::try_parse_from(["hm", "fav", "list"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Fav(_)));
    }

    // === Lyrics ===

    #[test]
    fn test_lyric_show() {
        let cli = Cli::try_parse_from(["hm", "lyric", "show"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Lyric(_)));
    }

    #[test]
    fn test_lyric_search() {
        let cli = Cli::try_parse_from(["hm", "lyric", "search", "Hello world"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Lyric(_)));
    }

    #[test]
    fn test_lyric_download() {
        let cli = Cli::try_parse_from(["hm", "lyric", "download"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Lyric(_)));
    }

    #[test]
    fn test_lyric_download_service() {
        let cli = Cli::try_parse_from(["hm", "lyric", "download", "--service", "netease"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Lyric(_)));
    }

    #[test]
    fn test_lyric_link() {
        let cli = Cli::try_parse_from(["hm", "lyric", "link", "/path/to/lyrics.lrc"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Lyric(_)));
    }

    #[test]
    fn test_lyric_offset_positive() {
        let cli = Cli::try_parse_from(["hm", "lyric", "offset", "200"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Lyric(_)));
    }

    #[test]
    fn test_lyric_offset_negative() {
        let cli = Cli::try_parse_from(["hm", "lyric", "offset", "--", "-150"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Lyric(_)));
    }

    #[test]
    fn test_lyric_clear() {
        let cli = Cli::try_parse_from(["hm", "lyric", "clear"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Lyric(_)));
    }

    // === MusicBrainz ===

    #[test]
    fn test_musicbrainz_search() {
        let cli = Cli::try_parse_from(["hm", "musicbrainz", "Bohemian Rhapsody"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Musicbrainz(_)));
    }

    #[test]
    fn test_musicbrainz_with_flags() {
        let cli = Cli::try_parse_from(["hm", "musicbrainz", "--artist", "Queen", "--apply", "1", "--auto", "song"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Musicbrainz(_)));
    }

    // === Cover ===

    #[test]
    fn test_cover_show() {
        let cli = Cli::try_parse_from(["hm", "cover", "show"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Cover(_)));
    }

    #[test]
    fn test_cover_extract_default() {
        let cli = Cli::try_parse_from(["hm", "cover", "extract"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Cover(_)));
    }

    #[test]
    fn test_cover_extract_output() {
        let cli = Cli::try_parse_from(["hm", "cover", "extract", "cover.jpg"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Cover(_)));
    }

    #[test]
    fn test_cover_write() {
        let cli = Cli::try_parse_from(["hm", "cover", "write", "cover.jpg", "song.flac"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Cover(_)));
    }

    #[test]
    fn test_cover_clear() {
        let cli = Cli::try_parse_from(["hm", "cover", "clear"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Cover(_)));
    }

    // === Tag commands ===

    #[test]
    fn test_tag_show() {
        let cli = Cli::try_parse_from(["hm", "tag", "show", "song.flac"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Tag(_)));
    }

    #[test]
    fn test_tag_set() {
        let cli = Cli::try_parse_from(["hm", "tag", "set", "song.flac", "artist", "Test Artist"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Tag(_)));
    }

    #[test]
    fn test_tag_batch() {
        let cli = Cli::try_parse_from(["hm", "tag", "batch", "/music/dir", "{artist} - {title}"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Tag(_)));
    }

    #[test]
    fn test_tag_format() {
        let cli = Cli::try_parse_from(["hm", "tag", "format", "input.flac", "output.mp3"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Tag(_)));
    }

    #[test]
    fn test_tag_format_with_options() {
        let cli = Cli::try_parse_from(["hm", "tag", "format", "src.flac", "dest.mp3", "--format", "mp3", "--bitrate", "320"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Tag(_)));
    }

    #[test]
    fn test_tag_online_default() {
        let cli = Cli::try_parse_from(["hm", "tag", "online", "song.flac"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Tag(_)));
    }

    #[test]
    fn test_tag_online_with_options() {
        let cli = Cli::try_parse_from(["hm", "tag", "online", "song.flac", "--service", "qq", "--auto", "--cover"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Tag(_)));
    }

    #[test]
    fn test_tag_from_name() {
        let cli = Cli::try_parse_from(["hm", "tag", "from-name", "/music/dir"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Tag(_)));
    }

    #[test]
    fn test_tag_from_name_custom_pattern() {
        let cli = Cli::try_parse_from(["hm", "tag", "from-name", "/music/dir", "{track}. {title}"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Tag(_)));
    }

    // === Media library ===

    #[test]
    fn test_media_scan() {
        let cli = Cli::try_parse_from(["hm", "media", "scan", "/music"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Media(_)));
    }

    #[test]
    fn test_media_refresh() {
        let cli = Cli::try_parse_from(["hm", "media", "refresh"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Media(_)));
    }

    #[test]
    fn test_media_stats() {
        let cli = Cli::try_parse_from(["hm", "media", "stats"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Media(_)));
    }

    // === Device ===

    #[test]
    fn test_device_list() {
        let cli = Cli::try_parse_from(["hm", "device", "list"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Device(_)));
    }

    // === Last.fm ===

    #[test]
    fn test_lastfm_login() {
        let cli = Cli::try_parse_from(["hm", "lastfm", "login", "testuser", "testpass"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Lastfm(_)));
    }

    #[test]
    fn test_lastfm_status() {
        let cli = Cli::try_parse_from(["hm", "lastfm", "status"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Lastfm(_)));
    }

    // === Plugin ===

    #[test]
    fn test_plugin_load() {
        let cli = Cli::try_parse_from(["hm", "plugin", "load", "bassopus.dll"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Plugin(_)));
    }

    #[test]
    fn test_plugin_list() {
        let cli = Cli::try_parse_from(["hm", "plugin", "list"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Plugin(_)));
    }

    // === Rate ===

    #[test]
    fn test_rate() {
        let cli = Cli::try_parse_from(["hm", "rate", "4"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Rate(_)));
    }

    // === Nowplaying ===

    #[test]
    fn test_nowplaying_default() {
        let cli = Cli::try_parse_from(["hm", "nowplaying"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Nowplaying(_)));
    }

    #[test]
    fn test_nowplaying_json() {
        let cli = Cli::try_parse_from(["hm", "nowplaying", "--json"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Nowplaying(_)));
    }

    #[test]
    fn test_nowplaying_progress() {
        let cli = Cli::try_parse_from(["hm", "nowplaying", "--progress"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Nowplaying(_)));
    }

    #[test]
    fn test_nowplaying_progress_short() {
        let cli = Cli::try_parse_from(["hm", "nowplaying", "-p"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Nowplaying(_)));
    }

    // === Info ===

    #[test]
    fn test_info_version() {
        let cli = Cli::try_parse_from(["hm", "info", "version"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Info(_)));
    }

    // === Config ===

    #[test]
    fn test_config_list() {
        let cli = Cli::try_parse_from(["hm", "config", "list"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Config(_)));
    }

    #[test]
    fn test_config_get() {
        let cli = Cli::try_parse_from(["hm", "config", "get", "volume"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Config(_)));
    }

    #[test]
    fn test_config_set() {
        let cli = Cli::try_parse_from(["hm", "config", "set", "volume", "80"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Config(_)));
    }

    // === CUE ===

    #[test]
    fn test_cue_parse() {
        let cli = Cli::try_parse_from(["hm", "cue", "/path/to/file.cue"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Cue(_)));
    }

    // === Daemon ===

    #[test]
    fn test_daemon_start() {
        let cli = Cli::try_parse_from(["hm", "daemon", "start"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Daemon(_)));
    }

    #[test]
    fn test_daemon_stop() {
        let cli = Cli::try_parse_from(["hm", "daemon", "stop"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Daemon(_)));
    }

    #[test]
    fn test_daemon_status() {
        let cli = Cli::try_parse_from(["hm", "daemon", "status"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Daemon(_)));
    }

    // === Stats ===

    #[test]
    fn test_stats_show() {
        let cli = Cli::try_parse_from(["hm", "stats", "show"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Stats(_)));
    }

    #[test]
    fn test_stats_top() {
        let cli = Cli::try_parse_from(["hm", "stats", "top", "20"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Stats(_)));
    }

    // === OSU ===

    #[test]
    fn test_osu_info() {
        let cli = Cli::try_parse_from(["hm", "osu", "info", "beatmap.osu"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Osu(_)));
    }

    // === Convert (Chinese) ===

    #[test]
    fn test_convert_simplify() {
        let cli = Cli::try_parse_from(["hm", "convert", "simplify", "繁體字"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Convert(_)));
    }

    #[test]
    fn test_convert_traditionalize() {
        let cli = Cli::try_parse_from(["hm", "convert", "traditionalize", "简体字"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Convert(_)));
    }

    // === Recent ===

    #[test]
    fn test_recent() {
        let cli = Cli::try_parse_from(["hm", "recent"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Recent));
    }

    // === Open Location ===

    #[test]
    fn test_open_location() {
        let cli = Cli::try_parse_from(["hm", "open-location", "/path/to/file.flac"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::OpenLocation(_)));
    }

    // === File Association ===

    #[test]
    fn test_file_assoc_register() {
        let cli = Cli::try_parse_from(["hm", "file-assoc", "register"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::FileAssoc(_)));
    }

    #[test]
    fn test_file_assoc_unregister() {
        let cli = Cli::try_parse_from(["hm", "file-assoc", "unregister"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::FileAssoc(_)));
    }

    // === Short form commands ===

    #[test]
    fn test_short_aliases_play() {
        // Some commands might have short aliases
        let cli = Cli::try_parse_from(["hm", "play", "song.mp3"]);
        assert!(cli.is_ok());
    }

    // === CommandFactory assertion ===

    #[test]
    fn test_ensure_full_command_name_coverage() {
        use clap::CommandFactory;
        // Just ensure no panics when building command metadata
        let cmd = Cli::command();
        // Every subcommand name should exist
        for sub_name in &["play", "pause", "stop", "next", "prev", "seek",
            "volume", "speed", "pitch", "repeat", "jump", "eq", "reverb",
            "ab", "playlist", "fav", "lyric", "musicbrainz", "cover", "tag",
            "media", "device", "lastfm", "midi", "plugin", "rate", "status",
            "nowplaying", "info", "config", "cue", "daemon", "stats",
            "convert", "osu", "recent", "open-location", "file-assoc"]
        {
            let found = cmd.clone().find_subcommand(sub_name).is_some();
            assert!(found, "Subcommand '{}' not found in Cli definition", sub_name);
        }
    }

    #[test]
    fn test_invalid_volume_value_out_of_range() {
        // Volume set 0-100, but clap will accept any u32 - that's a business rule test
        let cli = Cli::try_parse_from(["hm", "volume", "set", "999"]).unwrap();
        assert!(matches!(cli.command.unwrap(), Commands::Volume(_)));
    }
}