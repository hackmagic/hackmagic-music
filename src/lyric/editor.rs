//! Lyric editor logic (pure functions, no GPUI dependency).
//! Provides timestamp formatting/parsing for the lyrics editor.

/// Format milliseconds as [MM:SS.xx] timestamp.
pub fn format_time_ms(ms: u64) -> String {
    let minutes = ms / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let centiseconds = (ms % 1_000) / 10;
    format!("[{:02}:{:02}.{:02}]", minutes, seconds, centiseconds)
}

/// Parse a timestamp like `[MM:SS.xx]` to milliseconds.
pub fn parse_time_ms(s: &str) -> Option<u64> {
    let s = s.trim();
    let inner = s.trim_start_matches('[').trim_end_matches(']');
    let parts: Vec<&str> = inner.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let min: u64 = parts[0].parse().ok()?;
    let sec_parts: Vec<&str> = parts[1].split('.').collect();
    let sec: u64 = sec_parts[0].parse().ok()?;
    let cs: u64 = if sec_parts.len() > 1 {
        let cs_str = sec_parts[1];
        if cs_str.len() == 1 {
            cs_str.parse::<u64>().ok()? * 100
        } else if cs_str.len() == 2 {
            cs_str.parse::<u64>().ok()? * 10
        } else {
            cs_str.parse::<u64>().ok()?
        }
    } else {
        0
    };
    Some(min * 60_000 + sec * 1_000 + cs)
}

/// Generate the LRC output string from lyric lines data.
pub fn to_lrc_string(
    rows: &[(String, String)],
    title: Option<&str>,
    artist: Option<&str>,
    album: Option<&str>,
    offset_ms: Option<i64>,
) -> String {
    let mut output = String::new();
    if let Some(t) = title { if !t.is_empty() { output.push_str(&format!("[ti:{}]\n", t)); } }
    if let Some(a) = artist { if !a.is_empty() { output.push_str(&format!("[ar:{}]\n", a)); } }
    if let Some(al) = album { if !al.is_empty() { output.push_str(&format!("[al:{}]\n", al)); } }
    if let Some(off) = offset_ms { if off != 0 { output.push_str(&format!("[offset:{}]\n", off)); } }
    if !output.is_empty() { output.push('\n'); }
    for (ts, text) in rows {
        let line_text = if text.is_empty() { " " } else { text.as_str() };
        output.push_str(&format!("{}{}\n", ts, line_text));
    }
    output
}

/// Adjust a timestamp by a delta in milliseconds.
pub fn adjust_timestamp(ts: &str, delta_ms: i64) -> Option<String> {
    let ms = parse_time_ms(ts)?;
    let new_ms = if delta_ms >= 0 {
        ms.saturating_add(delta_ms as u64)
    } else {
        ms.saturating_sub((-delta_ms) as u64)
    };
    Some(format_time_ms(new_ms))
}

/// Shift all timestamps by a delta (in ms).
pub fn shift_all_timestamps(timestamps: &mut [String], delta_ms: i64) {
    for ts in timestamps.iter_mut() {
        if let Some(new_ts) = adjust_timestamp(ts, delta_ms) {
            *ts = new_ts;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_ms() {
        assert_eq!(format_time_ms(0), "[00:00.00]");
        assert_eq!(format_time_ms(1000), "[00:01.00]");
        assert_eq!(format_time_ms(65000), "[01:05.00]");
        assert_eq!(format_time_ms(12345), "[00:12.34]");
        assert_eq!(format_time_ms(999), "[00:00.99]");
        assert_eq!(format_time_ms(59999), "[00:59.99]");
        assert_eq!(format_time_ms(3600000), "[60:00.00]");
    }

    #[test]
    fn test_parse_time_ms() {
        assert_eq!(parse_time_ms("[00:00.00]"), Some(0));
        assert_eq!(parse_time_ms("[00:01.00]"), Some(1000));
        assert_eq!(parse_time_ms("[01:05.00]"), Some(65000));
        assert_eq!(parse_time_ms("[00:12.34]"), Some(12340));
        assert_eq!(parse_time_ms("[00:00.99]"), Some(990));
        assert_eq!(parse_time_ms("[00:59.99]"), Some(59990));
    }

    #[test]
    fn test_parse_time_ms_three_digit() {
        assert_eq!(parse_time_ms("[00:01.500]"), Some(1500));
        assert_eq!(parse_time_ms("[00:02.999]"), Some(2999));
    }

    #[test]
    fn test_parse_time_ms_variants() {
        assert_eq!(parse_time_ms("[00:01.5]"), Some(1500));
        assert_eq!(parse_time_ms("[01:30]"), Some(90_000));
        assert_eq!(parse_time_ms(" [00:01.00] "), Some(1000));
    }

    #[test]
    fn test_parse_time_ms_invalid() {
        assert_eq!(parse_time_ms("no timestamp"), None);
        assert_eq!(parse_time_ms("[00:00:00.00]"), None);
        assert_eq!(parse_time_ms("[abc:def.gh]"), None);
    }

    #[test]
    fn test_adjust_timestamp_positive() {
        assert_eq!(adjust_timestamp("[00:01.00]", 500), Some("[00:01.50]".to_string()));
    }

    #[test]
    fn test_adjust_timestamp_negative() {
        assert_eq!(adjust_timestamp("[00:01.50]", -500), Some("[00:01.00]".to_string()));
    }

    #[test]
    fn test_adjust_timestamp_saturating() {
        assert_eq!(adjust_timestamp("[00:00.10]", -500), Some("[00:00.00]".to_string()));
    }

    #[test]
    fn test_adjust_timestamp_invalid_input() {
        assert_eq!(adjust_timestamp("not a timestamp", 500), None);
    }

    #[test]
    fn test_shift_all() {
        let mut ts = vec![
            "[00:01.00]".to_string(),
            "[00:02.00]".to_string(),
            "[00:03.00]".to_string(),
        ];
        shift_all_timestamps(&mut ts, 1000);
        assert_eq!(ts[0], "[00:02.00]");
        assert_eq!(ts[1], "[00:03.00]");
        assert_eq!(ts[2], "[00:04.00]");
    }

    #[test]
    fn test_to_lrc_string() {
        let rows = vec![
            ("[00:01.00]".to_string(), "Line one".to_string()),
            ("[00:02.00]".to_string(), "Line two".to_string()),
        ];
        let lrc = to_lrc_string(&rows, Some("Title"), Some("Artist"), Some("Album"), Some(0));
        assert!(lrc.contains("[ti:Title]"));
        assert!(lrc.contains("[ar:Artist]"));
        assert!(lrc.contains("[al:Album]"));
        assert!(lrc.contains("[00:01.00]Line one"));
        assert!(lrc.contains("[00:02.00]Line two"));
    }

    #[test]
    fn test_roundtrip_format_parse() {
        for ms in [0, 100, 1000, 5000, 12345, 60000, 99999] {
            let formatted = format_time_ms(ms);
            let parsed = parse_time_ms(&formatted).unwrap();
            assert!(
                parsed.abs_diff(ms) < 10,
                "Roundtrip failed for {}: formatted={}, parsed={}",
                ms, formatted, parsed
            );
        }
    }
}
