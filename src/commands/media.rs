use crate::cli::{MediaArgs, MediaAction, SearchLyricsArgs, SearchOnlineArgs};
use crate::error::Result;

/// Simple CLI i18n helper: returns the ZH string when the *given* config sets a Chinese
/// language. Takes the config as a parameter so behaviour does not depend on a second,
/// implicit `Config::load()` — tests construct their own config and must get matching output.
fn cli_tr(cfg: &crate::config::Config, en: &str, zh: &str) -> String {
    match cfg.general.language.as_str() {
        "zh-CN" | "zh" | "zh_CN" | "chinese" => zh.to_string(),
        _ => en.to_string(),
    }
}

/// Search the local vector index by lyric text similarity.
pub fn cmd_search_lyrics(args: &SearchLyricsArgs) -> Result<()> {
    let db_path = crate::vecdb::default_db_path();
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    crate::vecdb::init_vecdb(&db_path).map_err(|e| {
        crate::error::PlayerError::BassError(format!("Failed to init vecdb: {e}"))
    })?;

    let conn = crate::vecdb::try_global_db().ok_or_else(|| {
        crate::error::PlayerError::BassError("vecdb not initialised".into())
    })?;

    let query_emb = crate::ai::embed_lyrics_text(&args.query).ok_or_else(|| {
        crate::error::PlayerError::BassError(
            "Embedding API unavailable (set OPENAI_API_KEY environment variable)".into(),
        )
    })?;

    let rt = crate::runtime();
    let hits = rt.block_on(async {
        crate::vecdb::search_lyrics(conn, &query_emb, args.limit)
            .await
            .map_err(|e| {
                crate::error::PlayerError::BassError(format!("Lyric search failed: {e}"))
            })
    })?;

    if hits.is_empty() {
        println!("No matching lyrics found.");
        return Ok(());
    }
    println!("Top {} lyric matches for \"{}\":", hits.len(), args.query);
    for (i, hit) in hits.iter().enumerate() {
        let name = if hit.title.is_empty() {
            hit.path.clone()
        } else {
            format!("{} - {}", hit.artist, hit.title)
        };
        let snippet: String = hit
            .lyrics
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(80)
            .collect();
        println!(
            "  {}. {}\n     album: {}\n     path: {}\n     distance: {:.4}\n     lyrics: {}",
            i + 1,
            name,
            hit.album,
            hit.path,
            hit.distance,
            snippet
        );
    }
    Ok(())
}

/// Resolve a source name (id / alias / display name) against the registry.
/// Unknown names return `None`; callers decide the error message.
fn resolve_source(raw: &str) -> Option<crate::source::SourceId> {
    crate::source::SourceId::from_str(raw).or_else(|| {
        crate::source::all_sources()
            .into_iter()
            .find(|s| s.display_name().eq_ignore_ascii_case(raw))
            .map(|s| s.id())
    })
}

