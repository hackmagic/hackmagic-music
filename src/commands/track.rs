use crate::cli::{FavArgs, FavAction, RateArgs, LyricArgs, LyricAction, MusicBrainzArgs, CoverArgs, CoverAction, TagArgs, TagAction};
use crate::commands::get_player;
use crate::error::{PlayerError, Result};

pub fn cmd_fav(args: &FavArgs) -> Result<()> {
    let player = get_player();
    match &args.action {
        FavAction::Add(v) => {
            let idx = v.index.unwrap_or_else(|| player.playlist_mut().current_index().unwrap_or(0));
            if let Some(track) = player.playlist_mut().get_mut(idx) {
                track.is_favourite = true;
                crate::play_stats::set_favourite(&track.file_path, true);
            }
        }
        FavAction::Remove(v) => {
            let idx = v.index.unwrap_or_else(|| player.playlist_mut().current_index().unwrap_or(0));
            if let Some(track) = player.playlist_mut().get_mut(idx) {
                track.is_favourite = false;
                crate::play_stats::set_favourite(&track.file_path, false);
            }
        }
        FavAction::Toggle(v) => {
            let idx = v.index.unwrap_or_else(|| player.playlist_mut().current_index().unwrap_or(0));
            if let Some(track) = player.playlist_mut().get_mut(idx) {
                track.is_favourite = !track.is_favourite;
                crate::play_stats::set_favourite(&track.file_path, track.is_favourite);
                println!("{} {}", if track.is_favourite { "\u{2665}" } else { "\u{2661}" }, track.display_name("title"));
            }
        }
        FavAction::List => {
            let pl = get_player().playlist_mut();
            let fmt = player.display_format();
            for (i, t) in pl.tracks().iter().enumerate() {
                if t.is_favourite {
                    println!("\u{2665} {:<4} {}", i, t.display_name(&fmt));
                }
            }
        }
    }
    Ok(())
}

pub fn cmd_rate(args: &RateArgs) -> Result<()> {
    let player = get_player();
    let idx = args.index.unwrap_or_else(|| player.playlist_mut().current_index().unwrap_or(0));
    let rating = args.rating.min(5);
    if let Some(track) = player.playlist_mut().get_mut(idx) {
        track.rating = rating;
        crate::play_stats::set_rating(&track.file_path, rating);
        let stars = "\u{2605}".repeat(rating as usize) + &"\u{2606}".repeat((5 - rating) as usize);
        println!("{} {} ({})", stars, track.display_name("title"), rating);
    } else {
        eprintln!("Invalid track index: {idx}");
    }
    Ok(())
}

/// Display lyrics with progress bar
fn display_lyrics(track: &crate::core::playlist::Track, lyrics: &crate::lyric::parser::Lyrics, pos_ms: u64) -> Result<()> {
    let cfg = crate::config::Config::load();
    let show_trans = cfg.lyric.show_translate;
    let adj_pos = if lyrics.offset_ms > 0 {
        pos_ms.saturating_sub(lyrics.offset_ms as u64)
    } else {
        pos_ms.saturating_add((-lyrics.offset_ms) as u64)
    };

    if lyrics.is_empty() {
        println!("  (empty lyrics)");
        return Ok(());
    }

    let current = lyrics.current_line_index(adj_pos);
    let next = lyrics.next_line_index(adj_pos);

    println!("\n\u{1f3b5} Lyrics for: {}", track.display_name("title"));
    if !lyrics.artist.is_empty() || !lyrics.album.is_empty() {
        let meta = [lyrics.artist.as_str(), lyrics.album.as_str()]
            .iter().filter(|s| !s.is_empty())
            .copied().collect::<Vec<&str>>().join(" \u{2014} ");
        println!("   {meta}\n");
    }

    let context = 3;
    let start = current.unwrap_or(0).saturating_sub(context);
    let end = (current.unwrap_or(0) + context + 1).min(lyrics.len().saturating_sub(1));

    let cur_ts = format!("[{:02}:{:02}.{:02}]",
        (pos_ms / 60000) % 60, (pos_ms / 1000) % 60, (pos_ms % 1000) / 10);

    for i in start..=end {
        let line = &lyrics.lines[i];
        let is_current = Some(i) == current;
        let is_next = Some(i) == next;
        if is_current {
            let elapsed = adj_pos.saturating_sub(line.time_ms);
            let next_time = lyrics.lines.get(i + 1).map_or(line.time_ms + 5000, |l| l.time_ms);
            let total = next_time.saturating_sub(line.time_ms);
            let pct = if total > 0 { (elapsed * 100 / total).min(100) } else { 0 };
            let bar_len: usize = 20;
            let filled_usize = (pct as usize * bar_len / 100).min(bar_len);
            let bar: String = (0..bar_len).map(|j| {
                if j < filled_usize { '\u{2588}' }
                else if j == filled_usize.min(bar_len - 1) { '\u{2592}' }
                else { '\u{2591}' }
            }).collect();
            println!("{cur_ts}  {bar} {pct: >3}%");
            print!("  \u{25b6}\u{25b6} {}", line.text);
            if show_trans && !line.translate.is_empty() {
                print!("  [{}]", line.translate);
            }
            println!();
        } else if is_next {
            print!("  \u{23ed} {}", line.text);
            if show_trans && !line.translate.is_empty() {
                print!("  [{}]", line.translate);
            }
            println!();
        } else {
            print!("     {}", line.text);
            if show_trans && !line.translate.is_empty() {
                print!("  [{}]", line.translate);
            }
            println!();
        }
    }
    Ok(())
}

