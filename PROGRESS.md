# r-dlp progress

Updated: 2026-10-19 (Europe/Berlin)

## Current acceptance: PASS on PC0373

Final measured resolver performance (median, four videos, three rounds each):
- Cold: yt-dlp 3113.607 ms -> r-dlp 772.385 ms; 75.193% reduction.
- Warm: yt-dlp 1100.838 ms -> r-dlp 58.311 ms; 94.703% reduction.
- Each of the four videos individually exceeds 50% in both modes.
- All 48 comparison requests passed real HTTP 206 media checks (24 per tool).
- All four native URLs also passed actual ffmpeg audio decoding.
- Cold includes native visitor bootstrap when needed. Warm uses fresh network
  extraction in a retained process, with no signed-URL result cache.
- Scope: direct YouTube audio resolution in the PC0373 companion runtime.
  Full app click-to-play latency and production rollout are not measured here.
- Server opt-in integration is pushed on feat/r-dlp-audio, not activated in
  production by this task. Its local compile check passed.
- Source implementation: 98fb624; measured binary SHA-256 below.
- Evidence: benchmarks/2026-10-19-final.jsonl and
  benchmarks/2026-10-19-media-decode.jsonl. Failed intermediate runs are retained.

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

## Initial investigation status (historical)
- r-dlp directory existed and was empty; local main repository initialized.
- GitHub creation requested for glyphmusic/r-dlp (private by default); pending verification.
- SSH to PC0373 works. Production companion is a Linux Docker container.
- Installed production yt-dlp: 2026.08.19.
- Current audio contract: bestaudio[ext=m4a]/bestaudio[acodec=vorbis].
- Server already uses three persistent Python workers, fast extraction arguments,
  and fallback. Both ordinary CLI and persistent baseline need measuring.
- Upstream YouTube client policies inspected: legacy Android VR URLs can be
  rejected with HTTP 403. Returning a URL alone is insufficient evidence.

## Initial measurement state (historical)
Measurements were pending at repository creation. Final acceptance is above.

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

## Remote measurement started
- PC0373 Linux release build succeeded (Rust Docker builder; optimized binary).
- Test binary copied to /tmp/r-dlp in the existing companion runtime, with no
  replacement of production binaries, service restart or configuration change.
- Benchmark is running: four videos, three rounds per mode, interleaved runs,
  ordinary yt-dlp CLI and persistent YoutubeDL vs native CLI and persistent
  r-dlp, with a 1024-byte HTTP 206 check on every successful extraction.
- Raw redacted output will be retained in benchmarks/ after completion.

## First full run: FAILED (retained)
- Ordinary yt-dlp succeeded on all 12 cold and 12 warm requests.
- Initial native direct-only implementation succeeded on only 3/12 in each mode;
  therefore both acceptance gates FAILED despite low successful-request timings.
- Raw initial failures: benchmarks/2026-10-19-initial-failed.jsonl.
- Root cause established with remote A/B probe: adding anonymous visitor context
  obtained from the video webpage makes the rejected player call succeed.
- Implemented one bounded webpage bootstrap + player retry on LOGIN_REQUIRED;
  visitor context is retained in memory for warm requests. No user login/cookies.
- Three local tests pass, including escaped visitor-context extraction.
- Rebuilding native Linux binary before repeating the complete benchmark.

## Second run preparation
- Visitor-context fix is committed/pushed at 15e5383.
- Acceptance script now also calculates per-video median reductions for both
  cold and warm modes. Missing/failed samples cannot pass that per-video gate.
- Initial failed run remains checked in; it is not an accepted benchmark.
- Concurrent unrelated edits appeared in glyph-server after the opt-in commit;
  only ytdlp_pool.rs was committed for this task. Other edits are left untouched.

## Protocol test found another compatibility bug
- Remote protocol smoke recovered from invalid JSON/IDs correctly, but failed
  on the archived/live-content test video: isLiveContent was treated as live now.
- Corrected live detection to actual isLive/isLiveNow flags, allowing completed
  broadcasts with direct formats. Added archived/live assertions to unit tests.
- Second benchmark is diagnostic only; it cannot pass with that rejected video.
- Reusing the remote Rust builder cache for the corrected Linux binary and tests.

## Corrected Linux build and protocol checks: PASS
- PC0373 `cargo test --locked`: 3 unit tests passed, binary/doc-test targets passed.
- PC0373 optimized rebuild succeeded; source implementation is commit 98fb624.
- Real persistent-process smoke passed: readiness, recovery from malformed JSON
  and invalid IDs, two real video resolutions including the formerly rejected
  completed broadcast, EOF shutdown and oversized-frame rejection.
- Final media-decode checks and full interleaved benchmark are now running.

## Actual media decoding: PASS
- All four corpus videos decoded successfully on PC0373 with ffmpeg.
- Each native result produced 44,100 bytes of PCM from a 0.25-second segment.
- Tested Linux binary size: 3,377,664 bytes.
- SHA-256: 5fa85d9583c0945b96fc56d115dbd3e8ac7d7e88d84e5eb29014365918cd37c3.
- Local ready-to-use copy: dist/r-dlp-linux-x86_64 (not committed as source).
- Full final benchmark is still running; no final acceptance claim yet.