/// Choose which sources `cmd_search_online` should hit.
///
/// - With an explicit `--source`: that single source (error if unknown or disabled).
/// - Without: the configured `source.default` when it names an enabled source
///   (the config field previously had no consumer at all); otherwise all enabled
///   sources.
pub fn select_search_targets(
    cfg: &crate::config::Config,
    explicit: Option<&str>,
) -> Result<Vec<Box<dyn crate::source::MusicSource>>> {
    let enabled_ids = cfg.source.enabled.clone();

    // Resolve `--source` against the full registry first: this distinguishes an
    // unknown id from a known-but-disabled one, and makes the documented aliases
    // ("ia"/"archive"/"fma") work — they used to be rejected because only the
    // canonical `to_str()` value was compared.
    if let Some(raw) = explicit {
        let enabled = crate::source::enabled_sources(&enabled_ids);
        let wanted = resolve_source(raw).ok_or_else(|| {
            crate::error::PlayerError::Other(format!(
                "Unknown source '{raw}'. Valid ids: internet_archive (aliases: ia, archive), \
                 free_music_archive (alias: fma)."
            ))
        })?;
        let filtered: Vec<Box<dyn crate::source::MusicSource>> = enabled
            .into_iter()
            .filter(|s| s.id() == wanted)
            .collect();
        if filtered.is_empty() {
            return Err(crate::error::PlayerError::Other(format!(
                "Source '{}' is disabled. Add it to config 'source.enabled', or clear that list to enable all.",
                wanted.to_str()
            )));
        }
        return Ok(filtered);
    }

    // No `--source`: honour the configured default when it names an enabled
    // source, otherwise fall back to searching every enabled source.
    if let Some(default_id) = resolve_source(&cfg.source.default) {
        let matched: Vec<Box<dyn crate::source::MusicSource>> =
            crate::source::enabled_sources(&enabled_ids)
                .into_iter()
                .filter(|s| s.id() == default_id)
                .collect();
        if !matched.is_empty() {
            return Ok(matched);
        }
    }

    let enabled = crate::source::enabled_sources(&enabled_ids);
    if enabled.is_empty() {
        return Err(crate::error::PlayerError::Other(cli_tr(cfg,
            "No online source available. Check config 'source.enabled'.",
            "没有可用的在线音源，请检查配置 'source.enabled'。",
        )));
    }
    Ok(enabled)
}


/// Search online music sources and optionally play the first result.
pub fn cmd_search_online(args: &SearchOnlineArgs) -> Result<()> {
    let cfg = crate::config::Config::load();
    let targets = select_search_targets(&cfg, args.source.as_deref())?;

    println!(
        "{} \"{}\" {} {}{}...",
        cli_tr(&cfg, "Searching", "搜索"),
        args.keyword,
        cli_tr(&cfg, "across", "跨"),
        targets.len(),
        cli_tr(&cfg, " source(s)", "个音源")
    );
    let mut all_tracks: Vec<crate::source::SourceTrack> = Vec::new();
    let mut ok_count = 0usize;
    let mut err_count = 0usize;
    let mut last_error: Option<String> = None;

    let rt = crate::runtime();
    for src in &targets {
        print!("  [{}] ", src.display_name());
        let result = rt.block_on(async { src.search(&args.keyword).await });
        match result {
            Ok(tracks) => {
                ok_count += 1;
                println!(
                    "{}{} ",
                    tracks.len(),
                    cli_tr(&cfg, " results", " 个结果")
                );
                for t in tracks {
                    all_tracks.push(t);
                }
            }
            Err(e) => {
                err_count += 1;
                last_error = Some(e.to_string());
                println!("{}: {}", cli_tr(&cfg, "error", "错误"), e);
            }
        }
    }

    // Deduplicate by stream_url + truncate to limit
    let mut seen = std::collections::HashSet::new();
    all_tracks.retain(|t| {
        let url = t.stream_url.as_deref().unwrap_or("").to_string();
        seen.insert(url)
    });
    all_tracks.truncate(args.limit);

    if all_tracks.is_empty() {
        // If every source errored, exit non-zero so callers/scripts can tell a
        // network failure apart from a genuine "no matches" result. This used to
        // print "No results found." and exit 0 even when nothing was reached.
        if ok_count == 0 && err_count > 0 {
            return Err(crate::error::PlayerError::Other(match last_error {
                Some(e) => format!("All {err_count} online source(s) failed. Last error: {e}"),
                None => format!("All {err_count} online source(s) failed"),
            }));
        }
        println!("{}", cli_tr(&cfg, "No results found.", "未找到结果。"));
        return Ok(());
    }

    println!(
        "{}{} ",
        all_tracks.len(),
        cli_tr(&cfg, " result(s)", " 个结果")
    );
    for (i, track) in all_tracks.iter().enumerate() {
        let dur = if track.duration > 0 {
            format!("{:02}:{:02}", track.duration / 60, track.duration % 60)
        } else {
            "-:--".to_string()
        };
        let playable = track.stream_url.is_some();
        println!(
            "  {}. [{}] {} - {} ({} {})",
            i + 1,
            track.source.to_str(),
            track.title,
            track.artist,
            dur,
            if playable {
                cli_tr(&cfg, "playable", "可播放")
            } else {
                cli_tr(&cfg, "no stream", "无流")
            }
        );
    }

    if args.play {
        if let Some(track) = all_tracks.first() {
            let url = match &track.stream_url {
                Some(u) => u.clone(),
                None => {
                    eprintln!(
                        "{}",
                        cli_tr(&cfg, 
                            "No stream URL for first result, skipping play.",
                            "第一条结果无流地址，跳过播放。",
                        )
                    );
                    return Ok(());
                }
            };
            println!(
                "{}: {}",
                cli_tr(&cfg, "Playing", "正在播放"),
                track.title
            );
            if let Err(e) = crate::commands::get_player().play_file(&url) {
                eprintln!(
                    "{}: {}",
                    cli_tr(&cfg, "Play failed", "播放失败"),
                    e
                );
            }
        }
    }
    Ok(())
}

