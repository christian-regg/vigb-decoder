---
date: 2026-05-10
topic: Rust port of the PaperPort 2 (.max) decoder
status: design (approved 2026-05-10 after legal + demand research)
crate: vigb-decoder
binary: max2pdf
---

# Rust port of `max2pdf.py` — design

## Goal

Port the canonical Python decoder (`max2pdf.py`, 1197 lines, IoU=1.000
on the test corpus) to Rust as a library + CLI crate published to
crates.io. Outcomes:

- `cargo install vigb-decoder` installs a `max2pdf` binary that
  replaces the Python tool as the user's daily driver — no Python
  runtime needed, single static binary, faster.
- `cargo add vigb-decoder` lets others build on the decoder
  programmatically (PNG export, batch tooling, web service, etc.).
- Public GitHub repo with CI, docs, and the format reverse-engineering
  notes that someone hitting a `.max` file in 2030 can find via search.

## Naming and legal posture (decided 2026-05-10)

- **Crate name `vigb-decoder`**, not `paperport-max`. "PaperPort" is
  a live trademark (Tungsten Automation actively sells PaperPort 14;
  USPTO Reg. 2189867 et al). Using the mark as the dominant element
  of *our* product name is the use most likely to trigger a
  cease-and-desist. `vigb-decoder` uses the four-byte file magic
  ("ViGBe"), is format-accurate, neutral, unambiguous. The README
  freely uses "decoder for PaperPort 2 (`.max`) files" — that's
  classic nominative fair use.
- **Binary stays `max2pdf`** to match Python muscle memory.
- **CCITT-T.6 tables re-derived from the ITU-T T.6 specification**,
  not from `__pp_src__/paperman_btab.dat`. paperman is GPL-2-or-later;
  copying its lookup table values into a permissive crate risks
  copyleft contamination of downstream consumers. Tables-as-facts
  (ITU standard values) is the strongest legal position. A
  `docs/provenance.md` documents clean-room separation: which
  artefacts came from the spec, which from the MAXKER2.DLL disasm,
  which from the developer's own bit-traces, vs. which OSS projects
  were only used as cross-checks (paperman, max2pdf — both credited
  in `docs/credits.md`).
- **README includes a "Reverse-engineering legal basis" section**
  citing Swiss URG Art. 21, EU Software Directive 2009/24/EC Art. 6,
  and US DMCA §1201(f) + Sega v. Accolade, 977 F.2d 1510 (9th Cir.
  1992). States explicitly: "Decoder ships zero bytes from PaperPort."
- **No MAXKER2.DLL bytes in the repo.** Pre-publish check:
  `git grep -i maxker` returns nothing; binary scan of
  `tests/fixtures/` confirms no PaperPort binaries.

## Demand context (research 2026-05-10)

Modest but real. Estimated ~50–500 useful users per year, almost all
one-off batch recovery (genealogy, estate paperwork, legacy admin
archives). Essentially zero ongoing daily users. This is *archival
recovery*, not a workflow tool. The Rust crate would be the **first
known decoder that handles PaperPort 2 era files** (`paperman` and
`max2pdf` both explicitly do not). Commercial validation:
DiskTransfer.co.uk charges for PP1/2/3 recovery.

**Marketing channels documented for v0.1 release** (in
`docs/release-checklist.md`, not yet written):

