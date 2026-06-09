# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-06-09

### Added
- First public release.
- Library crate `vigb_decoder` with `decode_max`, `decode_max_file`,
  `write_pdf`, `Config`, `Page`, `MaxError`. `Config`, `Page`, and
  `Preview` are `#[non_exhaustive]`; construct `Config` via
  `Config::default()` / `Config::builder()` and obtain `Page` /
  `Preview` from the decoder functions.
- `vigb-max2pdf` binary with CLI flag parity vs the Python reference
  decoder (canonical fixes ON by default, diagnostic flags opt-in).
  Rust-only additions: `--stats` and `--max-pages`.
- Pure-Python sibling implementation
  (`python-reference/vigb_max2pdf.py`) — same algorithm, same
  canonical bit-perfect output, same CLI flags.
- Per-line CCITT-T.6 decoder with the bug4 canonical reference-table
  walk and lazy-bit-loading toggle.
- Preview thumbnail decoder (102×146 RLE → upscaled 1-bit), off by
  default; `--preview` appends the thumbnail as an extra PDF page per
  source page (fallback when the main CCITT decode fails on
  hand-drawn content or stamps).
- Hand-written PDF writer (no PDF crate dependency).
- Smart-resync state machine (`fail_resync_max` / `lookahead` /
  `min_confidence` / `budget`).
- CCITT-T.6 lookup tables derived from CCITT T.6 (1988) +
  TIFF 6.0 (Aldus, 1992) — clean-room from public standards.
- Local-only corpus regression test (`cargo test --features corpus`)
  that pixel-compares the Rust decoder's output against reference
  PDFs produced by `python-reference/vigb_max2pdf.py` over a private
  archive.

### Security
- Hardening against adversarial `.max` input, designed in from the
  start:
  - Image chunks shorter than `IMAGE_CHUNK_MIN_LEN` (`0x42`) are
    rejected at discovery time; preview decoding bails safely when
    `preview_size > chunk_length` would otherwise underflow
    (CRIT-01, SEC-H01).
  - `width × height` capped at `MAX_IMAGE_PIXELS` (200 MP) and
    `padded_x × preview_y` at `MAX_PREVIEW_PIXELS` (16 MP);
    `checked_mul` on the bitmap byte-count for 32-bit safety.
    Returns `MaxError::ImageTooLarge` instead of allocating ~537 MB /
    ~4 GB from a 64-byte malicious header (CRIT-02, SEC-H02).
  - `Config::fail_resync_max`, `fail_resync_lookahead`, and
    `fail_resync_budget` clamped at use site to safe upper bounds;
    `fail_resync_budget == 0` means "use the default cap of 1024"
    (SEC-M02; the Python reference keeps 0 = unlimited).
  - Per-chunk dispatcher bounded to `chunk_start + chunk_length`,
    closing a quadratic-CPU vector via crafted files packing many
    minimum-size image chunks back-to-back. Mirrored in the Python
    reference (SEC-M03).
  - `Config::max_pages` (default 1024, CLI `--max-pages`) caps the
    per-file image-chunk count, bounding resident memory on crafted
    many-chunk files; `MaxError::TooManyPages` (SEC-M04, Rust-only).
- The CLI's output-path trust model (it honours `-o` verbatim,
  including `..` traversal) is documented in `docs/cli.md` together
  with a canonicalize-and-contain recipe for service operators
  wrapping the binary on untrusted input.

### Verified
- Bit-perfect against PaperPort 3.6 (run under `otvdm`) on the
  author's 159-page private test corpus (median IoU = 1.000);
  pixel-identical to the Python reference, n_fail=0 across the board.
- ~4× faster end-to-end than the Python reference (38 ms/page vs
  151 ms/page on 2464×3508 scans).

[Unreleased]: https://github.com/christian-regg/vigb-decoder/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/christian-regg/vigb-decoder/releases/tag/v0.1.0
