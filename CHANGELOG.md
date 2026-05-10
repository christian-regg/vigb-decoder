# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-05-10

### Added
- First public release.
- Library crate `vigb_decoder` with `decode_max`, `decode_max_file`,
  `write_pdf`, `Config`, `Page`, `MaxError`.
- `max2pdf` binary with full CLI flag parity vs the Python reference
  decoder (canonical fixes ON by default, diagnostic flags opt-in).
- Per-line CCITT-T.6 decoder with the bug4 canonical reference-table
  walk and lazy-bit-loading toggle.
- Preview thumbnail decoder (102×146 RLE → upscaled 1-bit).
- Hand-written PDF writer (no PDF crate dependency).
- Smart-resync state machine (`fail_resync_max` / `lookahead` /
  `min_confidence` / `budget`).
- CCITT-T.6 lookup tables derived from CCITT T.6 (1988) +
  TIFF 6.0 (Aldus, 1992) — clean-room from public standards.

[Unreleased]: https://github.com/creggch/vigb-decoder/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/creggch/vigb-decoder/releases/tag/v0.1.0