pub fn cmd_lyric(args: &LyricArgs) -> Result<()> {
    match &args.action {
        LyricAction::Show => {
            let player = get_player();
            let pl = player.playlist_mut();
            if let Some(track) = pl.current_track() {
                let cfg = crate::config::Config::load();

                // Check embedded lyrics first if configured
                if cfg.lyric.use_inner_lyric_first {
                    if let Ok(lyric_text) = crate::tag::reader::read_embedded_lyrics(&track.file_path) {
                        if let Ok(lyrics) = crate::lyric::parser::parse_lrc_str(&lyric_text) {
                            let pos = player.position().as_millis() as u64;
                            display_lyrics(track, &lyrics, pos)?;
                            return Ok(());
                        }
                    } // fall through to file-based lyrics
                }

                let lrc_path = if track.lyric_file.is_empty() {
                    let p = std::path::Path::new(&track.file_path).with_extension("lrc");
                    if p.exists() { Some(p) } else { None }
                } else {
                    let p = std::path::Path::new(&track.lyric_file);
                    if p.exists() { Some(p.to_path_buf()) } else { None }
                };

                if let Some(path) = lrc_path {
                    match crate::lyric::parser::load_lyric_file(path.to_str().unwrap()) {
                        Ok(lyrics) => {
                            let pos = player.position().as_millis() as u64;
                            display_lyrics(track, &lyrics, pos)?;
                        }
                        Err(e) => {
                            eprintln!("Error: Cannot load lyric: {e}");
                        }
                    }
                } else {
                    println!("  (no lyric file found)");
                    println!("  Tip: use `lyric link <file.lrc>` to manually link");
                }
            } else {
                println!("No track playing. Start playback first.");
            }
        }
        LyricAction::Offset(args) => {
            let player = get_player();
            let mut pl = player.playlist_mut();
            if let Some(idx) = pl.current_index() {
                if let Some(track) = pl.get_mut(idx) {
                    track.lyric_offset = args.offset;
                    println!("\u{2705} Lyric offset set to {} ms for current track", args.offset);
                    println!("   (positive = delay lyrics, negative = advance lyrics)");
                }
            } else {
                eprintln!("No track playing.");
            }
        }
        LyricAction::Link(args) => {
            let linked_path = &args.file;
            let player = get_player();
            let mut pl = player.playlist_mut();
            if let Some(idx) = pl.current_index() {
                if let Some(track) = pl.get_mut(idx) {
                    let abs_path = std::path::Path::new(linked_path);
                    let abs_path = if abs_path.is_absolute() {
                        linked_path.clone()
                    } else if let Ok(cwd) = std::env::current_dir() {
                        cwd.join(abs_path).to_string_lossy().to_string()
                    } else {
                        linked_path.clone()
                    };
                    track.lyric_file = abs_path.clone();
                    println!("\u{2705} Linked lyric file: {abs_path}");
                    println!("   Use `lyric clear` to unlink");
                }
            } else {
                eprintln!("No track playing.");
            }
        }
        LyricAction::Clear => {
            let player = get_player();
            let mut pl = player.playlist_mut();
            if let Some(idx) = pl.current_index() {
                if let Some(track) = pl.get_mut(idx) {
                    track.lyric_file.clear();
                    println!("\u{2705} Lyric association cleared");
                }
            } else {
                eprintln!("No track playing.");
            }
        }
        LyricAction::Search(v) => {
            println!("Searching for lyrics: '{}'...", v.keyword);
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(crate::online::netease_search(&v.keyword)) {
                Ok(results) => {
                    if results.is_empty() {
                        println!("  (no results found)");
                    } else {
                        for (i, r) in results.iter().enumerate() {
                            println!("  {}. {} - {} [{}]", i, r.artist, r.title, r.album);
                        }
                        println!("\nUse `lyric download --service netease` for the current track, or:");
                        println!("  lyric download --service netease (auto-matches current track)");
                    }
                }
                Err(e) => {
                    eprintln!("Search failed: {e}");
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    if let Ok(qq_results) = rt.block_on(crate::online::qqmusic_search(&v.keyword)) {
                        if !qq_results.is_empty() {
                            println!("QQ Music results:");
                            for (i, r) in qq_results.iter().enumerate() {
                                println!("  {}. {} - {}", i, r.artist, r.title);
                            }
                        }
                    }
                }
            }
        }
        LyricAction::Download(v) => {
            let player = get_player();
            let pl = player.playlist_mut();
            let track = if let Some(t) = pl.current_track() { t.clone() } else {
                eprintln!("No track playing. Specify a keyword with `lyric search` first.");
                return Ok(());
            };

            let service = v.service.as_deref().unwrap_or("netease");
            let keyword = format!("{} {}", track.artist, track.title);
            let keyword = keyword.trim();
            let keyword = if keyword.is_empty() { &track.file_name } else { keyword };

            println!("Downloading lyrics for '{keyword}' from {service}...");

            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = match service {
                "qq" | "qqmusic" => {
                    let results = rt.block_on(crate::online::qqmusic_search(keyword))?;
                    if results.is_empty() {
                        Err(PlayerError::Other("No results from QQ Music".into()))
                    } else {
                        let lrc = rt.block_on(crate::online::qqmusic_download_lyric(&results[0].id))?;
                        let audio_path = std::path::Path::new(&track.file_path);
                        let lrc_path = audio_path.with_extension("lrc");
                        std::fs::write(&lrc_path, &lrc)?;
                        println!("\u{2705} Lyrics saved to: {}", lrc_path.display());
                        Ok(())
                    }
                }
                _ => {
                    let results = rt.block_on(crate::online::netease_search(keyword))?;
                    if results.is_empty() {
                        Err(PlayerError::Other("No results from Netease".into()))
                    } else {
                        let lrc = rt.block_on(crate::online::netease_download_lyric(&results[0].id))?;
                        let audio_path = std::path::Path::new(&track.file_path);
                        let lrc_path = audio_path.with_extension("lrc");
                        std::fs::write(&lrc_path, &lrc)?;
                        println!("\u{2705} Lyrics saved to: {}", lrc_path.display());
                        Ok(())
                    }
                }
            };

            if let Err(e) = result {
                eprintln!("Lyric download failed: {e}");
            }
        }
    }
    Ok(())
}