1. Add a Software entry to JustSolve / fileformats.archiveteam.org
   wiki at the [PaperPort (MAX) page](http://fileformats.archiveteam.org/wiki/PaperPort_(MAX)).
2. Open an issue on [sjg20/paperman](https://github.com/sjg20/paperman)
   linking to the new tool (Simon Glass is active; it's the only
   PP2 follow-up channel on GitHub).
3. One-line "FYI, here's a Rust tool that handles PP2" replies on
   the live forum threads (BleepingComputer, Microsoft Q&amp;A,
   WindowsBBS, Linux Mint Forums, Tech Support Guy, XnView). These
   still rank on Google for "open .max file".
4. Single posts to r/DataHoarder, r/datarecovery, r/genealogy.
5. Open Preservation Foundation blog / mailing list submission
   (disappearing-file-formats series).
6. Optional: contact DiskTransfer.co.uk — they openly offer paid
   PP1/2/3 recovery; offering to license/cite the decoder turns a
   competitor into a referrer.

The Python repo (this one) stays as the research workspace. The Rust
repo is the production artifact.

## Settled choices (from brainstorming)

| Area | Choice |
|------|--------|
| Goal | Library + CLI on crates.io AND personal daily driver |
| Scope | Full parity with `max2pdf.py` CLI (every flag, including diagnostic ones) |
| Repo | New standalone repo `vigb-decoder` |
| PDF | Hand-written (port of `max2pdf.py:write_pdf`) — no PDF crate dep |
| Crate split | Single crate, `[lib]` + `[[bin]]` |
| License | MIT OR Apache-2.0 (dual) |
| Tests | Synthetic `.max` checked into repo; real corpus tests local-only |
| Architecture | Idiomatic modular Rust (modules per concern, builder config, `Result<T, MaxError>`) |
| Benches | Yes — `criterion` on `decomp_line`, `decode_image_chunk`, whole file |

## Section 1 — Repo & crate

**Crate name**: `vigb-decoder` (see "Naming and legal posture" above).

**Cargo.toml**:

```toml
[package]
name = "vigb-decoder"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "MIT OR Apache-2.0"
repository = "https://github.com/<owner>/vigb-decoder"
description = "Decoder for PaperPort 2 .max (ViGBe) image scans (1986–87 era)"
keywords = ["paperport", "max", "ccitt", "decoder", "retro"]
categories = ["multimedia::images", "command-line-utilities"]

[lib]
name = "vigb_decoder"

[[bin]]
name = "max2pdf"
path = "src/bin/max2pdf.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
thiserror = "2"
anyhow = "1"          # bin-only

[dev-dependencies]
insta = "1"           # snapshot testing for bitmap / PDF output
criterion = "0.5"

[features]
corpus = []           # local-only feature; gates the personal-archive test

[[bench]]
name = "decoder"
harness = false
```

No `image`, no PDF crate, no `bitvec`. Matches Python's "minimal deps"
deploy story. The whole crate compiles in seconds and produces a
binary in the low-MB range.

**Repo layout**:

```
vigb-decoder/
├── Cargo.toml
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── CHANGELOG.md
├── docs/
│   ├── format.md             # ported from wiki/topics/format.md
│   ├── decoder.md            # canonical fixes + reference-table walk
│   ├── cli.md                # flag mapping vs max2pdf.py
│   ├── credits.md            # PP3.6 / paperman / otvdm / source archive
│   ├── provenance.md         # clean-room separation: spec / disasm / bit-traces
│   └── release-checklist.md  # marketing channels for v0.1 release
├── src/
│   ├── lib.rs                # public re-exports
│   ├── error.rs              # MaxError + Result alias
│   ├── bitstream.rs          # BitCursor (eager + lazy refill)
│   ├── ccitt.rs              # CCITT-T.6 tables (white/black runs, V/P/H/EOL)
│   ├── decoder.rs            # decomp_line — port of _decomp_line
│   ├── dispatch.rs           # decode_image_chunk — per-line marker dispatch
│   ├── preview.rs            # 102×146 thumbnail decoder + upscale
│   ├── chunks.rs             # parse_max / find_image_chunks
│   ├── pdf.rs                # hand-written PDF writer
│   ├── config.rs             # Config + ConfigBuilder + T0DropMode enum
│   └── bin/
│       └── max2pdf.rs        # clap derive parser → calls library
├── tests/
│   ├── synthetic.rs          # round-trip on a generated tiny .max
│   ├── ccitt_tables.rs       # spot-checks individual CCITT codes
│   ├── chunks.rs             # synthetic-magic chunk discovery
│   └── fixtures/
│       ├── synthetic.max
│       └── synthetic.pbm
├── benches/
│   └── decoder.rs            # criterion benches
└── .github/workflows/
    └── ci.yml                # build/test/fmt/clippy on Linux + Win + macOS
```

## Section 2 — Module API & core types

Public surface (re-exported from `lib.rs`):

```rust
pub use config::{Config, ConfigBuilder, T0DropMode, DispatchKind};
pub use decoder::{decode_max, decode_max_file, Page, Preview, DecodeStats};
pub use error::{MaxError, Result};
pub use pdf::{write_pdf, write_pdf_to, PdfOptions};
```

### `config.rs`

```rust
#[derive(Debug, Clone)]
pub struct Config {
    // canonical fixes (default ON; --no-* flips)
    pub bug4: bool,
    pub strict_t0: bool,
    pub drop_blank_after_drift: bool,
    pub suppress_t1_all: bool,

    // experimental knobs (default OFF; opt-in flags)
    pub lazy_bit_loading: bool,
    pub embed_preview: bool,                  // default true
    pub t0_reset: bool,
    pub t0_drop_after_drift: T0DropMode,      // None | Marker | Full
    pub t0_drop_kinds: Option<Vec<DispatchKind>>,
    pub fail_scan_forward: u32,
    pub suppress_t2_fail_y_in_cascade: bool,

    // smart resync
    pub fail_resync_max: u32,
    pub fail_resync_lookahead: u32,           // default 5
    pub fail_resync_min_confidence: u32,
    pub fail_resync_budget: u32,
    pub reset_ref_after_drift: bool,
}

impl Default for Config { /* canonical defaults — produces IoU=1.000 */ }
impl Config { pub fn builder() -> ConfigBuilder { /* ... */ } }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum T0DropMode { None, Marker, Full }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchKind { Ok, V0, T0, T1, Fail, Bad }
```

### `decoder.rs` — top-level entry

```rust
pub struct Page {
    pub width: u32,
    pub height: u32,
    pub bitmap: Vec<u8>,           // 1-bit packed, MSB-first, bit=1 means BLACK
    pub preview: Option<Preview>,
    pub stats: DecodeStats,
}

pub struct Preview { pub width: u32, pub height: u32, pub gray: Vec<u8> }

pub struct DecodeStats {
    pub n_ok: u32, pub n_v0: u32, pub n_t0: u32, pub n_t1: u32,
    pub n_fail: u32, pub max_consecutive_fail: u32,
    pub first_fail_y: Option<u32>,
    pub resync_probes: u32, pub resync_hits: u32,
    pub blank_drops_after_drift: u32,
}

pub fn decode_max(data: &[u8], cfg: &Config) -> Result<Vec<Page>>;
pub fn decode_max_file(path: &Path, cfg: &Config) -> Result<Vec<Page>>;
```

The bit-1-means-BLACK polarity is documented loudly on `Page.bitmap`'s
docstring — this was the source of a session-6 bug in the GT
comparison framework.

### `error.rs`

```rust
#[derive(Debug, thiserror::Error)]
pub enum MaxError {
    #[error("not a ViGB file: bad magic at offset {offset:#x}")]
    BadMagic { offset: u64 },
    #[error("truncated chunk at {offset:#x}: need {need} bytes, have {have}")]
    Truncated { offset: u64, need: usize, have: usize },
    #[error("decoder bit underrun at line {y}, x={x}")]
    BitUnderrun { y: u32, x: u32 },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, MaxError>;
```

### Internal modules (not re-exported)

| Module | Purpose |
|--------|---------|
| `bitstream` | `BitCursor` with `peek(n) -> u32`, `consume(n)`, `pos_bytes()`. Eager (refill until ≥24 bits buffered) and lazy (byte-by-byte at decision points) refill paths via a flag — no separate type needed. |
| `ccitt` | Const tables for CCITT-T.6: white run codes, black run codes, V_/P/H/EOL. Built once at module load. |
| `decoder` (private side) | `decomp_line` — direct port of `_decomp_line` from `max2pdf.py:201`, including the `bug4` and `lazy` flags. |
| `dispatch` | `decode_image_chunk` — port of the per-line marker dispatch loop, all heuristic gates as `&Config` reads. The smart-resync state machine lives inside this module (invasive, not a wrapper — same as Python). |
| `preview` | RLE decoder + nearest-neighbor upscale to A4. Uses pack-and-resize approach matching the recent 5–6× speedup in Python (PIL `frombytes` + `resize(NEAREST)` equivalent). |
| `chunks` | `find_image_chunks` + magic detection + multi-chunk page assembly. |
| `pdf` | `PdfWriter::add_image_page`, `add_preview_page`, `finish() -> Vec<u8>`. Hand-written, no deps. |

## Section 3 — Data flow & error handling

### Decode flow (single `.max` → `Vec<Page>`)

```
file bytes
  └─► chunks::find_image_chunks   ──► [(offset, length, kind)]
        for each image chunk:
          ├─► dispatch::decode_image_chunk(&data[offset..], &cfg)
          │     ├─► decoder::decomp_line(...)   per row
          │     │     └─► bitstream::BitCursor + ccitt tables
          │     └─► returns: Bitmap + DecodeStats
          if cfg.embed_preview && preview chunk paired:
            └─► preview::decode_preview_chunk(...)
                  └─► returns: Preview { 102, 146, gray }
        assemble Page { bitmap, preview, stats }
```

### PDF flow (separate, library-optional)

```
&[Page] + PdfOptions ──► pdf::write_pdf(...)
  ├─► writes page 1, page 1 preview, page 2, page 2 preview, ...
  └─► returns Vec<u8>   (or write_pdf_to(&mut Write))
```

### Error policy

- **Hard errors** (`Result::Err(MaxError)`): file not found, bad magic,
  truncated chunk header, IO error. Stop processing the file.
- **Soft errors** (recorded in `DecodeStats`, never panic, never `Err`):
  per-line FAIL events, BLANK over-dispatch drops, T1 suppressions,
  smart-resync probes. The whole point of the heuristics is to keep
  going.
- **Panics**: never. Even on malformed CCITT, decoding degrades to FAIL
  events. Slice access uses `.get()` / cursor methods that return `Err`
  on underrun.

### CLI batch behavior

`max2pdf a.max b.max c.max` continues past per-file errors (matching
`max2pdf.py`); prints `error: <file>: <MaxError>` to stderr. Exit code:
`0` if all files succeeded, `1` if any failed. `--stats` prints
`DecodeStats` per file.

## Section 4 — Testing strategy

### Layer 1 — Unit tests on primitives (always run)

Inside `src/<module>.rs` with `#[cfg(test)] mod tests`:

- `ccitt`: every white/black run-length code decodes to the expected
  `(run, length)` pair; V_R3, V_L3, P, H, EOL exact bit patterns.
- `bitstream`: round-trip via a `#[cfg(test)]` bit-writer; eager vs
  lazy refill produce identical `consume_bits()` totals.
- `pdf`: synthetic 1-page input — header, xref, trailer offsets parse
  with a regex.
- `chunks`: synthetic 64-byte buffer with two ViGB magic markers;
  offsets and lengths recovered.

### Layer 2 — Integration test: synthetic round-trip (always run)

`tests/synthetic.rs`:

1. Build a tiny synthetic bitmap programmatically — checkerboard
   region + horizontal lines + sparse single pixels (exercises white
   runs, black runs, V/P/H mode mix).
2. Encode to a `.max` chunk via a test-only encoder (port the relevant
   slice of `encoder_validator.py`). Check `tests/fixtures/synthetic.max`
   into git so CI doesn't regenerate.
3. Decode with `decode_max(...)` under default `Config`.
4. Assert decoded bitmap == source pixel-for-pixel (snapshot via `insta`).

This is the public regression test. No personal documents required.

### Layer 3 — Local-only corpus test (gated)

`tests/corpus.rs` behind `#[cfg(feature = "corpus")]`:

- Reads `tests-private/inventory.csv` (path via env var
  `VIGB_DECODER_CORPUS`).
- For each paired file, decodes and asserts IoU ≥ 0.99 against the GT.
- CI never enables this feature; user runs `cargo test --features corpus`
  locally.

### Layer 4 — Benches (criterion)

`benches/decoder.rs`:

- `bench_decomp_line` — single-line decode on a fixed 2464-pixel row.
- `bench_decode_image_chunk` — full chunk decode on the synthetic
  fixture.
- `bench_whole_file` — `decode_max_file` on the synthetic fixture.

Tracks regressions as Python's `profile_max2pdf.py` does. Expected
ballpark: 5–20× faster than current Python (140–300 ms/page → 7–60
ms/page).

### Layer 5 — CI matrix

`.github/workflows/ci.yml`:

- OS: `ubuntu-latest`, `windows-latest`, `macos-latest`.
- Steps: `cargo fmt --check`, `cargo clippy -- -D warnings`,
  `cargo build --release`, `cargo test`.
- Cache via `Swatinem/rust-cache`.
- Optional `cargo audit` job.

### Excluded from public testing

- Real `.max` files from the user's archive.
- Bit-perfect equality against PaperPort 3.6 BMP exports
  (references in `archive/sessions-5-11/`; stay local).
- Anything requiring otvdm / Wine / the bridge.

## Section 5 — CLI, packaging, README

### CLI

`clap` derive parser. Every `max2pdf.py` flag preserved with the same
long name (muscle memory transfers). Defaults match Python defaults
(canonical fixes ON, experimental flags OFF).

Full flag mapping table goes in `docs/cli.md` and is checked against
`max2pdf.py:1066–1156` — the parity claim must be verifiable.

### Packaging

- `cargo install vigb-decoder` → drops `max2pdf` in `~/.cargo/bin/`.
- Pre-built binaries via `cargo dist` or hand-rolled GitHub Actions
  release workflow: Linux x86_64, Windows x86_64, macOS aarch64,
  triggered on `git tag v*`.
- `cargo add vigb-decoder` for library use.
- crates.io publish: `cargo publish` from a clean tag; CHANGELOG
  updated per release.

### README outline

```
# vigb-decoder

Decoder for PaperPort 2 (.max) image scans from 1986–87.

## Why this exists
[Format is dead. Only way to open these files was a 1996 Windows app
that doesn't run on modern Windows. This is the reverse-engineered
decoder. First known tool that handles PaperPort 2 era files —
paperman and max2pdf both explicitly do not.]

## Install
    cargo install vigb-decoder

## Use
    max2pdf scan1.max scan2.max -o out/

## Library use
    use vigb_decoder::{decode_max_file, write_pdf, Config};
    let pages = decode_max_file("scan.max", &Config::default())?;
    write_pdf(&pages, "scan.pdf")?;

## Status
Bit-perfect against the canonical PaperPort 3.6 reference on every
file we have ground truth for. Median IoU = 1.000 across a 159-page
test corpus.

## Format reverse-engineering
See docs/format.md and docs/decoder.md. Provenance: docs/provenance.md.

## Reverse-engineering legal basis
This decoder was reverse-engineered for interoperability under:
- Switzerland: URG Art. 21 (decompilation for interface info).
- EU: Software Directive 2009/24/EC Art. 6 (RE for interoperability).
- US: DMCA §1201(f) safe harbour and Sega v. Accolade,
  977 F.2d 1510 (9th Cir. 1992) (disassembly for interop is fair use).

The decoder ships zero bytes from PaperPort. CCITT-T.6 lookup tables
are derived from the ITU-T T.6 Recommendation (a public standard);
format dispatch logic was developed against bit-traces of
self-owned .max files cross-checked against the disassembly of
ScanSoft's MAXKER2.DLL (extracted from the publicly distributed
Visioneer 5.2 installer ISO, archive.org, 2020).

## Credits
- PaperPort 3.6 (ScanSoft, 1996) — bridge that made the RE possible.
  MAXKER2.DLL is the canonical implementation matched here.
- ITU-T T.6 Recommendation — source for CCITT Group 4 table values.
- paperman (Simon Glass) and max2pdf (orangeturtle739) — prior-art
  OSS projects, used as cross-checks during RE. Both GPL-2-or-later;
  no code is copied from either project (see docs/provenance.md).
- otvdm (otya128) — runs the PP3.6 bridge under modern Windows.

## License
MIT OR Apache-2.0
```

### Docs ported from the Python repo's wiki

- `docs/format.md` ← `wiki/topics/format.md`
- `docs/decoder.md` ← `wiki/topics/decoder-heuristics.md` +
  `bridge-and-bug-fixes.md` summary
- `docs/credits.md` ← `wiki/sources/*` highlights

### Excluded from the public repo

- `NOTES.md` (1500-line research log — stays in the Python repo).
- `wiki/timeline.md` (session-by-session — stays here).
- All experimental scripts in `archive/sessions-5-11/` — stays here.
- `inventory_vigb.csv` and any path referencing
  `D:\office (nsa220)\` — never public.

### Versioning

Semver, starting `0.1.0`. Bug fixes patch, additive flags minor,
breaking lib changes major. Tag `v0.1.0` for first publish.

## Out of scope (for v0.1)

- `serde` impls on public types (easy to add later as a feature flag).
- Async / streaming decode (Python is fully synchronous; matches use
  case).
- WASM target (could be added behind a feature flag if there's
  demand).
- Encoder for `.max` (only the test-only minimal encoder needed for
  synthetic fixtures; full encoder would be a separate crate, since
  no real-world use case is known).
- Color image support (PaperPort 2 was 1-bit only).
- GUI / drag-and-drop wrapper.

## Risks and open questions

- **CCITT-T.6 table provenance**: the Python decoder's `_build_lookup`
  and `_build_run_table` build the lookup tables at runtime from
  paperman-derived values (`__pp_src__/paperman_btab.dat`). The Rust
  port must instead derive the table values from the ITU-T T.6
  Recommendation directly — paperman is GPL-2-or-later and copying
  its table arrays into a permissively-licensed crate is the single
  highest-impact legal risk identified in the 2026-05-10 research.
  Mitigation: (1) port the spec PDF's tables by hand into
  `src/ccitt.rs` as `const` arrays; (2) cite the ITU recommendation
  as the source in a header comment; (3) the synthetic round-trip
  test is the correctness gate — if table values diverge by even
  one entry, the pixel comparison fails. The unit tests in
  `ccitt.rs` spot-check known codes against values quoted directly
  from the ITU-T T.6 spec, never against `__pp_src__/paperman_btab.dat`.
- **`bug4` reference-table walk**: the most subtle bug fixed in
  session 12. The Rust port must replicate the canonical `lodsw +1
  idx per V code` semantics exactly. The Python's `test_bug4.py`
  regression test is essentially a per-line transition diff against
  PP 3.6 BMP exports — the Rust port can't easily run that test
  publicly (BMPs are personal documents), but the synthetic test
  exercises the V-code path enough to catch grossly wrong walks. Real
  validation happens in the local-only corpus test layer.
- **Binary size**: `clap` adds a few hundred KB. Acceptable for a CLI;
  if it ever matters, `clap`'s `derive` feature can be replaced with
  a hand-rolled parser (the Python uses argparse with similar weight).
- **Windows path handling**: the user is on Windows. `PathBuf` and
  `std::fs` handle this correctly out of the box; CI matrix includes
  `windows-latest` to catch regressions.

## Implementation order (preview — full plan goes to writing-plans)

1. Scaffold crate + CI + LICENSE files (no logic).
2. `error.rs` + `bitstream.rs` + `ccitt.rs` with unit tests.
3. `chunks.rs` (file walking, no decode).
4. `decoder.rs` (`decomp_line`) — port + unit tests.
5. `dispatch.rs` (`decode_image_chunk`) — port + canonical-defaults
   integration with synthetic fixture.
6. Build the synthetic fixture + the test-only encoder in
   `tests/synthetic.rs`. Round-trip green = canonical decoder works.
7. `preview.rs`.
8. `pdf.rs` — port `write_pdf`.
9. `config.rs` + `bin/max2pdf.rs` (clap derive).
10. Heuristic flags wired in (smart resync, t0-drop modes,
    fail-scan-forward, etc.).
11. Docs port (`format.md`, `decoder.md`, `cli.md`, `credits.md`).
12. README, CHANGELOG, release workflow, `cargo publish` v0.1.0.

Each step is its own commit / PR-sized unit of work. The plan from
the writing-plans skill will expand each step with concrete files,
expected diffs, and test gates.
