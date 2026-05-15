# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `Config::max_pages` (default 1024) caps the per-file image-chunk
  count accepted by `decode_max`; new `MaxError::TooManyPages`
  variant; CLI flag `--max-pages` exposes the knob (raise for
  legitimate large scanned collections, lower for service
  deployments). SEC-M04.
- CLI: `--t0-drop-after-drift none` is now accepted alongside the
  historical empty-string form on both the Rust binary and the
  Python sibling. Matches `docs/cli.md` which documented `none`
  before either CLI accepted it.

### Changed
- **Breaking (pre-1.0):** `Config`, `Page`, and `Preview` are now
  marked `#[non_exhaustive]`. Out-of-crate callers must construct
  these types via decoder functions (`decode_max`, `decode_max_file`)
  or via `Config::default` / `Config::builder` — not struct-literal
  syntax. Allows future fields to be added without a semver-breaking
  change and prevents adversarial-`Page` paths into `write_pdf`
  (e.g., `dpi_x = 0`, `row_bytes = u32::MAX`).
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
- `docs/cli.md` gains a "Security notes" section documenting the
  CLI's output-path trust model (it honours `-o` verbatim, including
  `..` traversal) and provides a canonicalize-and-contain recipe for
  service operators wrapping the binary on untrusted input. No code
  change — interactive `-o ../foo` workflows are intentionally
  supported.
- `Config::fail_resync_budget` docstring, `--fail-resync-budget` CLI
  help, and `docs/cli.md` updated to accurately describe the SEC-M02
  cap. The code already capped `0` at 1024 since SEC-M02 landed; the
  docs continued to claim "unlimited" until now. No behaviour change.

### Removed
- **Breaking (pre-1.0):** `MaxError::BitUnderrun` variant. It was
  public but never constructed — soft underruns flow through
  `DecodeStats::n_fail` via the FAIL sentinel.

### Security
- Bound the per-chunk dispatcher to `chunk_start + chunk_length`
  instead of `data.len()`. Closes a quadratic-CPU vector via crafted
  files packing many minimum-size (0x42-byte) image chunks back-to-back;
  the previous bound allowed each chunk's dispatch loop to scan into
  every later chunk's bytes, producing roughly O(N²) work in the
  chunk count. Mirrored in the Python reference at
  `python-reference/vigb_max2pdf.py`. SEC-M03.
- Cap per-file image-chunk count via `Config::max_pages` (default
  1024). Each decoded `Page` allocates up to `MAX_IMAGE_PIXELS / 8`
  (~25 MiB) of bitmap and is retained in memory until `decode_max`
  returns; without the cap a crafted file with N chunks could request
  `N × 25 MiB` resident memory. SEC-M04. Rust-only hardening (Python
  reference has no analogue, matching the SEC-M02 pattern).
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
