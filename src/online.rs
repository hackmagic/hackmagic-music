use crate::error::{PlayerError, Result};
use serde_json::Value;

/// Search result from online service
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover_url: Option<String>,
}

/// Search for a song on Netease Cloud Music
pub async fn netease_search(keyword: &str) -> Result<Vec<SearchResult>> {
    let url = format!(
        "https://music.163.com/api/search/get?type=1&s={}&limit=10",
        urlencoding(keyword)
    );
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| PlayerError::Other(e.to_string()))?;

    let resp: Value = client
        .get(&url)
        .header("Referer", "https://music.163.com/")
        .send()
        .await?
        .json()
        .await?;

    let songs = resp["result"]["songs"]
        .as_array()
        .ok_or_else(|| PlayerError::Other("No results from Netease".into()))?;

    Ok(songs
        .iter()
        .map(|s| SearchResult {
            id: s["id"].as_i64().unwrap_or(0).to_string(),
            title: s["name"].as_str().unwrap_or("").to_string(),
            artist: s["artists"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|a| a["name"].as_str())
                .unwrap_or("")
                .to_string(),
            album: s["album"]["name"].as_str().unwrap_or("").to_string(),
            cover_url: s["album"]["picUrl"].as_str().map(std::string::ToString::to_string),
        })
        .collect())
}

/// Search for a song on QQ Music
pub async fn qqmusic_search(keyword: &str) -> Result<Vec<SearchResult>> {
    let url = format!(
        "https://c.y.qq.com/splcloud/fcgi-bin/smartbox_new.fcg?key={}&format=json",
        urlencoding(keyword)
    );
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()
        .map_err(|e| PlayerError::Other(e.to_string()))?;

    let resp: Value = client.get(&url).send().await?.json().await?;
    let songs = resp["data"]["song"]["itemlist"]
        .as_array()
        .ok_or_else(|| PlayerError::Other("No results from QQ Music".into()))?;

    Ok(songs
        .iter()
        .map(|s| SearchResult {
            id: s["mid"].as_str().unwrap_or("").to_string(),
            title: s["name"].as_str().unwrap_or("").to_string(),
            artist: s["singer"].as_str().unwrap_or("").to_string(),
            album: String::new(),
            cover_url: None,
        })
        .collect())
}

/// Download lyrics from Netease by song ID
pub async fn netease_download_lyric(song_id: &str) -> Result<String> {
    let url = format!(
        "https://music.163.com/api/song/lyric?id={song_id}&lv=1&kv=1&tv=-1"
    );
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| PlayerError::Other(e.to_string()))?;

    let resp: Value = client
        .get(&url)
        .header("Referer", "https://music.163.com/")
        .send()
        .await?
        .json()
        .await?;

    let lrc_text = resp["lrc"]["lyric"]
        .as_str()
        .ok_or_else(|| PlayerError::Other("No lyrics found on Netease".into()))?;

    if lrc_text.trim().is_empty() {
        return Err(PlayerError::Other("Empty lyrics from Netease".into()));
    }

    Ok(lrc_text.to_string())
}

/// Download lyrics from QQ Music by song mid
pub async fn qqmusic_download_lyric(song_mid: &str) -> Result<String> {
    let url = format!(
        "https://c.y.qq.com/lyric/fcgi-bin/fcg_query_lyric_new.fcg?songmid={song_mid}&format=json"
    );
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| PlayerError::Other(e.to_string()))?;

    let resp: serde_json::Value = client
        .get(&url)
        .header("Referer", "https://y.qq.com/")
        .send()
        .await?
        .json()
        .await?;

    let lrc_b64 = resp["lyric"]
        .as_str()
        .ok_or_else(|| PlayerError::Other("No lyrics found on QQ Music".into()))?;

    if lrc_b64.trim().is_empty() {
        return Err(PlayerError::Other("Empty lyrics from QQ Music".into()));
    }

    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, lrc_b64)
        .map_err(|e| PlayerError::Other(format!("Base64 decode error: {e}")))?;

    // QQ Music lyrics are GBK-encoded, try UTF-8 first then GBK
    let lrc_text = String::from_utf8(decoded.clone())
        .unwrap_or_else(|_| {
            use encoding_rs::GBK;
            GBK.decode(&decoded).0.to_string()
        });

    Ok(lrc_text)
}

