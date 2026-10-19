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
            "r-dlp [--get-url | --verify] VIDEO\nr-dlp serve   # newline JSON {{\"id\":\"VIDEO_ID\"}} on stdin"
        );
        return Ok(());
    }
    let resolver = Resolver::new()?;
    if args == ["serve"] {
        println!(
            "{}",
            json!({"ready":true,"version":env!("CARGO_PKG_VERSION")})
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
            let result = match resolver.resolve(id) {
                Ok(audio) => serde_json::to_value(audio)?,
                Err(error) => json!({"id":id,"error":error.to_string()}),
            };
            println!("{result}");
            io::stdout().flush()?;
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
