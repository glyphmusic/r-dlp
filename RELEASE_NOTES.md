# r-dlp 0.1.1

Connection and startup improvements for the native YouTube audio resolver:

- HTTP/2 negotiation with HTTP/1.1 fallback. Set `RDLP_HTTP1_ONLY=1` for comparison runs.
- `r-dlp serve --warmup` primes the player connection and visitor context before readiness, with a four-second best-effort budget.
- One eight-second resolution budget now covers the player request, visitor webpage and player retry. Library callers can use `resolve_with_timeout()`.
- New transport benchmark interleaves both modes and checks real audio bytes without logging signed URLs.

Local Windows validation: five resolver tests passed and all 24 transport comparison requests passed real media-range checks. Median warm resolution was 63 ms with normal protocol negotiation versus 65 ms with forced HTTP/1.1. This small sample does not establish a meaningful HTTP/2 speed improvement or end-to-end playback improvement; the negotiated arm allows HTTP/1.1 fallback.

The companion changes for next-available-worker scheduling and retaining native workers beyond 100 requests live in the separate Glyph server repository. They require an updated server deployment; publishing this resolver release does not deploy that integration. Existing `serve` callers remain compatible, and updated servers can use `serve --warmup` with this version.
