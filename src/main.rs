use r_dlp::{Error, Resolver};
use serde_json::json;
use std::io::{self, BufRead, Write};

fn main() {
    if let Err(error) = run() {
        eprintln!("r-dlp: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--version"] {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.is_empty() || args == ["--help"] {
        println!(
            "r-dlp [--get-url | --verify | --video [--max-height N] [--get-url]] VIDEO\nr-dlp serve [--warmup]   # newline JSON {{\"id\":\"VIDEO_ID\",\"kind\":\"audio|video\",\"max_height\":720}} on stdin"
        );
        return Ok(());
    }
    let resolver = Resolver::new()?;
    if args == ["serve"] || args == ["serve", "--warmup"] {
        if args.len() == 2 {
            if let Err(error) = resolver.warmup() {
                eprintln!("r-dlp: warmup skipped: {error}");
            }
        }
        println!(
            "{}",
            json!({"ready":true,"version":env!("CARGO_PKG_VERSION"),"capabilities":["audio","video"]})
        );
        io::stdout().flush()?;
        let mut input = io::stdin().lock();
        loop {
            // Bound each frame and stop on oversized input instead of losing framing.
            let mut line = Vec::new();
            use std::io::Read;
            let bytes = input.by_ref().take(4097).read_until(b'\n', &mut line)?;
            if bytes == 0 {
                break;
            }
            if line.len() > 4096 || !line.ends_with(b"\n") {
                return Err("Invalid request frame".into());
            }
            let request: serde_json::Value = match serde_json::from_slice(&line) {
                Ok(request) => request,
                Err(_) => {
                    println!("{}", json!({"error":"Invalid JSON"}));
                    io::stdout().flush()?;
                    continue;
                }
            };
            let id = request["id"].as_str().unwrap_or("");
            let is_video = request["kind"].as_str() == Some("video");
            let result = if is_video {
                let max_h = request["max_height"].as_u64().map(|n| n.min(2160) as u32);
                match resolver.resolve_video(id, max_h) {
                    Ok(video) => serde_json::to_value(video)?,
                    Err(error) => json!({"kind":"video","id":id,"error":error.to_string()}),
                }
            } else {
                match resolver.resolve(id) {
                    Ok(audio) => serde_json::to_value(audio)?,
                    Err(error) => json!({"id":id,"error":error.to_string()}),
                }
            };
            println!("{result}");
            io::stdout().flush()?;
        }
    } else if args.first().map(String::as_str) == Some("--video") {
        let mut get_url = false;
        let mut max_height: Option<u32> = None;
        let mut input: Option<&str> = None;
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--get-url" => get_url = true,
                "--max-height" => {
                    i += 1;
                    let raw = args.get(i).ok_or("Missing value for --max-height")?;
                    max_height = Some(raw.parse::<u32>().map_err(|_| "Invalid --max-height")?);
                }
                v if !v.starts_with('-') && input.is_none() => input = Some(v),
                _ => return Err("Unsupported arguments; see --help".into()),
            }
            i += 1;
        }
        let video = resolver.resolve_video(input.ok_or("Missing VIDEO")?, max_height)?;
        if get_url {
            println!("{}", video.url);
        } else {
            println!("{}", serde_json::to_string(&video)?);
        }
    } else {
        let (flag, input) = match args.as_slice() {
            [input] => ("", input),
            [flag, input] if flag == "--get-url" || flag == "--verify" => (flag.as_str(), input),
            _ => return Err("Unsupported arguments; see --help".into()),
        };
        let audio = resolver.resolve(input)?;
        if flag == "--verify" {
            resolver.verify(&audio)?;
        }
        if flag == "--get-url" {
            println!("{}", audio.url);
        } else {
            println!("{}", serde_json::to_string(&audio)?);
        }
    }
    Ok(())
}
