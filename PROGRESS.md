# r-dlp progress

Updated: 2026-10-19 (Europe/Berlin)

## Goal and acceptance
Native Rust implementation of the YouTube audio subset used by Glyph companion.
Cold and warm resolution must each be at least 50% faster than ordinary yt-dlp
on PC0373, with usable audio verified from that same machine. No success claim
until measurements establish both thresholds. Existing production services stay available.

## Plan
1. Inspect companion call contracts and current yt-dlp implementation.
2. Record baseline on PC0373: fresh process, persistent process, multiple songs,
   same network and format selection, plus actual audio range reads.
3. Implement focused Rust library/CLI and persistent request mode. Keep HTTP
   connections and extractor state reusable. Explicit errors for unsupported features.
4. Build and test in isolation on PC0373; compare paired runs and retain raw evidence.
5. Prepare companion integration on a separate branch, document limitations.

## Current status
- r-dlp directory existed and was empty; local main repository initialized.
- GitHub creation requested for glyphmusic/r-dlp (private by default); pending verification.
- SSH to PC0373 works. Production companion is a Linux Docker container.
- Installed production yt-dlp: 2026.08.19.
- Current audio contract: bestaudio[ext=m4a]/bestaudio[acodec=vorbis].
- Server already uses three persistent Python workers, fast extraction arguments,
  and fallback. Both ordinary CLI and persistent baseline need measuring.
- Upstream YouTube client policies inspected: legacy Android VR URLs can be
  rejected with HTTP 403. Returning a URL alone is insufficient evidence.

## Measurements
Pending. No performance improvement has been established.

## Decisions and scope
- Initial target: Linux x86_64 companion on PC0373.
- Prioritize direct YouTube audio URLs with the existing compatible codec filter.
- Avoid importing thousands of unrelated extractors, postprocessing and CLI options.
- Do not replace or restart the production companion during experiments.

## Implementation update
- GitHub repository created: https://github.com/glyphmusic/r-dlp (private).
- Isolated direct VISIONOS player probe on PC0373: 0.213 s to metadata;
  real Googlevideo range returned HTTP 206 and 1024 bytes.
- Native Rust implementation now includes strict ID/URL validation, bounded
  metadata and JSON frames, codec/default-track selection, media probe,
  reusable HTTP pool, library API, CLI and persistent stdin/stdout mode.
- No result cache: warm benchmarks must perform a new upstream request.
- Local tests/build and multi-song remote baseline are next.

## Build/test update
- Initial source committed and pushed to main in the newly created repository.
- Remote Docker build hit Windows SSH credential-helper error (logon session).
  Investigating an isolated Docker client config for anonymous public image pulls;
  production configuration and running containers are unchanged.
- Local Rust compilation is running; remote performance results still pending.

## Verification update
- Local `cargo test`: 2 tests passed (host/input validation and compatible/default
  audio selection), plus binary/doc-test targets passed.
- Server branch `feat/r-dlp-audio` adds opt-in GLYPH_RDLP_PATH worker startup;
  existing yt-dlp fallback remains. Local native server `cargo check` passed
  with two pre-existing unused-function warnings.
- Remote public image pulls work using isolated Docker auths config. Production
  Docker credentials were not changed. Linux release build is underway.
- README documents supported subset, upstream commit, persistent protocol,
  failure/fallback boundaries, and exact cold/warm measurement definitions.
