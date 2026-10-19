//! Direct YouTube audio resolution. One client can serve many requests without
//! Python imports, a JS runtime, unrelated extractors, or a result cache.
use reqwest::{Url, blocking::Client};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

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

pub struct Resolver {
    client: Client,
    visitor: Mutex<Option<String>>,
}

impl Resolver {
    /// Construct once at companion startup to retain the connection pool.
    pub fn new() -> Result<Self, Error> {
        let mut builder = Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .pool_idle_timeout(Duration::from_secs(90))
            .redirect(reqwest::redirect::Policy::none());
        // Keep an explicit HTTP/1 control for same-host A/B measurements.
        if std::env::var("RDLP_HTTP1_ONLY").as_deref() == Ok("1") {
            builder = builder.http1_only();
        }
        Ok(Self {
            visitor: Mutex::new(None),
            client: builder.build()?,
        })
    }

    /// Resolve afresh: repeated calls do not reuse signed URLs or cached results.
    pub fn resolve(&self, input: &str) -> Result<Audio, Error> {
        self.resolve_with_timeout(input, Duration::from_secs(8))
    }

    /// One budget covers the player, visitor bootstrap and player retry.
    pub fn resolve_with_timeout(&self, input: &str, timeout: Duration) -> Result<Audio, Error> {
        self.resolve_at(
            input,
            timeout,
            "https://www.youtube.com/youtubei/v1/player?prettyPrint=false",
            "https://www.youtube.com/watch",
        )
    }

    fn resolve_at(
        &self,
        input: &str,
        timeout: Duration,
        player_url: &str,
        watch_url: &str,
    ) -> Result<Audio, Error> {
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or("Invalid resolution budget")?;
        let id = video_id(input)?;
        let visitor = self
            .visitor
            .lock()
            .map_err(|_| "Visitor state poisoned")?
            .clone();
        let mut info = self.player(&id, visitor.as_deref(), deadline, player_url)?;
        // Some videos require an anonymous visitor context. Bootstrap it only
        // when needed, retain it in memory, and retry once. No user cookies.
        if info["playabilityStatus"]["status"].as_str() == Some("LOGIN_REQUIRED") {
            let response = self
                .client
                .get(format!("{watch_url}?v={id}"))
                .timeout(remaining(deadline)?)
                .send()?
                .error_for_status()?;
            use std::io::Read;
            let mut page = String::new();
            response.take(4 * 1024 * 1024).read_to_string(&mut page)?;
            let visitor = extract_visitor(&page)?;
            *self.visitor.lock().map_err(|_| "Visitor state poisoned")? = Some(visitor.clone());
            info = self.player(&id, Some(&visitor), deadline, player_url)?;
        }
        remaining(deadline)?;
        select_audio(&id, &info)
    }

    /// Prime the actual player connection and anonymous visitor state. Failure
    /// is best effort: ordinary requests can still retry after startup.
    pub fn warmup(&self) -> Result<(), Error> {
        self.resolve_with_timeout("9bZkp7q19f0", Duration::from_secs(4))
            .map(|_| ())
    }

    fn player(
        &self,
        id: &str,
        visitor: Option<&str>,
        deadline: Instant,
        endpoint: &str,
    ) -> Result<Value, Error> {
        let mut client = json!({
            "clientName":"VISIONOS", "clientVersion":"1.02",
            "deviceMake":"Apple", "deviceModel":"RealityDevice17,1",
            "osName":"visionOS", "osVersion":"26.5.23O471", "hl":"en", "gl":"US",
            "userAgent":USER_AGENT
        });
        let mut request = self
            .client
            .post(endpoint)
            .header("X-Youtube-Client-Name", "101")
            .header("X-Youtube-Client-Version", "1.02")
            .header("Origin", "https://www.youtube.com");
        if let Some(visitor) = visitor {
            client["visitorData"] = json!(visitor);
            request = request.header("X-Goog-Visitor-Id", visitor);
        }
        let response = request.json(&json!({"context":{"client":client},
                "videoId":id,"contentCheckOk":true,"racyCheckOk":true,
                "playbackContext":{"contentPlaybackContext":{"html5Preference":"HTML5_PREF_WANTS"}}}))
            .timeout(remaining(deadline)?)
            .send()?
            .error_for_status()?;
        // Limit upstream metadata allocation, even if the remote endpoint misbehaves.
        use std::io::Read;
        let mut body = Vec::new();
        response.take(4 * 1024 * 1024 + 1).read_to_end(&mut body)?;
        if body.len() > 4 * 1024 * 1024 {
            return Err("Player response too large".into());
        }
        Ok(serde_json::from_slice(&body)?)
    }

