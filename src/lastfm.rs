use crate::config::Config;
use crate::error::{PlayerError, Result};
use crate::core::playlist::Track;
use md5::{Md5, Digest};
use std::collections::BTreeMap;

const API_URL: &str = "https://ws.audioscrobbler.com/2.0/";

pub struct LastfmApi {
    api_key: String,
    shared_secret: String,
    session_key: String,
    username: String,
}

impl LastfmApi {
    pub fn from_config() -> Option<Self> {
        let cfg = Config::load();
        if !cfg.lastfm.enabled || cfg.lastfm.api_key.is_empty() {
            return None;
        }
        Some(Self {
            api_key: cfg.lastfm.api_key.clone(),
            shared_secret: cfg.lastfm.shared_secret.clone(),
            session_key: cfg.lastfm.session_key.clone(),
            username: cfg.lastfm.username.clone(),
        })
    }

    fn sign(params: &BTreeMap<String, String>, secret: &str) -> String {
        let mut s = String::new();
        for (k, v) in params {
            s.push_str(k);
            s.push_str(v);
        }
        s.push_str(secret);
        let mut hasher = Md5::new();
        hasher.update(s.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    async fn api_call(&self, method: &str, mut params: BTreeMap<String, String>) -> Result<serde_json::Value> {
        params.insert("method".into(), method.into());
        params.insert("api_key".into(), self.api_key.clone());
        params.insert("format".into(), "json".into());
        if !self.session_key.is_empty() {
            params.insert("sk".into(), self.session_key.clone());
        }
        let needs_signing = matches!(method, "track.updateNowPlaying" | "track.scrobble" | "track.love" | "track.unlove");
        if needs_signing {
            let sig = Self::sign(&params, &self.shared_secret);
            params.insert("api_sig".into(), sig);
        }
        let client = reqwest::Client::new();
        let resp = client.post(API_URL)
            .form(&params)
            .send()
            .await?;
        let json: serde_json::Value = resp.json().await?;
        if let Some(err) = json.get("error") {
            let msg = json.get("message").and_then(|m| m.as_str()).unwrap_or("unknown");
            return Err(PlayerError::Other(format!("Last.fm error {err}: {msg}")));
        }
        Ok(json)
    }

    pub async fn authenticate(api_key: &str, shared_secret: &str) -> Result<String> {
        let mut params = BTreeMap::new();
        params.insert("method".into(), "auth.getToken".into());
        params.insert("api_key".into(), api_key.into());
        params.insert("format".into(), "json".into());
        let sig = Self::sign(&params, shared_secret);
        params.insert("api_sig".into(), sig);

        let client = reqwest::Client::new();
        let resp: serde_json::Value = client.post(API_URL)
            .form(&params)
            .send()
            .await?
            .json()
            .await?;
        let token = resp.get("token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| PlayerError::Other("Failed to get auth token".into()))?
            .to_string();

        println!("1. Visit: https://www.last.fm/api/auth/?api_key={api_key}&token={token}");
        println!("2. Authorize the application, then use:");
        println!("   lastfm login <username> <api_key> <shared_secret> <token>");

        Ok(token)
    }

    pub async fn get_session(api_key: &str, shared_secret: &str, token: &str) -> Result<(String, String)> {
        let mut params = BTreeMap::new();
        params.insert("method".into(), "auth.getSession".into());
        params.insert("api_key".into(), api_key.into());
        params.insert("token".into(), token.into());
        params.insert("format".into(), "json".into());
        let sig = Self::sign(&params, shared_secret);
        params.insert("api_sig".into(), sig);

        let client = reqwest::Client::new();
        let resp: serde_json::Value = client.post(API_URL)
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        let session = resp.get("session")
            .ok_or_else(|| PlayerError::Other("No session in response".into()))?;
        let key = session.get("key")
            .and_then(|k| k.as_str())
            .ok_or_else(|| PlayerError::Other("No session key".into()))?
            .to_string();
        let name = session.get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        Ok((key, name))
    }

    pub async fn now_playing(&self, track: &Track) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("artist".into(), track.artist.clone());
        params.insert("track".into(), track.title.clone());
        if !track.album.is_empty() {
            params.insert("album".into(), track.album.clone());
        }
        params.insert("duration".into(), track.duration.as_secs().to_string());
        self.api_call("track.updateNowPlaying", params).await?;
        Ok(())
    }

    pub async fn scrobble(&self, track: &Track, timestamp: i64) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("artist".into(), track.artist.clone());
        params.insert("track".into(), track.title.clone());
        if !track.album.is_empty() {
            params.insert("album".into(), track.album.clone());
        }
        params.insert("timestamp".into(), timestamp.to_string());
        self.api_call("track.scrobble", params).await?;
        Ok(())
    }

    pub async fn love(&self, track: &Track) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("artist".into(), track.artist.clone());
        params.insert("track".into(), track.title.clone());
        self.api_call("track.love", params).await?;
        Ok(())
    }

