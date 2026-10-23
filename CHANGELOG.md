# Changelog

## Unreleased

### Changed

- Releases are signed: `SHA256SUMS.txt` comes with `SHA256SUMS.txt.minisig`, a signature by the Glyph release key for this exact version. The Glyph companion installs r-dlp updates only with it.

## 0.1.1 - 2026-10-19

### Changed

- Faster connections to YouTube with HTTP/2 (HTTP/1.1 still works as a fallback).
- `serve --warmup` gets connections ready before the first request.
- A single eight-second time limit per song, so a stuck request never hangs.

## 0.1.0 - 2026-10-19

### Added

- The first release: a small, fast tool that finds the direct audio stream of a YouTube video, built to run inside the Glyph companion.