    /// Verify that the same egress can read real media bytes. Separate from
    /// resolution latency so callers can measure either boundary explicitly.
    pub fn verify(&self, audio: &Audio) -> Result<usize, Error> {
        validate_audio_url(&audio.url)?;
        let response = self
            .client
            .get(&audio.url)
            .header("Range", "bytes=0-1023")
            .send()?
            .error_for_status()?;
        if response.status().as_u16() != 206 {
            return Err("Expected partial media response".into());
        }
        let mut data = Vec::new();
        use std::io::Read;
        response.take(1024).read_to_end(&mut data)?;
        if data.len() != 1024 {
            return Err("Incomplete media probe".into());
        }
        Ok(data.len())
    }
}

fn remaining(deadline: Instant) -> Result<Duration, Error> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| "Resolution deadline exceeded".into())
}

fn extract_visitor(page: &str) -> Result<String, Error> {
    let value = page
        .split_once("\"VISITOR_DATA\"")
        .and_then(|(_, rest)| rest.split_once(':'))
        .map(|(_, rest)| rest.trim_start())
        .ok_or("No anonymous visitor context")?;
    let visitor = serde_json::Deserializer::from_str(value)
        .into_iter::<String>()
        .next()
        .ok_or("Missing visitor value")??;
    if visitor.is_empty() || visitor.len() > 1024 || !visitor.is_ascii() {
        return Err("Invalid visitor context".into());
    }
    Ok(visitor)
}

pub fn video_id(input: &str) -> Result<String, Error> {
    let valid = |id: &str| {
        id.len() == 11
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    };
    if valid(input) {
        return Ok(input.into());
    }
    let url = Url::parse(input)?;
    if url.scheme() != "https" {
        return Err("Expected HTTPS YouTube URL".into());
    }
    let candidate = match url.host_str() {
        Some("youtu.be") => url.path().strip_prefix('/').map(str::to_owned),
        Some("youtube.com" | "www.youtube.com" | "music.youtube.com" | "m.youtube.com") => {
            if url.path() == "/watch" {
                url.query_pairs()
                    .find(|(k, _)| k == "v")
                    .map(|(_, v)| v.into_owned())
            } else {
                url.path()
                    .strip_prefix("/shorts/")
                    .or_else(|| url.path().strip_prefix("/embed/"))
                    .map(str::to_owned)
            }
        }
        _ => None,
    };
    candidate
        .filter(|id| valid(id))
        .ok_or_else(|| "Expected a YouTube video ID or URL".into())
}

fn validate_audio_url(value: &str) -> Result<(), Error> {
    let url = Url::parse(value)?;
    if value.len() > 60000
        || url.scheme() != "https"
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || !url
            .host_str()
            .is_some_and(|host| host.ends_with(".googlevideo.com"))
    {
        return Err("Invalid audio host".into());
    }
    Ok(())
}