/// Look up metadata from `MusicBrainz`
pub fn cmd_musicbrainz(args: &MusicBrainzArgs) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let keyword = args.keyword.clone().unwrap_or_default();
    let (title, artist) = if keyword.is_empty() {
        let pl = get_player().playlist_mut();
        if let Some(track) = pl.current_track() {
            (track.title.clone(), track.artist.clone())
        } else {
            eprintln!("No track playing. Specify a keyword.");
            return Ok(());
        }
    } else {
        let parts: Vec<&str> = keyword.splitn(2, " - ").collect();
        if parts.len() == 2 {
            (parts[1].to_string(), parts[0].to_string())
        } else {
            (keyword.clone(), args.artist.clone().unwrap_or_default())
        }
    };

    let results = match rt.block_on(crate::online::musicbrainz_search(&title, &artist)) {
        Ok(r) => r,
        Err(e) => { eprintln!("MusicBrainz search failed: {e}"); return Ok(()); }
    };

    if results.is_empty() {
        println!("No results from MusicBrainz for '{keyword}'");
        return Ok(());
    }

    println!("\nMusicBrainz results for '{keyword}':");
    for (i, r) in results.iter().enumerate() {
        println!("  {}. {} - {} ({} | {})", i, r.artist, r.title, r.album, r.year);
    }

    if args.auto {
        let pick = &results[0];
        let pl = get_player().playlist_mut();
        if let Some(track) = pl.current_track() {
            let path_str = track.file_path.clone();
            drop(pl);
            let mut fields = Vec::new();
            if !pick.title.is_empty() { fields.push(("title", pick.title.clone())); }
            if !pick.artist.is_empty() { fields.push(("artist", pick.artist.clone())); }
            if !pick.album.is_empty() { fields.push(("album", pick.album.clone())); }
            if let Some(tn) = pick.track_num { fields.push(("track", tn.to_string())); }
            if !pick.year.is_empty() { fields.push(("year", pick.year[..4.min(pick.year.len())].to_string())); }
            for (field, value) in &fields {
                if !value.is_empty() {
                    let _ = crate::tag::writer::set_tag_field(&path_str, field, value);
                }
            }
            println!("\u{2705} Auto-tagged from MusicBrainz: {} - {}", pick.artist, pick.title);
        }
    } else if args.apply < results.len() {
        let pick = &results[args.apply];
        let pl = get_player().playlist_mut();
        if let Some(track) = pl.current_track() {
            let path_str = track.file_path.clone();
            drop(pl);
            let mut fields = Vec::new();
            if !pick.title.is_empty() { fields.push(("title", pick.title.clone())); }
            if !pick.artist.is_empty() { fields.push(("artist", pick.artist.clone())); }
            if !pick.album.is_empty() { fields.push(("album", pick.album.clone())); }
            if let Some(tn) = pick.track_num { fields.push(("track", tn.to_string())); }
            if !pick.year.is_empty() { fields.push(("year", pick.year[..4.min(pick.year.len())].to_string())); }
            for (field, value) in &fields {
                if !value.is_empty() {
                    let _ = crate::tag::writer::set_tag_field(&path_str, field, value);
                }
            }
            println!("\u{2705} Applied result #{}: {} - {}", args.apply, pick.artist, pick.title);
        }
    }
    Ok(())
}

