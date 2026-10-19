# Changelog

## 0.1.1 - 2026-10-19

- Enable HTTP/2 negotiation, with HTTP/1.1 fallback and an HTTP/1-only benchmark control.
- Add bounded `serve --warmup` to prime connections and anonymous visitor context.
- Apply one eight-second resolution budget across player, visitor bootstrap and retry.
- Add caller-defined resolution budgets and regression coverage for stalled response bodies.
- Add a transport A/B benchmark with real audio-range validation and redacted output.
- Document coordinated installation with the updated Glyph server worker integration.

## 0.1.0 - 2026-10-19

- Native direct YouTube audio resolver with compatible MP4/Vorbis selection.
- Reusable HTTP client and persistent JSON-lines worker mode.
- Anonymous visitor bootstrap on rejected player requests.
- Strict input/host validation, bounded responses and request frames.
- Reproducible PC0373 cold/warm benchmark with real media range checks.
- Companion integration available on the separate server feat/r-dlp-audio branch.