pub fn cmd_media(args: &MediaArgs) -> Result<()> {
    match &args.action {
        MediaAction::Scan(v) => {
            println!("Scanning: {} (recursive={})", v.path, v.recursive);
            use std::io::Write;
            // Show a live-updating line for each file found
            let count = std::sync::atomic::AtomicUsize::new(0);
            let entries = crate::media::scan_directory(&v.path, v.recursive, Some(&|path, _| {
                let n = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                // Truncate path for display
                let display = if path.len() > 55 {
                    format!("...{}", &path[path.len().saturating_sub(52)..])
                } else {
                    path.to_string()
                };
                print!("\r  [{n:>4}] {display}  ");
                let _ = std::io::stdout().flush();
            }))?;
            // Clear the progress line
            print!("\r{}\r", " ".repeat(80));
            let mut lib = crate::media::MediaLib::load();
            for entry in entries {
                lib.upsert(entry);
            }
            lib.last_scan = chrono::Utc::now().to_rfc3339();
            lib.save()?;
            let stats = lib.stats();
            println!("Scan complete: {} tracks indexed", stats.get("total_tracks").unwrap_or(&0));
        }
        MediaAction::Refresh(v) => {
            println!("Refreshing media library (force={})", v.force);
            let mut lib = crate::media::MediaLib::load();
            let paths: Vec<String> = lib.entries.iter().map(|e| e.file_path.clone()).collect();
            let mut refreshed = 0;
            for path_str in &paths {
                if std::path::Path::new(path_str).exists() {
                    if let Ok(track) = crate::tag::reader::read_tags(path_str) {
                        let entry = crate::media::LibEntry {
                            file_path: path_str.clone(),
                            title: track.title,
                            artist: track.artist,
                            album: track.album,
                            genre: track.genre,
                            track_number: track.track_number,
                            year: track.year,
                            duration_secs: track.duration.as_secs(),
                            bitrate: track.bitrate,
                            is_favourite: false,
                            play_count: 0,
                            last_played: String::new(),
                            song_id_netease: track.song_id_netease,
                            song_id_qq_music: track.song_id_qq_music,
                            rating: 0,
                        };
                        lib.upsert(entry);
                        refreshed += 1;
                    }
                } else if v.force {
                    lib.entries.retain(|e| e.file_path != *path_str);
                }
            }
            lib.last_scan = chrono::Utc::now().to_rfc3339();
            lib.save()?;
            println!("Refreshed {refreshed} tracks");
        }
        MediaAction::Stats => {
            let lib = crate::media::MediaLib::load();
            let stats = lib.stats();
            println!("Media Library Statistics:");
            println!("  Total tracks:    {}", stats.get("total_tracks").unwrap_or(&0));
            println!("  Total artists:   {}", stats.get("total_artists").unwrap_or(&0));
            println!("  Total albums:    {}", stats.get("total_albums").unwrap_or(&0));
            println!("  Total genres:    {}", stats.get("total_genres").unwrap_or(&0));
            let total_secs = stats.get("total_duration_secs").unwrap_or(&0);
            println!("  Total duration:  {:02}:{:02}:{:02}",
                total_secs / 3600, (total_secs % 3600) / 60, total_secs % 60);
            if !lib.last_scan.is_empty() {
                println!("  Last scan:       {}", lib.last_scan);
            }
        }
        MediaAction::Search(v) => {
            let lib = crate::media::MediaLib::load();
            let results = lib.search(&v.keyword);
            println!("Search '{}': {} results", v.keyword, results.len());
            for (i, entry) in results.iter().enumerate() {
                println!("  {:<4} {} - {} [{}]", i, entry.artist, entry.title, entry.album);
            }
        }
        MediaAction::Artist(v) => {
            let lib = crate::media::MediaLib::load();
            if let Some(name) = &v.name {
                let results = lib.by_artist(Some(name));
                println!("Artist: {} ({} tracks)", name, results.len());
                for entry in &results {
                    println!("  {} - {}", entry.title, entry.album);
                }
            } else {
                let artists = lib.artists();
                println!("All artists ({}):", artists.len());
                for a in &artists {
                    let count = lib.by_artist(Some(a)).len();
                    println!("  {a} ({count} tracks)");
                }
            }
        }
        MediaAction::Album(v) => {
            let lib = crate::media::MediaLib::load();
            if let Some(name) = &v.name {
                let results = lib.by_album(Some(name));
                println!("Album: {} ({} tracks)", name, results.len());
                for entry in &results {
                    println!("  {} - {}", entry.artist, entry.title);
                }
            } else {
                let albums = lib.albums();
                println!("All albums ({}):", albums.len());
                for a in &albums {
                    let count = lib.by_album(Some(a)).len();
                    println!("  {a} ({count} tracks)");
                }
            }
        }
        MediaAction::Genre(v) => {
            let lib = crate::media::MediaLib::load();
            if let Some(name) = &v.name {
                println!("Genre: {name}");
            } else {
                let genres = lib.genres();
                println!("All genres ({}):", genres.len());
                for g in &genres { println!("  {g}"); }
            }
        }
        MediaAction::All => {
            let lib = crate::media::MediaLib::load();
            println!("All tracks ({}):", lib.entries.len());
            for (i, entry) in lib.entries.iter().enumerate() {
                println!("  {:<4} {} - {} [{}]", i, entry.artist, entry.title, entry.album);
            }
        }
        MediaAction::Recent(v) => {
            let limit = v.range.as_deref().and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);
            let recent = crate::play_stats::recent_played(limit);
            if recent.is_empty() {
                println!("  (no recent tracks)");
            } else {
                println!("Recently played tracks (last {}):", recent.len());
                for (i, (path, entry)) in recent.iter().enumerate() {
                    let name = std::path::Path::new(path)
                        .file_stem().map(|s| s.to_string_lossy()).unwrap_or(std::borrow::Cow::Borrowed(path));
                    let ts = chrono::DateTime::from_timestamp(entry.last_played as i64, 0).map_or_else(|| "unknown".to_string(), |t| t.format("%Y-%m-%d %H:%M").to_string());
                    println!("  {:2}. {}  (last: {}, plays: {})", i + 1, name, ts, entry.play_count);
                }
            }
        }
        MediaAction::Year(v) => {
            let lib = crate::media::MediaLib::load();
            if let Some(name) = &v.name {
                let results: Vec<&crate::media::LibEntry> = lib.entries.iter()
                    .filter(|e| e.year.to_string() == *name).collect();
                println!("Year: {} ({} tracks)", name, results.len());
                for entry in &results {
                    println!("  {} - {} [{}]", entry.artist, entry.title, entry.album);
                }
            } else {
                let mut years: Vec<u32> = lib.entries.iter().map(|e| e.year).collect();
                years.sort_unstable();
                years.dedup();
                println!("All years ({}):", years.len());
                for y in years {
                    if y > 0 {
                        let count = lib.entries.iter().filter(|e| e.year == y).count();
                        println!("  {y} ({count} tracks)");
                    }
                }
            }
        }
        MediaAction::Bitrate(v) => {
            let lib = crate::media::MediaLib::load();
            if let Some(name) = &v.name {
                let br: u32 = name.parse().unwrap_or(0);
                let results: Vec<&crate::media::LibEntry> = lib.entries.iter()
                    .filter(|e| e.bitrate == br).collect();
                println!("Bitrate: {} kbps ({} tracks)", name, results.len());
                for entry in &results {
                    println!("  {} - {} [{}]", entry.artist, entry.title, entry.album);
                }
            } else {
                let mut brs: Vec<u32> = lib.entries.iter().map(|e| e.bitrate).collect();
                brs.sort_unstable();
                brs.dedup();
                println!("All bitrates ({}):", brs.len());
                for b in brs {
                    if b > 0 {
                        let count = lib.entries.iter().filter(|e| e.bitrate == b).count();
                        println!("  {b} kbps ({count} tracks)");
                    }
                }
            }
        }
        MediaAction::Rating(v) => {
            let lib = crate::media::MediaLib::load();
            if let Some(name) = &v.name {
                let r: u32 = name.parse().unwrap_or(0);
                let results: Vec<&crate::media::LibEntry> = lib.entries.iter()
                    .filter(|e| e.is_favourite && r >= 3).collect();
                println!("Rating: {} stars ({} tracks)", name, results.len());
                for entry in &results {
                    println!("  {} - {} [{}]", entry.artist, entry.title, entry.album);
                }
            } else {
                println!("Favourite tracks ({}):", lib.entries.iter().filter(|e| e.is_favourite).count());
                for entry in lib.entries.iter().filter(|e| e.is_favourite) {
                    println!("  {} - {} [{}]", entry.artist, entry.title, entry.album);
                }
            }
        }
        MediaAction::PlayFrom(v) => {
            println!("Play from {}: {}", v.r#type, v.name);
            eprintln!("Play from library not yet implemented");
        }
        MediaAction::Browse(v) => {
            let dir = v.path.clone().unwrap_or_else(|| ".".to_string());
            let abs_path = std::path::Path::new(&dir);
            let abs_path = if abs_path.is_absolute() {
                dir.clone()
            } else if let Ok(cwd) = std::env::current_dir() {
                cwd.join(abs_path).to_string_lossy().to_string()
            } else {
                dir.clone()
            };
            match crate::media::browse_directory(&abs_path, v.recursive) {
                Ok(entries) => {
                    if entries.is_empty() {
                        println!("  (empty directory)");
                    } else {
                        println!("📁 {abs_path}");
                        println!("{}", "─".repeat(60));
                        for entry in &entries {
                            if entry.is_dir {
                                if v.details {
                                    println!("  📂 {:<30} {:>8} bytes", entry.name, entry.size);
                                } else {
                                    println!("  📂 {}", entry.name);
                                }
                            } else if entry.is_audio {
                                let dur_str = if entry.duration_secs > 0 {
                                    format!("{:02}:{:02}", entry.duration_secs / 60, entry.duration_secs % 60)
                                } else {
                                    "-:--".to_string()
                                };
                                if v.details {
                                    println!("  🎵 {:<30} {:>8} bytes  {}", entry.name, entry.size, dur_str);
                                } else {
                                    println!("  🎵 {}", entry.name);
                                }
                            } else if v.details {
                                println!("     {:<30} {:>8} bytes", entry.name, entry.size);
                            }
                        }
                        let audio_count = entries.iter().filter(|e| e.is_audio).count();
                        let dir_count = entries.iter().filter(|e| e.is_dir).count();
                        println!("{}", "─".repeat(60));
                        println!("  {audio_count} file(s), {dir_count} director(ies)");
                    }
                }
                Err(e) => eprintln!("Error browsing directory: {e}"),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    /// `Result` helpers that don't need `T: Debug` (the source list isn't Debug).
    fn ok_only(r: crate::error::Result<Vec<Box<dyn crate::source::MusicSource>>>) -> Vec<Box<dyn crate::source::MusicSource>> {
        match r {
            Ok(v) => v,
            Err(e) => panic!("expected Ok, got Err: {e}"),
        }
    }

    fn err_only(r: crate::error::Result<Vec<Box<dyn crate::source::MusicSource>>>) -> crate::error::PlayerError {
        match r {
            Ok(v) => panic!("expected Err, got Ok with {} target(s)", v.len()),
            Err(e) => e,
        }
    }

    fn cfg_with(enabled: Vec<&str>, default: &str) -> Config {
        let mut cfg = Config::default();
        cfg.source.enabled = enabled.into_iter().map(String::from).collect();
        cfg.source.default = default.to_string();
        cfg
    }

    fn ids(targets: &[Box<dyn crate::source::MusicSource>]) -> Vec<crate::source::SourceId> {
        targets.iter().map(|s| s.id()).collect()
    }

    #[test]
    fn explicit_source_wins_over_default() {
        let cfg = cfg_with(vec![], "fma");
        // No `source.enabled` filter → all sources enabled; the explicit "ia"
        // request must win even though `source.default` says "fma".
        let t = ok_only(select_search_targets(&cfg, Some("ia")));
        assert_eq!(ids(&t), vec![crate::source::SourceId::InternetArchive]);
    }

    #[test]
    fn explicit_disabled_source_loses_to_default() {
        // "ia" is not enabled, and an explicit request never falls back to the
        // default: it errors instead.
        let cfg = cfg_with(vec!["fma"], "ia");
        let e = err_only(select_search_targets(&cfg, Some("ia")));
        assert!(e.to_string().contains("disabled"), "got: {e}");
    }

    #[test]
    fn explicit_unknown_source_errors() {
        let cfg = cfg_with(vec![], "");
        let e = err_only(select_search_targets(&cfg, Some("bogus")));
        assert!(e.to_string().contains("Unknown source 'bogus'"), "got: {e}");
    }

    #[test]
    fn explicit_disabled_source_errors() {
        let cfg = cfg_with(vec!["fma"], "");
        let e = err_only(select_search_targets(&cfg, Some("ia")));
        assert!(e.to_string().contains("disabled"), "got: {e}");
    }

    #[test]
    fn no_source_uses_configured_default_when_enabled() {
        let cfg = cfg_with(vec!["fma", "ia"], "fma");
        let t = ok_only(select_search_targets(&cfg, None));
        assert_eq!(ids(&t), vec![crate::source::SourceId::FreeMusicArchive]);
    }

    #[test]
    fn no_source_falls_back_to_all_enabled_when_default_unresolvable() {
        let cfg = cfg_with(vec![], "no_such_source");
        let t = ok_only(select_search_targets(&cfg, None));
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn no_source_falls_back_to_all_enabled_when_default_disabled() {
        let cfg = cfg_with(vec!["fma"], "ia");
        // Default "ia" is not in the enabled list → search every enabled source.
        let t = ok_only(select_search_targets(&cfg, None));
        assert_eq!(ids(&t), vec![crate::source::SourceId::FreeMusicArchive]);
    }

    #[test]
    fn no_enabled_sources_errors() {
        let cfg = cfg_with(vec!["bogus"], "");
        let e = err_only(select_search_targets(&cfg, None));
        assert!(e.to_string().contains("No online source"), "got: {e}");
    }

    #[test]
    fn display_name_also_resolves() {
        let cfg = cfg_with(vec![], "");
        let t = ok_only(select_search_targets(&cfg, Some("Internet Archive")));
        assert_eq!(ids(&t), vec![crate::source::SourceId::InternetArchive]);
    }
}
