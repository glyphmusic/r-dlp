# Security policy

## Supported versions

Security fixes ship on `main` and in the latest signed release. Every release
includes `SHA256SUMS.txt` and `SHA256SUMS.txt.minisig`, signed with the Glyph
release key (`release-key.pub`).

| Version | Supported |
| --- | --- |
| Latest release (`0.2.x`) / `main` | Yes |
| Older releases | No — update to the latest release |

## Reporting a vulnerability

Report security vulnerabilities privately through
**[GitHub Security Advisories](https://github.com/glyphmusic/r-dlp/security/advisories/new)**
or the contact in [glyph.my/.well-known/security.txt](https://glyph.my/.well-known/security.txt).

Do **not** open a public issue or pull request for a security problem.

Please include:

- the `r-dlp` version or commit hash,
- the input or response payload that triggers the issue,
- the impact on the resolver process or calling companion service.

## Scope

### In scope

- Host or URL validation bypasses (returning non-Googlevideo media URLs, server-side request forgery, redirect traversal to local or private networks)
- JSON-lines framing injection, memory safety issues or unbounded resource consumption in `r-dlp serve` or player/webpage parsing
- Release signature verification bypasses (`release-key.pub`, `scripts/sign-release.sh`)

### Out of scope

- Upstream YouTube player changes that stop resolution without a security impact (open a regular issue instead)
