# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Breaking (pre-1.0):** preview-thumbnail embedding is now off by
  default. The 102×146 grayscale preview was historically appended as
  an extra PDF page per source page so users still got *something* when
  the main CCITT decode failed; with the canonical decoder now bit-perfect
  on the corpus the preview is a redundant low-res duplicate. Pass
  `--preview` (Rust binary or Python sibling) to opt back in.
  `Config::default().embed_preview` is now `false`; the CLI flag flipped
  from `--no-preview` (opt-out) to `--preview` (opt-in).
- Renamed Rust binary `max2pdf` → `vigb-max2pdf` and Python sibling
  `python-reference/max2pdf.py` → `python-reference/vigb_max2pdf.py` to
  disambiguate from the unrelated GPL `orangeturtle739/max2pdf` project
  (LEGAL-S01).

### Security
- Reject image chunks shorter than `IMAGE_CHUNK_MIN_LEN` (`0x42`) at
  discovery time and bail safely from preview decoding when
  `preview_size > chunk_length` would otherwise underflow (CRIT-01,
  SEC-H01).
- Cap `width × height` at `MAX_IMAGE_PIXELS` (200 MP) and
  `padded_x × preview_y` at `MAX_PREVIEW_PIXELS` (16 MP); use
  `checked_mul` on the bitmap byte-count for 32-bit safety. Returns
  new `MaxError::ImageTooLarge` instead of allocating ~537 MB / ~4 GB
  from a 64-byte malicious header (CRIT-02, SEC-H02).
- Clamp `Config::fail_resync_max`, `fail_resync_lookahead`, and
  `fail_resync_budget` at use site to safe upper bounds. Pre-cap a
  pathological config could drive ~16 quintillion CCITT decode calls
  per FAIL event (SEC-M02). `fail_resync_budget == 0` semantics
  changed from "unlimited" to "use cap" (= 1024) — harmless for
  real workloads.

## [0.1.0] — 2026-05-10

### Added
- First public release.
- Library crate `vigb_decoder` with `decode_max`, `decode_max_file`,
  `write_pdf`, `Config`, `Page`, `MaxError`.
- `vigb-max2pdf` binary with full CLI flag parity vs the Python
  reference decoder (canonical fixes ON by default, diagnostic flags
  opt-in).
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
  PDFs produced by `python-reference/vigb_max2pdf.py` over a private
  archive.

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
