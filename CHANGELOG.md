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
- Local-only corpus regression test (`cargo test --features corpus`)
  that pixel-compares the Rust decoder's output against reference
  PDFs produced by the Python `max2pdf.py` over a private archive.

### Verified
- Pixel-identical to the Python reference on the canonical 4-page
  test corpus, n_fail=0 across the board.
- ~4× faster end-to-end than the Python reference (38 ms/page vs
  151 ms/page on 2464×3508 scans).

### Pre-release fixes (caught by the corpus test, not the synthetic)
- Reverted incorrect "transcription correction" of `BLACK_TERM` runs
  26–29 (`0x4A`–`0x4D` → `0xCA`–`0xCD`). The synthetic 200×100
  round-trip never generates black runs of length 26–29, so the bug
  only surfaces on real text/form content.

[Unreleased]: https://github.com/creggch/vigb-decoder/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/creggch/vigb-decoder/releases/tag/v0.1.0
