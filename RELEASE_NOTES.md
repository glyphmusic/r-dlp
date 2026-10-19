# r-dlp 0.1.0

Focused native Rust YouTube audio resolver for Glyph companion, with reusable HTTP connections, anonymous visitor bootstrap and a persistent JSON-lines worker mode.

Measured on PC0373 against yt-dlp 2026.08.19: median cold resolution 3.114 s to 0.772 s (75.2% lower), warm 1.101 s to 0.058 s (94.7% lower). Four videos, three rounds per mode; all 48 comparison requests passed audio-range checks and all four native results decoded successfully.

Supports direct MP4 audio with Vorbis fallback. Search, authenticated content, other sites, live streams and signature deciphering stay with yt-dlp. Measurements cover resolver latency, not full application playback startup. See README.md and PROGRESS.md for scope and raw evidence.