    pub async fn unlove(&self, track: &Track) -> Result<()> {
        let mut params = BTreeMap::new();
        params.insert("artist".into(), track.artist.clone());
        params.insert("track".into(), track.title.clone());
        self.api_call("track.unlove", params).await?;
        Ok(())
    }

    pub fn should_scrobble(track: &Track, position_secs: f64, least_perdur: u32, least_dur: u32) -> bool {
        if track.duration.as_secs() == 0 {
            return false;
        }
        let pct = (position_secs / track.duration.as_secs_f64()) * 100.0;
        pct >= f64::from(least_perdur) && position_secs >= f64::from(least_dur)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::playlist::Track;
    use std::time::Duration;

    fn make_track(duration_secs: u64) -> Track {
        Track {
            file_path: "/fake/path.mp3".to_string(),
            file_name: "path.mp3".to_string(),
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: "Test Album".to_string(),
            genre: String::new(),
            track_number: 1,
            year: 2024,
            duration: Duration::from_secs(duration_secs),
            bitrate: 320,
            file_type: "mp3".to_string(),
            is_cue: false,
            cue_file_path: String::new(),
            cue_track_number: 0,
            start_pos: Duration::ZERO,
            end_pos: Duration::ZERO,
            sample_rate: 44100,
            bit_depth: 16,
            channels: 2,
            is_favourite: false,
            rating: 0,
            listen_count: 0,
            lyric_file: String::new(),
            lyric_offset: 0,
            song_id_netease: 0,
            song_id_qq_music: String::new(),
            flags: 0,
            track_gain: 0.0,
            track_peak: 0.0,
            album_gain: 0.0,
            album_peak: 0.0,
        }
    }

    /// Zero-duration track should never be scrobbled.
    #[test]
    fn test_should_scrobble_zero_duration() {
        let track = make_track(0);
        assert!(!LastfmApi::should_scrobble(&track, 30.0, 50, 30));
    }

    /// Position below percentage threshold should return false, even if above min_dur.
    #[test]
    fn test_should_scrobble_below_percentage() {
        // 200s track, 50% = 100s, min_dur = 60s
        // position=80s → pct=40% < 50% → false (though 80 >= 60)
        let track = make_track(200);
        assert!(!LastfmApi::should_scrobble(&track, 80.0, 50, 60));
    }

    /// Position above percentage threshold but below min_dur should return false.
    #[test]
    fn test_should_scrobble_below_min_duration() {
        // 100s track, 50% = 50s, min_dur = 60s
        // position=55s → pct=55% >= 50%, but 55 < 60 → false
        let track = make_track(100);
        assert!(!LastfmApi::should_scrobble(&track, 55.0, 50, 60));
    }

    /// Both conditions met → true.
    #[test]
    fn test_should_scrobble_meets_both() {
        // 120s track, 50% = 60s, min_dur = 60s
        // position=80s → pct≈67% ≥ 50%, and 80 ≥ 60 → true
        let track = make_track(120);
        assert!(LastfmApi::should_scrobble(&track, 80.0, 50, 60));
    }

    /// Exactly at both thresholds → true.
    #[test]
    fn test_should_scrobble_exact_boundary() {
        // 120s track, 50% = 60s, min_dur = 60s
        // position=60s → pct=50% ≥ 50%, and 60 ≥ 60 → true
        let track = make_track(120);
        assert!(LastfmApi::should_scrobble(&track, 60.0, 50, 60));
    }

    /// Default config thresholds (50%, 60s) with a short track below min_dur.
    #[test]
    fn test_should_scrobble_default_short_track() {
        // 30s track, 50% = 15s, min_dur = 60s
        // position=20s → pct≈67% ≥ 50%, but 20 < 60 → false
        let track = make_track(30);
        assert!(!LastfmApi::should_scrobble(&track, 20.0, 50, 60));
    }

    /// Very long track, small percentage (10%), position barely above min_dur.
    #[test]
    fn test_should_scrobble_long_track_low_perdur() {
        // 1000s track, 10% = 100s, min_dur = 60s
        // position=100s → pct=10% ≥ 10%, and 100 ≥ 60 → true
        let track = make_track(1000);
        assert!(LastfmApi::should_scrobble(&track, 100.0, 10, 60));
    }
}