fn select_audio(id: &str, info: &Value) -> Result<Audio, Error> {
    if info["playabilityStatus"]["status"].as_str() != Some("OK") {
        return Err(format!(
            "Unavailable: {}",
            info["playabilityStatus"]["reason"]
                .as_str()
                .unwrap_or("Player rejected video")
        )
        .into());
    }
    if info["videoDetails"]["videoId"].as_str() != Some(id) {
        return Err("Mismatched player video".into());
    }
    // isLiveContent also marks completed broadcasts with ordinary direct audio.
    if info["videoDetails"]["isLive"].as_bool() == Some(true)
        || info["microformat"]["playerMicroformatRenderer"]["liveBroadcastDetails"]["isLiveNow"]
            .as_bool()
            == Some(true)
    {
        return Err("Live content is not supported".into());
    }
    let formats = info["streamingData"]["adaptiveFormats"]
        .as_array()
        .ok_or("No direct formats")?;
    let format = formats
        .iter()
        .filter(|f| {
            let mime = f["mimeType"].as_str().unwrap_or("");
            !f["drmFamilies"].is_array()
                && f["signatureCipher"].is_null()
                && (mime.starts_with("audio/mp4;")
                    || (mime.starts_with("audio/webm;") && mime.contains("vorbis")))
                && f["url"]
                    .as_str()
                    .is_some_and(|u| validate_audio_url(u).is_ok())
                && f["audioTrack"]["audioIsDefault"].as_bool() != Some(false)
        })
        .max_by_key(|f| {
            (
                f["mimeType"]
                    .as_str()
                    .unwrap_or("")
                    .starts_with("audio/mp4;"),
                f["bitrate"].as_u64().unwrap_or(0),
            )
        })
        .ok_or("No compatible direct audio; use yt-dlp fallback")?;
    Ok(Audio {
        id: id.into(),
        title: info["videoDetails"]["title"].as_str().unwrap_or("").into(),
        url: format["url"].as_str().ok_or("Missing URL")?.into(),
        format_id: format["itag"].as_u64().ok_or("Missing format ID")?,
        mime_type: format["mimeType"].as_str().unwrap_or("").into(),
        bitrate: format["bitrate"].as_u64().unwrap_or(0),
        duration_seconds: format["approxDurationMs"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .map(|ms| ms / 1000.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhausted_budget_stops_before_network() {
        assert!(remaining(Instant::now() - Duration::from_millis(1)).is_err());
        let resolver = Resolver::new().unwrap();
        let error = resolver
            .resolve_with_timeout("9bZkp7q19f0", Duration::ZERO)
            .unwrap_err();
        assert_eq!(error.to_string(), "Resolution deadline exceeded");
    }

    #[test]
    fn visitor_body_read_uses_the_remaining_resolution_budget() {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let upstream = std::thread::spawn(move || {
            for (delay, body) in [
                (
                    Duration::from_millis(200),
                    r#"{"playabilityStatus":{"status":"LOGIN_REQUIRED"}}"#,
                ),
                (
                    Duration::from_millis(200),
                    r#"{"VISITOR_DATA":"test-visitor"}"#,
                ),
            ] {
                let accept_deadline = Instant::now() + Duration::from_secs(2);
                let (mut stream, _) = loop {
                    match listener.accept() {
                        Ok(connection) => break connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < accept_deadline, "Missing mock request");
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("Mock accept failed: {error}"),
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    assert!(
                        reader.read_line(&mut line).unwrap() > 0,
                        "Incomplete mock request"
                    );
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse::<usize>().unwrap();
                    }
                }
                reader.read_exact(&mut vec![0; length]).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.flush().unwrap();
                std::thread::sleep(delay);
                let _ = stream.write_all(body.as_bytes());
            }
        });
        let resolver = Resolver {
            client: Client::builder().no_proxy().build().unwrap(),
            visitor: Mutex::new(None),
        };
        let started = Instant::now();
        let budget = Duration::from_millis(300);
        let error = resolver
            .resolve_at("9bZkp7q19f0", budget, &endpoint, &endpoint)
            .unwrap_err();
        let elapsed = started.elapsed();
        upstream.join().unwrap();
        // Reqwest can wrap a body timeout as a decoding or IO error depending
        // on the transport. Assert behavior rather than that wrapper type.
        assert!(elapsed >= budget, "Unexpected early failure: {error}");
        // Each stage fits within 300 ms individually. Resetting the budget
        // would consume the visitor body and store its value before retrying.
        assert!(resolver.visitor.lock().unwrap().is_none());
    }
    #[test]
    fn reads_escaped_visitor_without_parsing_entire_webpage() {
        assert_eq!(
            extract_visitor(r#"<script>{"VISITOR_DATA": "abc\u003d", "other":1}</script>"#)
                .unwrap(),
            "abc="
        );
        assert!(extract_visitor(r#"{"VISITOR_DATA": 42}"#).is_err());
    }
    #[test]
    fn rejects_foreign_hosts_and_malformed_inputs() {
        assert_eq!(
            video_id("https://youtu.be/dQw4w9WgXcQ?t=3").unwrap(),
            "dQw4w9WgXcQ"
        );
        for input in [
            "https://youtube.com.evil/watch?v=dQw4w9WgXcQ",
            "--help",
            "https://example.com/dQw4w9WgXcQ",
        ] {
            assert!(video_id(input).is_err());
        }
        for url in [
            "http://a.googlevideo.com/a",
            "https://a.googlevideo.com.evil/a",
            "https://a.googlevideo.com:123/a",
        ] {
            assert!(validate_audio_url(url).is_err());
        }
    }
    #[test]
    fn preserves_codec_preference_and_ignores_nondefault_tracks() {
        let mut info = json!({"playabilityStatus":{"status":"OK"},"videoDetails":{"videoId":"dQw4w9WgXcQ"},"streamingData":{"adaptiveFormats":[
            {"itag":140,"mimeType":"audio/mp4; codecs=\"mp4a.40.2\"","bitrate":128000,"url":"https://a.googlevideo.com/a"},
            {"itag":141,"mimeType":"audio/mp4; codecs=\"mp4a.40.2\"","bitrate":256000,"audioTrack":{"audioIsDefault":false},"url":"https://a.googlevideo.com/b"},
            {"itag":251,"mimeType":"audio/webm; codecs=\"opus\"","bitrate":160000,"url":"https://a.googlevideo.com/c"}
        ]}});
        assert_eq!(select_audio("dQw4w9WgXcQ", &info).unwrap().format_id, 140);
        info["videoDetails"]["isLiveContent"] = json!(true);
        assert!(select_audio("dQw4w9WgXcQ", &info).is_ok());
        info["videoDetails"]["isLive"] = json!(true);
        assert!(select_audio("dQw4w9WgXcQ", &info).is_err());
        info["videoDetails"]["isLive"] = json!(false);
        info["videoDetails"]["videoId"] = json!("aaaaaaaaaaa");
        assert!(select_audio("dQw4w9WgXcQ", &info).is_err());
    }
}