pub fn cmd_cover(args: &CoverArgs) -> Result<()> {
    match &args.action {
        CoverAction::Show => {
            let pl = get_player().playlist_mut();
            if let Some(track) = pl.current_track() {
                let has_embedded = match crate::tag::writer::read_pictures(&track.file_path) {
                    Ok(pics) => {
                        println!("Album art for: {}", track.display_name("title"));
                        for (i, (ext, data)) in pics.iter().enumerate() {
                            println!("  [{}] Embedded: {}.{} ({} bytes)", i, track.file_path, ext, data.len());
                        }
                        true
                    }
                    Err(_) => false,
                };
                // Check for external cover files
                let audio_path = std::path::Path::new(&track.file_path);
                if let Some(dir) = audio_path.parent() {
                    for ext in &["jpg", "jpeg", "png", "bmp"] {
                        let cover_path = dir.join(format!("cover.{ext}"));
                        if cover_path.exists() {
                            let size = std::fs::metadata(&cover_path).map(|m| m.len()).unwrap_or(0);
                            println!("  [external] {} ({} bytes)", cover_path.display(), size);
                        }
                        let folder_path = dir.join(format!("folder.{ext}"));
                        if folder_path.exists() && folder_path != cover_path {
                            let size = std::fs::metadata(&folder_path).map(|m| m.len()).unwrap_or(0);
                            println!("  [external] {} ({} bytes)", folder_path.display(), size);
                        }
                    }
                }
                if !has_embedded {
                    println!("  (no embedded or external cover art found)");
                }
            } else {
                println!("No track playing");
            }
        }
        CoverAction::Extract { output } => {
            let pl = get_player().playlist_mut();
            if let Some(track) = pl.current_track() {
                match crate::tag::writer::read_pictures(&track.file_path) {
                    Ok(pics) => {
                        let save_path = output.clone().unwrap_or_else(|| {
                            let p = std::path::Path::new(&track.file_path);
                            let stem = p.file_stem().map_or("cover", |s| s.to_str().unwrap_or("cover"));
                            p.with_file_name(format!("{}.{}", stem, pics[0].0))
                                .to_str().unwrap().to_string()
                        });
                        std::fs::write(&save_path, &pics[0].1)?;
                        println!("Cover extracted to: {save_path}");
                    }
                    Err(e) => eprintln!("Cannot extract cover: {e}"),
                }
            } else {
                println!("No track playing");
            }
        }
        CoverAction::Write(args) => {
            let target_file = if let Some(file) = &args.file {
                file.clone()
            } else {
                let pl = get_player().playlist_mut();
                if let Some(t) = pl.current_track() { t.file_path.clone() } else {
                    eprintln!("No track playing. Specify a file path.");
                    return Ok(());
                }
            };
            match crate::tag::writer::write_picture(&target_file, &args.image) {
                Ok(()) => println!("\u{2705} Cover image '{}' written to '{}'", args.image, target_file),
                Err(e) => eprintln!("Error writing cover: {e}"),
            }
        }
        CoverAction::Download => {
            let pl = get_player().playlist_mut();
            let track = if let Some(t) = pl.current_track() { t.clone() } else {
                eprintln!("No track playing.");
                return Ok(());
            };
            let keyword = format!("{} {}", track.artist, track.title);
            let keyword = keyword.trim();
            let keyword = if keyword.is_empty() { &track.file_name } else { keyword };

            println!("Searching album cover for '{keyword}'...");
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(crate::online::netease_search(keyword)) {
                Ok(results) => {
                    if let Some(cover_url) = results.first().and_then(|r| r.cover_url.as_ref()) {
                        let audio_path = std::path::Path::new(&track.file_path);
                        let cover_path = audio_path.with_file_name("cover.jpg");
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        match rt.block_on(crate::online::download_cover(cover_url, cover_path.to_str().unwrap())) {
                            Ok(()) => println!("\u{2705} Cover saved to: {}", cover_path.display()),
                            Err(e) => eprintln!("Cover download failed: {e}"),
                        }
                    } else {
                        eprintln!("No cover URL found for '{keyword}'");
                    }
                }
                Err(e) => {
                    eprintln!("Cover search failed: {e}");
                }
            }
        }
        CoverAction::Clear => {
            let cache_dir = crate::config::get_config_dir().join("covers");
            if cache_dir.exists() {
                match std::fs::remove_dir_all(&cache_dir) {
                    Ok(()) => println!("Cover cache cleared: {}", cache_dir.display()),
                    Err(e) => eprintln!("Error clearing cover cache: {e}"),
                }
            } else {
                println!("Cover cache directory not found.");
            }
        }
    }
    Ok(())
}

