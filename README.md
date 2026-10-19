# r-dlp

Native Rust YouTube audio resolver for Glyph companion. This implements the
small direct-audio subset Glyph needs, rather than all of yt-dlp.
Progress and actual acceptance status: [PROGRESS.md](PROGRESS.md).

## Use

```sh
cargo build --release --locked
r-dlp dQw4w9WgXcQ                  # metadata and signed audio URL as JSON
r-dlp --get-url dQw4w9WgXcQ        # URL only
r-dlp --verify dQw4w9WgXcQ         # also read 1024 real audio bytes
r-dlp serve                       # persistent JSON-lines stdin/stdout
```

`serve` immediately emits `{"ready":true,"version":"0.1.0"}`. Send one
`{"id":"dQw4w9WgXcQ"}` line per request; each response contains the same ID
and either `url` plus metadata or `error`. A process handles requests serially;
start several for concurrent extraction. Closing stdin shuts it down.

The library's `Resolver::new()` creates a reusable HTTP client; call
`Resolver::resolve()` on a blocking worker thread. This avoids Python import,
general CLI parsing, unconditional webpage/config downloads, JavaScript startup and unrelated
extractors. HTTP keepalive and TLS state persist between requests. There is no
signed-URL result cache in this resolver.

When the player requires anonymous visitor context, the resolver fetches one
bounded video webpage and retries once, retaining that visitor context in memory.
This adds work to some cold calls; the benchmark includes that cost.

## Compatibility

Selection matches Glyph's `bestaudio[ext=m4a]/bestaudio[acodec=vorbis]`
preference: highest bitrate direct MP4 audio first, then Vorbis. Opus remains
excluded to match the existing companion contract. Nondefault translated tracks,
encrypted formats and non-Googlevideo URLs are excluded.

Initial target: Linux x86_64 companion on PC0373. Windows compilation is checked
locally. Other devices and platforms have not been validated. Native HTTP uses
Rustls bundled roots; no Python, ffmpeg or Deno is required for resolution.

Not implemented: search, playlists, subtitles, downloads/remux, other sites,
authenticated/age-restricted videos, made-for-kids support, live streams,
signature deciphering, or proof-of-origin token generation. The VISIONOS player
client may stop working when YouTube changes. Keep yt-dlp as the fallback for
unsupported/rejected responses and media failures. Signed URLs must be consumed
from the same public IP that resolves them and before expiry.

## Companion integration

The `glyphmusic/server` branch `feat/r-dlp-audio` adds an opt-in worker backend.
Install the Linux binary in the image and set:

```sh
GLYPH_RDLP_PATH=/usr/local/bin/r-dlp
```

The existing three workers prestart `r-dlp serve`, retain cancellation cleanup,
validate returned hosts/IDs, and use the existing yt-dlp command fallback when
resolution fails. Search/metadata continue through yt-dlp. Unset the variable
and restart the test instance to restore Python workers. Production rollout
is separate from resolver benchmarks; see progress for what was actually tested.

## Reproduce benchmarks

Build using the included Dockerfile on PC0373. Copy the resulting Linux binary
to `/tmp/r-dlp` in the companion test runtime, and execute
`python3 scripts/benchmark.py` there. The script needs that runtime's installed
yt-dlp, Python and JS runtime for the baseline. It records versions, binary
SHA-256, process definitions, per-request timings, codec IDs and actual HTTP 206
media checks, without recording signed URLs, credentials or public IPs.

Cold is a fresh process, with the OS filesystem cache left intact. Warm is a
reused process and a new network extraction on every request. Both implementations
run interleaved over the same videos and network. A success gate requires all
audio probes to pass and at least 50% lower median resolution time in both modes.
These measurements do not imply a 50% reduction in the complete app playback
path, which also includes Worker, caching, transport and decoding.

## Upstream research

Studied [yt-dlp](https://github.com/yt-dlp/yt-dlp), specifically
[`_base.py`](https://github.com/yt-dlp/yt-dlp/blob/897341241f2bfb2439fe0367f4c7c9ccd29a3828/yt_dlp/extractor/youtube/_base.py)
and [`_video.py`](https://github.com/yt-dlp/yt-dlp/blob/897341241f2bfb2439fe0367f4c7c9ccd29a3828/yt_dlp/extractor/youtube/_video.py)
at commit `897341241f2bfb2439fe0367f4c7c9ccd29a3828` on 2026-10-19.
Client context values and format behavior informed this focused implementation.
The upstream client-policy comments also explain why older Android VR shortcuts
are unsuitable: they can return signed URLs that fail with HTTP 403.

License: MIT; upstream yt-dlp is separately distributed under its own license.
