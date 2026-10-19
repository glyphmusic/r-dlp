//! Direct YouTube audio resolution. One client can serve many requests without
//! Python imports, a JS runtime, unrelated extractors, or a result cache.
use reqwest::{Url, blocking::Client};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Duration;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";

#[derive(Debug, Serialize)]
pub struct Audio {
    pub id: String,
    pub title: String,
    pub url: String,
    pub format_id: u64,
    pub mime_type: String,
    pub bitrate: u64,
    pub duration_seconds: Option<f64>,
}

pub struct Resolver { client: Client }

impl Resolver {
    /// Construct once at companion startup to retain the connection pool.
    pub fn new() -> Result<Self, Error> {
        Ok(Self { client: Client::builder()
            .user_agent(USER_AGENT).connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15)).pool_idle_timeout(Duration::from_secs(90))
            .redirect(reqwest::redirect::Policy::none()).build()? })
    }

    /// Resolve afresh: repeated calls do not reuse signed URLs or cached results.
    pub fn resolve(&self, input: &str) -> Result<Audio, Error> {
        let id = video_id(input)?;
        let response = self.client.post("https://www.youtube.com/youtubei/v1/player?prettyPrint=false")
            .header("X-Youtube-Client-Name", "101")
            .header("X-Youtube-Client-Version", "1.02")
            .json(&json!({"context":{"client":{
                "clientName":"VISIONOS", "clientVersion":"1.02",
                "deviceMake":"Apple", "deviceModel":"RealityDevice17,1",
                "osName":"visionOS", "osVersion":"26.5.23O471", "hl":"en", "gl":"US"
            }},"videoId":id,"contentCheckOk":true,"racyCheckOk":true}))
            .send()?.error_for_status()?;
        // Limit upstream metadata allocation, even if the remote endpoint misbehaves.
        use std::io::Read;
        let mut body = Vec::new();
        response.take(4 * 1024 * 1024 + 1).read_to_end(&mut body)?;
        if body.len() > 4 * 1024 * 1024 { return Err("Player response too large".into()); }
        let info: Value = serde_json::from_slice(&body)?;
        select_audio(&id, &info)
    }

    /// Verify that the same egress can read real media bytes. Separate from
    /// resolution latency so callers can measure either boundary explicitly.
    pub fn verify(&self, audio: &Audio) -> Result<usize, Error> {
        validate_audio_url(&audio.url)?;
        let response = self.client.get(&audio.url).header("Range", "bytes=0-1023")
            .send()?.error_for_status()?;
        if response.status().as_u16() != 206 { return Err("Expected partial media response".into()); }
        let mut data = Vec::new();
        use std::io::Read;
        response.take(1024).read_to_end(&mut data)?;
        if data.len() != 1024 { return Err("Incomplete media probe".into()); }
        Ok(data.len())
    }
}

pub fn video_id(input: &str) -> Result<String, Error> {
    let valid = |id: &str| id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if valid(input) { return Ok(input.into()); }
    let url = Url::parse(input)?;
    if url.scheme() != "https" { return Err("Expected HTTPS YouTube URL".into()); }
    let candidate = match url.host_str() {
        Some("youtu.be") => url.path().strip_prefix('/').map(str::to_owned),
        Some("youtube.com" | "www.youtube.com" | "music.youtube.com" | "m.youtube.com") => {
            if url.path() == "/watch" { url.query_pairs().find(|(k, _)| k == "v").map(|(_, v)| v.into_owned()) }
            else { url.path().strip_prefix("/shorts/").or_else(|| url.path().strip_prefix("/embed/")).map(str::to_owned) }
        }
        _ => None,
    };
    candidate.filter(|id| valid(id)).ok_or_else(|| "Expected a YouTube video ID or URL".into())
}

fn validate_audio_url(value: &str) -> Result<(), Error> {
    let url = Url::parse(value)?;
    if value.len() > 60000 || url.scheme() != "https" || url.port_or_known_default() != Some(443)
        || !url.username().is_empty() || url.password().is_some()
        || !url.host_str().is_some_and(|host| host.ends_with(".googlevideo.com")) {
        return Err("Invalid audio host".into());
    }
    Ok(())
}

fn select_audio(id: &str, info: &Value) -> Result<Audio, Error> {
    if info["playabilityStatus"]["status"].as_str() != Some("OK") {
        return Err(format!("Unavailable: {}", info["playabilityStatus"]["reason"].as_str().unwrap_or("Player rejected video")).into());
    }
    if info["videoDetails"]["videoId"].as_str() != Some(id) { return Err("Mismatched player video".into()); }
    if info["videoDetails"]["isLiveContent"].as_bool() == Some(true) { return Err("Live content is not supported".into()); }
    let formats = info["streamingData"]["adaptiveFormats"].as_array().ok_or("No direct formats")?;
    let format = formats.iter().filter(|f| {
        let mime = f["mimeType"].as_str().unwrap_or("");
        !f["drmFamilies"].is_array() && f["signatureCipher"].is_null()
            && (mime.starts_with("audio/mp4;") || (mime.starts_with("audio/webm;") && mime.contains("vorbis")))
            && f["url"].as_str().is_some_and(|u| validate_audio_url(u).is_ok())
            && f["audioTrack"]["audioIsDefault"].as_bool() != Some(false)
    }).max_by_key(|f| (f["mimeType"].as_str().unwrap_or("").starts_with("audio/mp4;"), f["bitrate"].as_u64().unwrap_or(0)))
        .ok_or("No compatible direct audio; use yt-dlp fallback")?;
    Ok(Audio {
        id: id.into(), title: info["videoDetails"]["title"].as_str().unwrap_or("").into(),
        url: format["url"].as_str().ok_or("Missing URL")?.into(),
        format_id: format["itag"].as_u64().ok_or("Missing format ID")?,
        mime_type: format["mimeType"].as_str().unwrap_or("").into(),
        bitrate: format["bitrate"].as_u64().unwrap_or(0),
        duration_seconds: format["approxDurationMs"].as_str().and_then(|s| s.parse::<f64>().ok()).map(|ms| ms / 1000.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_foreign_hosts_and_malformed_inputs() {
        assert_eq!(video_id("https://youtu.be/dQw4w9WgXcQ?t=3").unwrap(), "dQw4w9WgXcQ");
        for input in ["https://youtube.com.evil/watch?v=dQw4w9WgXcQ", "--help", "https://example.com/dQw4w9WgXcQ"] { assert!(video_id(input).is_err()); }
        for url in ["http://a.googlevideo.com/a", "https://a.googlevideo.com.evil/a", "https://a.googlevideo.com:123/a"] { assert!(validate_audio_url(url).is_err()); }
    }
    #[test]
    fn preserves_codec_preference_and_ignores_nondefault_tracks() {
        let mut info = json!({"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ"},"streamingData":{"adaptiveFormats":[
            {"itag":140,"mimeType":"audio/mp4; codecs=\"mp4a.40.2\"","bitrate":128000,"url":"https://a.googlevideo.com/a"},
            {"itag":141,"mimeType":"audio/mp4; codecs=\"mp4a.40.2\"","bitrate":256000,"audioTrack":{"audioIsDefault":false},"url":"https://a.googlevideo.com/b"},
            {"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":160000,"url":"https://a.googlevideo.com/c"}
        ]}});
        assert_eq!(select_audio("dQw4w9WgXcQ", &info).unwrap().format_id, 140);
        info["videoDetails"]["videoId"] = json!("aaaaaaaaaaa");
        assert!(select_audio("dQw4w9WgXcQ", &info).is_err());
    }
}
