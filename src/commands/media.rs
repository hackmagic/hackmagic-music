use crate::cli::{MediaArgs, MediaAction};
use crate::error::Result;

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
