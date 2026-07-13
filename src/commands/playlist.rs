use crate::cli::{PlaylistArgs, PlaylistAction};
use crate::commands::get_player;
use crate::error::Result;
use crate::core::playlist::{PlaylistMode, SortMode, Track};

pub fn cmd_playlist(args: &PlaylistArgs) -> Result<()> {
    let player = get_player();
    let (playlist_snapshot, display_fmt) = {
        let pl = player.playlist_mut();
        let fmt = player.display_format();
        (pl.clone(), fmt)
    };

    match &args.action {
        PlaylistAction::List => {
            let active_name = player.active_playlist_name();
            let playlists = player.list_playlists();
            println!("Available playlists ({} total):", playlists.len());
            println!("{}", "-".repeat(50));
            for (name, count) in &playlists {
                let marker = if *name == active_name { "\u{25b6}" } else { " " };
                println!("  {marker} {name:<24} {count} tracks");
            }
            println!("\nActive: {active_name}");
        }
        PlaylistAction::Show(opts) => {
            if opts.json {
                let tracks: Vec<serde_json::Value> = playlist_snapshot.tracks().iter().map(|t| {
                    serde_json::json!({
                        "index": playlist_snapshot.tracks().iter().position(|x| std::ptr::eq(x, t)),
                        "file": t.file_path,
                        "title": t.title,
                        "artist": t.artist,
                        "album": t.album,
                        "duration": t.duration.as_secs(),
                        "sample_rate": t.sample_rate,
                        "bit_depth": t.bit_depth,
                        "channels": t.channels,
                        "bitrate": t.bitrate,
                    })
                }).collect();
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "name": playlist_snapshot.name(),
                    "count": playlist_snapshot.len(),
                    "current_index": playlist_snapshot.current_index(),
                    "tracks": tracks,
                })).unwrap());
            } else {
                println!("Playlist: {} ({} tracks, {})",
                    playlist_snapshot.name(),
                    playlist_snapshot.len(),
                    playlist_snapshot.total_duration_str());
                println!("{}", "-".repeat(60));
                for (i, track) in playlist_snapshot.tracks().iter().enumerate() {
                    let cur = playlist_snapshot.current_index() == Some(i);
                    let marker = if cur { "\u{25b6}" } else { " " };
                    println!("{:>4}{} {}", i, marker, track.display_name(&display_fmt));
                }
            }
        }
        PlaylistAction::New(v) => {
            let name = if v.name.is_empty() {
                let existing: std::collections::HashSet<String> = player.list_playlists()
                    .into_iter().map(|(n, _)| n).collect();
                let mut i = 1;
                loop {
                    let candidate = format!("playlist_{i}");
                    if !existing.contains(&candidate) {
                        break candidate;
                    }
                    i += 1;
                }
            } else {
                v.name.clone()
            };
            player.create_playlist(&name)?;
            println!("Created new playlist: '{name}'");
        }
        PlaylistAction::Load(v) => {
            let pl_dir = crate::config::get_config_dir().join("playlists");
            let pl_path = pl_dir.join(format!("{}.playlist", v.name));
            if pl_path.exists() {
                player.switch_playlist(&v.name)?;
                let name = player.active_playlist_name();
                let count = player.playlist_mut().len();
                println!("Switched to playlist '{name}' ({count} tracks)");
            } else {
                let tracks = crate::playlist_format::read_playlist(&v.name)?;
                let mut pl = player.playlist_mut();
                pl.clear();
                for path in tracks {
                    let mut track = crate::tag::reader::read_tags(&path).unwrap_or_else(|_| Track::new(&path));
                    crate::play_stats::backfill_track(&mut track);
                    pl.add_track(track);
                }
                println!("Loaded {} tracks from {}", pl.len(), v.name);
            }
            // Record recent playlist
            let mut recent = crate::config::RecentHistory::load();
            recent.add_playlist(&v.name);
        }
        PlaylistAction::Save(v) => {
            let path = v.path.clone().unwrap_or_else(|| {
                let pl_dir = crate::config::get_config_dir().join("playlists");
                std::fs::create_dir_all(&pl_dir).ok();
                pl_dir.join(format!("{}.playlist", playlist_snapshot.name()))
                    .to_string_lossy().to_string()
            });
            crate::playlist_format::write_playlist(&path, playlist_snapshot.tracks(), None)?;
            println!("Saved to {path}");
        }
        PlaylistAction::Add(v) => {
            let mut pl = player.playlist_mut();
            let mut cue_count = 0;
            for path in &v.files {
                if path.to_lowercase().ends_with(".cue") {
                    if let Ok(sheet) = crate::cuesheet::parse_cue_file(path) {
                        let count = sheet.tracks.len();
                        pl.add_cue_tracks(&sheet);
                        cue_count += 1;
                        println!("  CUE '{path}': {count} tracks added");
                    } else {
                        eprintln!("  Failed to parse CUE file: {path}");
                    }
                } else {
                    let mut track = crate::tag::reader::read_tags(path).unwrap_or_else(|_| Track::new(path));
                    crate::play_stats::backfill_track(&mut track);
                    pl.add_track(track);
                }
            }
            drop(pl);
            if let Err(e) = player.save_current_playlist() {
                tracing::warn!("Failed to save playlist after add: {}", e);
            }
            if cue_count > 0 {
                println!("Added tracks from {cue_count} CUE file(s)");
            } else {
                println!("Added {} tracks", v.files.len());
            }
        }
        PlaylistAction::Remove(v) => {
            let mut pl = player.playlist_mut();
            pl.remove_multiple(v.indices.clone());
            drop(pl);
            if let Err(e) = player.save_current_playlist() {
                tracing::warn!("Failed to save playlist after remove: {}", e);
            }
            println!("Removed {} tracks", v.indices.len());
        }
        PlaylistAction::Dedup => {
            let mut pl = player.playlist_mut();
            let removed = pl.dedup();
            drop(pl);
            if let Err(e) = player.save_current_playlist() {
                tracing::warn!("Failed to save playlist after dedup: {}", e);
            }
            println!("Removed {removed} duplicate track(s)");
        }
        PlaylistAction::Clean => {
            let mut pl = player.playlist_mut();
            let removed = pl.clean();
            drop(pl);
            if let Err(e) = player.save_current_playlist() {
                tracing::warn!("Failed to save playlist after clean: {}", e);
            }
            println!("Removed {removed} invalid track(s)");
        }
        PlaylistAction::Clear => {
            player.playlist_mut().clear();
            if let Err(e) = player.save_current_playlist() {
                tracing::warn!("Failed to save playlist after clear: {}", e);
            }
            println!("Playlist cleared");
        }
        PlaylistAction::Sort(v) => {
            let mode = SortMode::from_str(&v.field);
            let mut pl = player.playlist_mut();
            pl.sort(mode, v.desc);
            drop(pl);
            if let Err(e) = player.save_current_playlist() {
                tracing::warn!("Failed to save playlist after sort: {}", e);
            }
            if v.desc {
                println!("Sorted by {} (descending)", v.field);
            } else {
                println!("Sorted by {}", v.field);
            }
        }
        PlaylistAction::Move(v) => {
            player.playlist_mut().move_track(v.from, v.to)?;
            if let Err(e) = player.save_current_playlist() {
                tracing::warn!("Failed to save playlist after move: {}", e);
            }
        }
        PlaylistAction::Shuffle => {
            let mut pl = player.playlist_mut();
            pl.sort(SortMode::Random, false);
            drop(pl);
            if let Err(e) = player.save_current_playlist() {
                tracing::warn!("Failed to save playlist after shuffle: {}", e);
            }
        }
        PlaylistAction::Search(v) => {
            let results = playlist_snapshot.search(&v.keyword);
            if v.json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "keyword": v.keyword,
                    "results": results.iter().map(|(i, t)| serde_json::json!({
                        "index": i,
                        "title": t.title,
                        "artist": t.artist,
                        "file": t.file_path,
                    })).collect::<Vec<_>>(),
                })).unwrap());
            } else {
                println!("Search '{}': {} results", v.keyword, results.len());
                for (i, t) in &results {
                    println!("  {:<4} {}", i, t.display_name(&display_fmt));
                }
            }
        }
        PlaylistAction::Export(v) => {
            let fmt = v.format.as_deref().map(|f| match f {
                "m3u" => crate::playlist_format::PlaylistFormat::M3u,
                "m3u8" => crate::playlist_format::PlaylistFormat::M3u8,
                _ => crate::playlist_format::PlaylistFormat::Native,
            });
            crate::playlist_format::write_playlist(&v.path, playlist_snapshot.tracks(), fmt)?;
            println!("Exported to {}", v.path);
        }
        PlaylistAction::Import(v) => {
            let tracks = crate::playlist_format::read_playlist(&v.path)?;
            let mut pl = player.playlist_mut();
            for path in tracks {
                let track = crate::tag::reader::read_tags(&path).unwrap_or_else(|_| Track::new(&path));
                pl.add_track(track);
            }
            println!("Imported from {}", v.path);
        }
        PlaylistAction::Mode(v) => {
            let mode = match v.mode.to_lowercase().as_str() {
                "folder" => PlaylistMode::Folder,
                "playlist" => PlaylistMode::Playlist,
                "media-lib" | "medialib" => PlaylistMode::MediaLib,
                _ => {
                    eprintln!("Invalid mode: {}. Use: folder|playlist|media-lib", v.mode);
                    return Ok(());
                }
            };
            player.playlist_mut().set_mode(mode);
            println!("Playlist mode set to {mode:?}");
        }
        PlaylistAction::Rename(v) => {
            player.rename_playlist(&v.old_name, &v.new_name)?;
            println!("Playlist '{}' renamed to '{}'", v.old_name, v.new_name);
        }
        PlaylistAction::Delete(v) => {
            if v.name == player.active_playlist_name() {
                eprintln!("Cannot delete the active playlist. Switch to another first.");
                return Ok(());
            }
            player.delete_playlist(&v.name)?;
            println!("Playlist '{}' deleted", v.name);
        }
        PlaylistAction::FromMedia(v) => {
            let lib = crate::media::MediaLib::load();
            let entry_refs: Vec<&crate::media::LibEntry> = match v.category.to_lowercase().as_str() {
                "artist" => lib.by_artist(Some(&v.value)),
                "album" => lib.by_album(Some(&v.value)),
                "genre" => lib.entries.iter()
                    .filter(|e| e.genre.eq_ignore_ascii_case(&v.value))
                    .collect(),
                "year" => lib.entries.iter()
                    .filter(|e| e.year.to_string() == v.value)
                    .collect(),
                "rating" => {
                    let r: u32 = v.value.parse().unwrap_or(0);
                    lib.entries.iter()
                        .filter(|e| e.is_favourite && r >= 3)
                        .collect()
                }
                _ => {
                    eprintln!("Unknown category: {}. Use: artist, album, genre, year, rating", v.category);
                    return Ok(());
                }
            };
            if entry_refs.is_empty() {
                println!("No tracks found for {} = {}", v.category, v.value);
                return Ok(());
            }

            let total = entry_refs.len();
            let paths: Vec<String> = entry_refs.iter().map(|e| e.file_path.clone()).collect();
            drop(lib);

            if v.r#async {
                // 异步加载：后台逐批读取标签，主线程显示进度
                let (tx, rx) = std::sync::mpsc::channel::<Vec<crate::core::playlist::Track>>();
                let total_paths = paths.len();

                std::thread::spawn(move || {
                    let batch_size = 10;
                    let mut batch = Vec::with_capacity(batch_size);
                    for (i, p) in paths.iter().enumerate() {
                        let mut track = crate::tag::reader::read_tags(p)
                            .unwrap_or_else(|_| crate::core::playlist::Track::new(p));
                        crate::play_stats::backfill_track(&mut track);
                        batch.push(track);
                        if batch.len() >= batch_size || i == total_paths - 1 {
                            let _ = tx.send(batch);
                            batch = Vec::with_capacity(batch_size);
                        }
                    }
                });

                player.create_playlist(&v.name)?;
                let mut loaded = 0;
                while let Ok(batch) = rx.recv() {
                    let count = batch.len();
                    for t in batch {
                        player.playlist_mut().add_track(t);
                    }
                    loaded += count;
                    eprint!("\rLoading tracks... {loaded}/{total}");
                    use std::io::Write;
                    std::io::stderr().flush().ok();
                    if loaded >= total {
                        break;
                    }
                }
                eprintln!();
            } else {
                // 同步加载（原有逻辑）
                player.create_playlist(&v.name)?;
                for p in &paths {
                    let mut track = crate::tag::reader::read_tags(p)
                        .unwrap_or_else(|_| crate::core::playlist::Track::new(p));
                    crate::play_stats::backfill_track(&mut track);
                    player.playlist_mut().add_track(track);
                }
            }

            // 自动合并多版本（配置开启时）
            let cfg = crate::config::Config::load();
            if cfg.play.merge_song_different_versions {
                let mut mv = crate::multi_version::SongMultiVersion::new();
                let mut tracks = player.playlist_mut().tracks().to_vec();
                let removed = mv.merge(&mut tracks);
                if removed > 0 {
                    player.playlist_mut().clear();
                    for t in tracks {
                        player.playlist_mut().add_track(t);
                    }
                    println!("  (merged {removed} duplicate versions)");
                }
            }
            player.save_current_playlist()?;
            println!("Created playlist '{}' with {} tracks from {} = {}", v.name, player.playlist_mut().len(), v.category, v.value);
        }
        PlaylistAction::MergeVersions => {
            use crate::multi_version::SongMultiVersion;
            let mut pl = player.playlist_mut();
            let before = pl.len();
            let mut mv = SongMultiVersion::new();
            let mut tracks = pl.tracks().to_vec();
            let removed = mv.merge(&mut tracks);
            if removed > 0 {
                pl.clear();
                for t in tracks {
                    pl.add_track(t);
                }
                println!("Merged {} duplicate version(s). Playlist: {} tracks", removed, pl.len());
            } else {
                println!("No duplicate versions found ({before} tracks)");
            }
        }
        PlaylistAction::Versions(v) => {
            use crate::multi_version::SongMultiVersion;
            let track = {
                let pl = player.playlist_mut();
                if let Some(t) = pl.get(v.index) { t.clone() } else { eprintln!("Invalid track index: {}", v.index); return Ok(()); }
            };

            let mut mv = SongMultiVersion::new();
            let mut tracks = player.playlist_mut().tracks().to_vec();
            mv.merge(&mut tracks);

            let versions = mv.get_versions(&track).cloned();
            let versions = match versions {
                Some(v) if v.len() > 1 => v,
                _ => {
                    println!("Track #{} '{}' has no other versions", v.index, track.display_name("artist_title"));
                    return Ok(());
                }
            };

            if let Some(switch_to) = v.switch_to {
                if switch_to >= versions.len() {
                    eprintln!("Invalid version index {}. Valid: 0-{}", switch_to, versions.len() - 1);
                    return Ok(());
                }
                // Update the track in the actual playlist
                let mut pl = player.playlist_mut();
                if let Some(current) = pl.get_mut(v.index) {
                    *current = versions[switch_to].clone();
                }
                println!("Switched to version #{}: {} ({} kbps)", switch_to, versions[switch_to].file_path, versions[switch_to].bitrate);
                return Ok(());
            }

            // List all versions
            println!("Versions of '{}':", track.display_name("artist_title"));
            for (i, ver) in versions.iter().enumerate() {
                let dur_str = ver.duration_str();
                let marker = if ver.file_path == track.file_path { " ▶" } else { "  " };
                println!("  {}.{}{} ({} kbps, {})", v.index, i, marker, ver.bitrate, dur_str);
                println!("     {}", ver.file_path);
            }
            println!("Use `playlist versions {} <version>` to switch", v.index);
        }
    }

    Ok(())
}