/// Download album cover from a URL
pub async fn download_cover(url: &str, save_path: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()
        .map_err(|e| PlayerError::Other(e.to_string()))?;

    let bytes = client.get(url).send().await?.bytes().await?;
    std::fs::write(save_path, &bytes)?;
    Ok(())
}

fn urlencoding(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

/// `MusicBrainz` recording result
pub struct MbRecording {
    pub mbid: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: String,
    pub track_num: Option<u32>,
}

/// Search `MusicBrainz` for a recording by title and artist
pub async fn musicbrainz_search(title: &str, artist: &str) -> Result<Vec<MbRecording>> {
    let query = format!("recording:\"{title}\" AND artist:\"{artist}\"");
    let url = format!(
        "https://musicbrainz.org/ws/2/recording?query={}&fmt=json&limit=10",
        urlencoding(&query)
    );
    let client = reqwest::Client::builder()
        .user_agent("hm/1.0")
        .build()
        .map_err(|e| PlayerError::Other(e.to_string()))?;
    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;
    let recordings = resp["recordings"].as_array().ok_or_else(|| PlayerError::Other("No results".into()))?;
    Ok(recordings.iter().map(|r| {
        let mbid = r["id"].as_str().unwrap_or("").to_string();
        let title = r["title"].as_str().unwrap_or("").to_string();
        let artist = r["artist-credit"].as_array()
            .and_then(|a| a.first())
            .and_then(|a| a["artist"].as_object())
            .and_then(|a| a["name"].as_str())
            .unwrap_or("").to_string();
        let album = r["releases"].as_array()
            .and_then(|a| a.first())
            .and_then(|r| r["title"].as_str())
            .unwrap_or("").to_string();
        let year = r["releases"].as_array()
            .and_then(|a| a.first())
            .and_then(|r| r["date"].as_str())
            .unwrap_or("").to_string();
        let track_num = r["releases"].as_array()
            .and_then(|a| a.first())
            .and_then(|r| r["track-count"].as_u64())
            .and_then(|_| r["track-list"].as_array())
            .and_then(|tl| tl.first())
            .and_then(|t| t["position"].as_u64())
            .map(|n| n as u32)
            .or_else(|| r["releases"].as_array()
                .and_then(|a| a.first())
                .and_then(|r| r["track-offset"].as_u64()
                    .or_else(|| r["track-count"].as_u64()))
                .map(|n| n as u32));
        MbRecording { mbid, title, artist, album, year, track_num }
    }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencoding_unreserved_chars() {
        // Unreserved chars should remain as-is
        assert_eq!(urlencoding("abcXYZ123"), "abcXYZ123");
        assert_eq!(urlencoding("-_."), "-_.");
        assert_eq!(urlencoding("~"), "~");
    }

    #[test]
    fn test_urlencoding_space() {
        assert_eq!(urlencoding(" "), "%20");
        assert_eq!(urlencoding("hello world"), "hello%20world");
    }

    #[test]
    fn test_urlencoding_special_chars() {
        assert_eq!(urlencoding("!"), "%21");
        assert_eq!(urlencoding("@"), "%40");
        assert_eq!(urlencoding("#"), "%23");
        assert_eq!(urlencoding("$"), "%24");
        assert_eq!(urlencoding("%"), "%25");
        assert_eq!(urlencoding("&"), "%26");
        assert_eq!(urlencoding("+"), "%2B");
        assert_eq!(urlencoding("/"), "%2F");
        assert_eq!(urlencoding("?"), "%3F");
        assert_eq!(urlencoding("="), "%3D");
    }

    #[test]
    fn test_urlencoding_mixed() {
        assert_eq!(
            urlencoding("hello world!@#$%^&*()"),
            "hello%20world%21%40%23%24%25%5E%26%2A%28%29"
        );
    }

    #[test]
    fn test_urlencoding_empty_string() {
        assert_eq!(urlencoding(""), "");
    }

    #[test]
    fn test_urlencoding_unicode() {
        // Non-ASCII chars: cast as u8 truncates, which is the existing behavior
        assert_eq!(urlencoding("\u{00E9}"), "%E9");  // é
        assert_eq!(urlencoding("\u{4E2D}"), "%2D");  // 中 (0x4E2D -> u8 truncates to 0x2D)
    }

    #[test]
    fn test_urlencoding_query_string() {
        assert_eq!(
            urlencoding("q=rust language&page=1"),
            "q%3Drust%20language%26page%3D1"
        );
    }
}