pub fn cmd_tag(args: &TagArgs) -> Result<()> {
    match &args.action {
        TagAction::Show(v) => {
            match crate::tag::reader::read_tags(&v.file) {
                Ok(track) => {
                    println!("File: {}", track.file_path);
                    println!("Title:  {}", track.title);
                    println!("Artist: {}", track.artist);
                    println!("Album:  {}", track.album);
                    println!("Genre:  {}", track.genre);
                    println!("Track:  {}", track.track_number);
                    println!("Year:   {}", track.year);
                    println!("Duration: {:?}", track.duration);
                    if track.sample_rate > 0 {
                        println!("Sample rate: {} Hz", track.sample_rate);
                    }
                    if track.bit_depth > 0 {
                        println!("Bit depth: {}-bit", track.bit_depth);
                    }
                    if track.channels > 0 {
                        let ch = match track.channels { 1 => "Mono", 2 => "Stereo", 6 => "5.1", 8 => "7.1", _ => "Multi" };
                        println!("Channels: {} ({})", track.channels, ch);
                    }
                    println!("Bitrate: {}", track.bitrate);
                    if let Ok(pics) = crate::tag::writer::read_pictures(&v.file) {
                        println!("Covers: {} picture(s)", pics.len());
                    }
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        TagAction::Set(v) => {
            match crate::tag::writer::set_tag_field(&v.file, &v.field, &v.value) {
                Ok(()) => println!("Tag set: {} -> {}={}", v.file, v.field, v.value),
                Err(e) => eprintln!("Error setting tag: {e}"),
            }
        }
        TagAction::Batch(v) => {
            println!("Batch rename in {} with pattern '{}'", v.dir, v.pattern);
            let dir = std::path::Path::new(&v.dir);
            if !dir.is_dir() {
                eprintln!("Not a directory: {}", v.dir);
                return Ok(());
            }
            let read_dir = match std::fs::read_dir(dir) {
                Ok(rd) => rd,
                Err(e) => { eprintln!("Cannot read dir: {e}"); return Ok(()); }
            };
            for entry in read_dir {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let path = entry.path();
                if path.is_dir() || !path.is_file() { continue; }
                let path_str = path.to_str().unwrap_or_default().to_string();
                if let Ok(track) = crate::tag::reader::read_tags(&path_str) {
                    let new_name = v.pattern
                        .replace("{title}", &track.title)
                        .replace("{artist}", &track.artist)
                        .replace("{album}", &track.album)
                        .replace("{track}", &format!("{:02}", track.track_number))
                        .replace("{year}", &track.year.to_string());
                    if let Some(parent) = path.parent() {
                        let new_path = parent.join(&new_name).with_extension(
                            path.extension().unwrap_or_default()
                        );
                        if new_path != path {
                            match std::fs::rename(&path, &new_path) {
                                Ok(()) => println!("  Renamed: {} -> {}", path_str, new_path.display()),
                                Err(e) => eprintln!("  Error renaming {path_str}: {e}"),
                            }
                        }
                    }
                }
            }
        }
        TagAction::Format(v) => {
            let src = &v.src;
            let dest = &v.dest;

            if !std::path::Path::new(src).exists() {
                eprintln!("Source file not found: {src}");
                return Ok(());
            }

            let dest_path = std::path::Path::new(dest);
            if let Some(parent) = dest_path.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }

            // Determine output format from dest extension or --format flag
            let fmt = v.format.clone().unwrap_or_else(|| {
                dest_path.extension()
                    .and_then(|e| e.to_str())
                    .map(str::to_lowercase)
                    .unwrap_or_default()
            });

            // Get source duration for progress calculation
            let src_duration_ms = if v.progress {
                crate::tag::reader::read_tags(src)
                    .ok()
                    .and_then(|t| {
                        let d = t.duration;
                        if d > std::time::Duration::ZERO { Some(d) } else { None }
                    })
            } else {
                None
            };

            println!("Converting: {src} -> {dest} (format: {fmt})");

            // Prefer ffmpeg for conversion, fall back to format-specific tools
            let ffmpeg_check = std::process::Command::new("ffmpeg")
                .arg("-version")
                .output();

            let result = if ffmpeg_check.is_ok() {
                // Determine codec based on format
                let (codec, ext): (&str, &str) = match fmt.as_str() {
                    "mp3" => ("libmp3lame", "mp3"),
                    "ogg" | "vorbis" => ("libvorbis", "ogg"),
                    "flac" => ("flac", "flac"),
                    "wav" => ("pcm_s16le", "wav"),
                    "aac" | "m4a" => ("aac", "m4a"),
                    "opus" => ("libopus", "opus"),
                    "wma" => ("wmav2", "wma"),
                    _ => {
                        eprintln!("Unsupported output format: {fmt}");
                        eprintln!("Supported: mp3, ogg, flac, wav, aac, opus, wma");
                        return Ok(());
                    }
                };

                // Build ffmpeg arguments based on mode and format
                let mode = v.mode.to_lowercase();
                let mut extra_args: Vec<String> = Vec::new();
                if mode.as_str() == "vbr" {
                    let q = v.quality.unwrap_or(2);
                    extra_args.push("-q:a".to_string());
                    extra_args.push(q.to_string());
                } else {
                    // cbr or abr — both use -b:a for libmp3lame (abr) / other codecs (cbr)
                    let br = v.bitrate.unwrap_or(match fmt.as_str() {
                        "mp3" => 320,
                        "aac" | "m4a" => 256,
                        "opus" => 128,
                        "wma" => 192,
                        "ogg" | "vorbis" => 192,
                        _ => 192,
                    });
                    extra_args.push("-b:a".to_string());
                    extra_args.push(format!("{br}k"));
                }

                if let (true, Some(duration)) = (v.progress, src_duration_ms) {
                    // ── Live progress bar mode ──────────────────────────────
                    let total_us = duration.as_micros() as u64;

                    let mut child = std::process::Command::new("ffmpeg")
                        .args(["-nostats", "-progress", "pipe:2", "-i", src, "-c:a", codec])
                        .args(&extra_args)
                        .args(["-y", dest])
                        .stderr(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::null())
                        .spawn()
                        .map_err(|e| PlayerError::Other(format!("Cannot run ffmpeg: {e}")))?;

                    let stderr = child.stderr.take()
                        .ok_or_else(|| PlayerError::Other("Cannot capture ffmpeg stderr".into()))?;

                    use std::io::{BufRead, BufReader, Write};
                    let reader = BufReader::new(stderr);
                    for line in reader.lines() {
                        let line = match line {
                            Ok(l) => l,
                            Err(_) => break,
                        };
                        if let Some(us_str) = line.strip_prefix("out_time_us=") {
                            if let Ok(us) = us_str.trim().parse::<u64>() {
                                let pct = if total_us > 0 {
                                    (us as f64 / total_us as f64) * 100.0
                                } else {
                                    0.0
                                };
                                if (0.0..=100.0).contains(&pct) {
                                    print_conversion_progress(pct);
                                }
                            }
                        }
                    }

                    // Wait for ffmpeg to finish
                    let status = child.wait()
                        .map_err(|e| PlayerError::Other(format!("ffmpeg wait failed: {e}")))?;

                    // Clear progress line
                    print!("\r{}\r", " ".repeat(60));
                    let _ = std::io::stdout().flush();

                    if status.success() {
                        println!("\u{2705} Converted: {src} -> {dest} ({ext} format)");
                        if let Ok(meta) = std::fs::metadata(dest) {
                            println!("   Output size: {} bytes", meta.len());
                        }
                        Ok(())
                    } else {
                        Err(PlayerError::Other(format!(
                            "ffmpeg conversion failed (exit code: {:?})",
                            status.code()
                        )))
                    }
                } else {
                    // ── Original blocking mode ─────────────────────────────
                    let output = std::process::Command::new("ffmpeg")
                        .args(["-i", src, "-c:a", codec])
                        .args(&extra_args)
                        .args(["-y", dest])
                        .output()
                        .map_err(|e| PlayerError::Other(format!("Cannot run ffmpeg: {e}")))?;

                    if output.status.success() {
                        println!("\u{2705} Converted: {src} -> {dest} ({ext} format)");
                        if let Ok(meta) = std::fs::metadata(dest) {
                            println!("   Output size: {} bytes", meta.len());
                        }
                        Ok(())
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let first_line = stderr.lines().next().unwrap_or("");
                        Err(PlayerError::Other(format!("ffmpeg conversion failed: {first_line}")))
                    }
                }
            } else {
                Err(PlayerError::Other(
                    "ffmpeg not found in PATH. Please install FFmpeg to use format conversion.".into()
                ))
            };

            if let Err(e) = result {
                eprintln!("Conversion failed: {e}");
            }
        }
        TagAction::FromName(v) => {
            let dir = std::path::Path::new(&v.dir);
            if !dir.is_dir() {
                eprintln!("Not a directory: {}", v.dir);
                return Ok(());
            }
            let pattern = &v.pattern;
            let re_pattern = pattern
                .replace("{artist}", "(?<artist>.+)")
                .replace("{title}", "(?<title>.+)")
                .replace("{album}", "(?<album>.+)")
                .replace("{track}", "(?<track>\\d+)");
            let re = match regex::Regex::new(&format!("^{re_pattern}$")) {
                Ok(r) => r,
                Err(e) => { eprintln!("Invalid pattern: {e}"); return Ok(()); }
            };

            let read_dir = match std::fs::read_dir(dir) {
                Ok(rd) => rd,
                Err(e) => { eprintln!("Cannot read dir: {e}"); return Ok(()); }
            };

            let mut updated = 0;
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_dir() || !crate::audio_common::file_is_audio(path.to_str().unwrap_or_default()) {
                    continue;
                }
                let file_stem = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default();
                if let Some(caps) = re.captures(file_stem) {
                    let title = caps.name("title").map(|m| m.as_str().to_string());
                    let artist = caps.name("artist").map(|m| m.as_str().to_string());
                    let album = caps.name("album").map(|m| m.as_str().to_string());
                    let track = caps.name("track").and_then(|m| m.as_str().parse::<u32>().ok());

                    let path_str = path.to_str().unwrap_or_default();
                    if let Some(t) = &title {
                        let _ = crate::tag::writer::set_tag_field(path_str, "title", t);
                    }
                    if let Some(a) = &artist {
                        let _ = crate::tag::writer::set_tag_field(path_str, "artist", a);
                    }
                    if let Some(a) = &album {
                        let _ = crate::tag::writer::set_tag_field(path_str, "album", a);
                    }
                    if let Some(t) = track {
                        let _ = crate::tag::writer::set_tag_field(path_str, "track", &t.to_string());
                    }
                    let new_name = file_stem.to_string();
                    println!("  Updated tags from: {new_name}");
                    updated += 1;
                }
            }
            println!("Updated {updated} file(s)");
        }
        TagAction::Online(v) => {
            let path = std::path::Path::new(&v.file);
            if !path.exists() {
                eprintln!("File not found: {}", v.file);
                return Ok(());
            }
            let path_str = v.file.as_str();

            // Read existing tags to build search keyword
            let existing = crate::tag::reader::read_tags(path_str).ok();
            let keyword = match &existing {
                Some(t) if !t.title.is_empty() || !t.artist.is_empty() => {
                    format!("{} {}", t.artist, t.title).trim().to_string()
                }
                _ => {
                    path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string()
                }
            };
            if keyword.is_empty() || keyword == " " {
                eprintln!("Could not determine search keyword from file");
                return Ok(());
            }

            println!("Searching for '{}' on {}...", keyword.trim(), v.service);

            let rt = tokio::runtime::Runtime::new().unwrap();
            let results = match v.service.as_str() {
                "qq" | "qqmusic" => rt.block_on(crate::online::qqmusic_search(keyword.trim())),
                _ => rt.block_on(crate::online::netease_search(keyword.trim())),
            };

            let results = match results {
                Ok(r) => r,
                Err(e) => { eprintln!("Search failed: {e}"); return Ok(()); }
            };

            if results.is_empty() {
                eprintln!("No results found for '{}'", keyword.trim());
                return Ok(());
            }

            // Pick result
            let pick = if v.auto || results.len() == 1 {
                &results[0]
            } else {
                println!("\nMultiple results found. Select one:");
                for (i, r) in results.iter().enumerate() {
                    println!("  {}. {} - {} [{}]", i + 1, r.artist, r.title, r.album);
                }
                print!("Enter number (1-{}), or 0 to cancel: ", results.len());
                use std::io::Write;
                std::io::stdout().flush().ok();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).ok();
                match input.trim().parse::<usize>() {
                    Ok(n) if n >= 1 && n <= results.len() => &results[n - 1],
                    _ => { println!("Cancelled"); return Ok(()); }
                }
            };

            // Write tags
            let fields = [
                ("title", &pick.title),
                ("artist", &pick.artist),
                ("album", &pick.album),
            ];
            for (field, value) in &fields {
                if !value.is_empty() {
                    let _ = crate::tag::writer::set_tag_field(path_str, field, value);
                }
            }
            println!("\u{2705} Written: {} - {} ({})", pick.artist, pick.title, pick.album);

            // Download and embed cover if requested
            if v.cover {
                if let Some(ref cover_url) = pick.cover_url {
                    let tmp = std::env::temp_dir().join("online_cover.jpg");
                    let tmp_str = tmp.to_str().unwrap_or_default().to_string();
                    match rt.block_on(crate::online::download_cover(cover_url, &tmp_str)) {
                        Ok(()) => {
                            match crate::tag::writer::write_picture(path_str, &tmp_str) {
                                Ok(()) => println!("\u{2705} Cover embedded from online source"),
                                Err(e) => eprintln!("Failed to embed cover: {e}"),
                            }
                            let _ = std::fs::remove_file(&tmp);
                        }
                        Err(e) => eprintln!("Failed to download cover: {e}"),
                    }
                }
            }
        }
    }
    Ok(())
}

