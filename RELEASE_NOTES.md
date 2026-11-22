# r-dlp 0.2.3 - H.264 MP4 video stream resolution

- Added `Resolver::resolve_video`, the `--video [--max-height N] [--get-url]` CLI mode, and `{"kind":"video","max_height":...}` requests in `r-dlp serve` (with `"capabilities":["audio","video"]`) to resolve playable H.264 MP4 video streams and matching AAC audio in a single YouTube player call.