/// Print a colored one-line conversion progress bar.
///
/// Uses `\r` (carriage return) to overwrite the previous line, so the
/// terminal shows a single live-updating progress line.
fn print_conversion_progress(pct: f64) {
    use std::io::Write;

    const BAR_WIDTH: usize = 30;
    let filled = ((pct / 100.0) * BAR_WIDTH as f64).round() as usize;
    let filled = filled.min(BAR_WIDTH);
    let empty = BAR_WIDTH - filled;

    let bar_filled: String = std::iter::repeat_n('\u{2588}', filled).collect(); // █
    let bar_empty: String = std::iter::repeat_n('\u{2591}', empty).collect();   // ░

    let pct_color = match pct as u32 {
        0..=25 => crate::color::BRIGHT_BLUE,
        26..=50 => crate::color::CYAN,
        51..=75 => crate::color::YELLOW,
        76..=89 => crate::color::GREEN,
        _ => crate::color::BRIGHT_GREEN,
    };
    let bar_color = match pct as u32 {
        _ if pct >= 90.0 => crate::color::BRIGHT_GREEN,
        _ if pct >= 60.0 => crate::color::GREEN,
        _ if pct >= 30.0 => crate::color::YELLOW,
        _ => crate::color::BRIGHT_BLUE,
    };

    let line = format!(
        "\r {} [{}{}{}] {:5.1}%  ",
        crate::color::BOLD,
        crate::color::colorize(&bar_filled, bar_color),
        crate::color::colorize(&bar_empty, crate::color::DIM),
        crate::color::RESET,
        crate::color::colorize(&format!("{pct:.1}"), pct_color),
    );

    print!("{line}");
    let _ = std::io::stdout().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_conversion_progress_boundaries() {
        // Just verify it doesn't panic at boundary values
        print_conversion_progress(0.0);
        print_conversion_progress(25.0);
        print_conversion_progress(50.0);
        print_conversion_progress(75.0);
        print_conversion_progress(90.0);
        print_conversion_progress(100.0);
    }

    #[test]
    fn print_conversion_progress_color_ranges() {
        // Verify color matching logic covers all ranges without panicking
        let test_cases = [0, 25, 26, 50, 51, 75, 76, 89, 90, 100];
        for pct in test_cases {
            print_conversion_progress(pct as f64);
        }
    }
}
