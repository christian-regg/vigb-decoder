# vigb-decoder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the canonical Python `.max` decoder (`C:\Users\chris\Desktop\Alte Scans\max2pdf.py`, 1197 lines, IoU=1.000 on the test corpus) to a Rust library + CLI crate `vigb-decoder` published on crates.io.

**Architecture:** Single Cargo crate with `[lib]` + `[[bin]]` (binary `max2pdf`). Modules: `bitstream`, `ccitt`, `decoder`, `dispatch`, `preview`, `chunks`, `pdf`, `config`, `error`. `Result<T, MaxError>` with `thiserror`. `Decoder::Config` builder for the heuristic flags. Hand-written PDF (no PDF crate dep).

**Tech Stack:** Rust 2021 (MSRV 1.75), `clap` v4 (derive), `thiserror` v2, `anyhow` v1 (bin only), `insta` v1 + `criterion` v0.5 (dev only). No image, PDF, or bitvec crates.

**Reference source:** `C:\Users\chris\Desktop\Alte Scans\max2pdf.py` — the canonical Python decoder. The plan references this file by line number throughout. The engineer executing this plan must have read access to it.

**Spec:** `F:\Projects\vigb-decoder\docs\superpowers\specs\2026-05-10-rust-port-design.md` (committed as 84f2d23).

**Working directory:** `F:\Projects\vigb-decoder` for all `cargo`/`git` commands unless noted.

---

## File Structure

```
vigb-decoder/
├── Cargo.toml                              # Task 1
├── README.md                               # Task 17
├── LICENSE-MIT                             # Task 1
├── LICENSE-APACHE                          # Task 1
├── CHANGELOG.md                            # Task 18
├── .gitignore                              # Task 1
├── docs/
│   ├── format.md                           # Task 16
│   ├── decoder.md                          # Task 16
│   ├── cli.md                              # Task 16
│   ├── credits.md                          # Task 16
│   ├── provenance.md                       # Task 16
│   └── release-checklist.md                # Task 16
├── src/
│   ├── lib.rs                              # Task 1 (skeleton), 13 (final)
│   ├── error.rs                            # Task 2
│   ├── config.rs                           # Task 3
│   ├── bitstream.rs                        # Task 4
│   ├── ccitt.rs                            # Task 5
│   ├── chunks.rs                           # Task 6
│   ├── decoder.rs                          # Task 7  (decomp_line)
│   ├── dispatch.rs                         # Task 9  (canonical) + Task 10 (flags)
│   ├── preview.rs                          # Task 11
│   ├── pdf.rs                              # Task 12
│   └── bin/
│       └── max2pdf.rs                      # Task 14
├── tests/
│   ├── common/
│   │   └── encoder.rs                      # Task 8 (test-only encoder)
│   ├── synthetic.rs                        # Task 9 (uses encoder)
│   ├── chunks.rs                           # Task 6
│   └── fixtures/
│       └── synthetic.max                   # Task 8 (committed)
├── benches/
│   └── decoder.rs                          # Task 15
└── .github/workflows/
    ├── ci.yml                              # Task 1
    └── release.yml                         # Task 18
```

---

## Notes on translation strategy

- For **leaf utility code** (small functions, table builders, byte-format wrangling): the plan contains the full Rust code.
- For **algorithm ports** (`decomp_line`, `decode_image_chunk`): the plan provides the Rust signature, the test code (full), translation notes calling out the gotchas the Python source had baked-in lessons about, and an explicit pass criterion (the synthetic round-trip + the unit tests). The Python source at the cited line ranges is the algorithmic spec.
- **Page struct refinement**: the design spec lists `Page { width, height, bitmap, preview, stats }`. The Python `decode_image_chunk` actually returns `{width, height, dpi_x, dpi_y, row_bytes, raw}` — the PDF writer needs all of those. The Rust `Page` struct in this plan is the superset: `{width, height, dpi_x, dpi_y, row_bytes, bitmap, preview, stats}`. This supersedes the spec.
- **Polarity reminder**: the raw bitmap uses bit=1 means BLACK, MSB-first, packed (matches Python output and PDF `/Indexed` palette `[0=white, 1=black]` convention). This was the source of the 6th-session GT-comparison polarity bug in Python — document it loudly on `Page::bitmap`.

---

## Task 1: Project scaffold + CI + LICENSE files

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `LICENSE-MIT`, `LICENSE-APACHE`, `src/lib.rs`, `src/bin/max2pdf.rs`, `.github/workflows/ci.yml`, `rust-toolchain.toml`

- [ ] **Step 1: Create `Cargo.toml`**

```toml
[package]
name = "vigb-decoder"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
license = "MIT OR Apache-2.0"
repository = "https://github.com/creggch/vigb-decoder"
description = "Decoder for PaperPort 2 .max (ViGBe) image scans (1986–87 era)"
keywords = ["paperport", "max", "ccitt", "decoder", "retro"]
categories = ["multimedia::images", "command-line-utilities"]
readme = "README.md"

[lib]
name = "vigb_decoder"
path = "src/lib.rs"

[[bin]]
name = "max2pdf"
path = "src/bin/max2pdf.rs"

[dependencies]
thiserror = "2"

[dependencies.clap]
version = "4"
features = ["derive"]

[dev-dependencies]
anyhow = "1"
insta = { version = "1", features = ["yaml"] }
criterion = "0.5"

[features]
default = []
corpus = []   # Local-only feature: enables tests/corpus.rs against personal archive

[[bench]]
name = "decoder"
harness = false

[profile.release]
lto = "thin"
codegen-units = 1
```

The repository URL uses `creggch` as a placeholder — replace with the actual GitHub username at first push if different.

- [ ] **Step 2: Create `.gitignore`**

```
/target
**/*.rs.bk
Cargo.lock.bak
.idea
.vscode
*.swp
.DS_Store
# Local-only corpus tests pull from outside the repo
/tests-private/
```

Note: `Cargo.lock` IS committed (binary crate convention).

- [ ] **Step 3: Create `LICENSE-MIT`** with the standard MIT text, copyright "2026 Christian Regg".

```
MIT License

Copyright (c) 2026 Christian Regg

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 4: Create `LICENSE-APACHE`** with the Apache-2.0 text from https://www.apache.org/licenses/LICENSE-2.0.txt verbatim. Append the standard "Copyright 2026 Christian Regg / Licensed under the Apache License, Version 2.0" notice block at the end.

- [ ] **Step 5: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "1.75"
components = ["rustfmt", "clippy"]
```

- [ ] **Step 6: Create `src/lib.rs` skeleton**

```rust
//! Decoder for PaperPort 2 (.max) image scans.
//!
//! The PaperPort 2 file format ("ViGBe") is a proprietary container used by
//! ScanSoft's PaperPort 2 (1996) for 1-bit scanned documents. Each image
//! chunk wraps a CCITT-T.6 (Group 4 fax) compressed bitmap with a custom
//! per-line marker dispatcher.
//!
//! See `docs/format.md` and `docs/decoder.md` in this repo for the format
//! specification and the canonical decoder behaviour.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// Modules wired up incrementally in later tasks.
```

- [ ] **Step 7: Create `src/bin/max2pdf.rs` skeleton**

```rust
fn main() {
    println!("max2pdf placeholder — see Task 14 for the real CLI.");
}
```

- [ ] **Step 8: Create `.github/workflows/ci.yml`**

```yaml
name: ci

on:
  push:
    branches: [master, main]
  pull_request:

jobs:
  test:
    name: test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.75
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: cargo fmt --check
        run: cargo fmt --all -- --check
      - name: cargo clippy
        run: cargo clippy --all-targets -- -D warnings
      - name: cargo build --release
        run: cargo build --release --verbose
      - name: cargo test
        run: cargo test --verbose
```

- [ ] **Step 9: Verify the scaffold builds and tests pass**

Run: `cargo build && cargo test && cargo run --bin max2pdf`
Expected:
```
Compiling vigb-decoder v0.1.0 ...
Finished `dev` profile ...
running 0 tests ...
test result: ok. 0 passed; 0 failed; ...
max2pdf placeholder — see Task 14 for the real CLI.
```

If any of these fail, fix before continuing.

- [ ] **Step 10: Commit**

```powershell
git add -A
git commit -m @'
chore: scaffold vigb-decoder crate

Empty lib + placeholder binary, MIT/Apache-2.0 dual license,
Rust 1.75 MSRV, CI matrix on Linux/Windows/macOS.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 2: Error type

**Files:**
- Create: `src/error.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to `src/error.rs` (will create file in step 2):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_magic_displays() {
        let e = MaxError::BadMagic { offset: 0x42 };
        assert_eq!(e.to_string(), "not a ViGB file: bad magic at offset 0x42");
    }

    #[test]
    fn truncated_displays() {
        let e = MaxError::Truncated { offset: 0x100, need: 8, have: 3 };
        assert_eq!(e.to_string(), "truncated chunk at 0x100: need 8 bytes, have 3");
    }

    #[test]
    fn bit_underrun_displays() {
        let e = MaxError::BitUnderrun { y: 305, x: 2376 };
        assert_eq!(e.to_string(), "decoder bit underrun at line 305, x=2376");
    }

    #[test]
    fn io_error_wrapping_round_trips() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let e: MaxError = io.into();
        assert!(matches!(e, MaxError::Io(_)));
        assert!(e.to_string().contains("io:"));
    }
}
```

- [ ] **Step 2: Create `src/error.rs`**

```rust
//! Error types for the vigb-decoder crate.

use std::result::Result as StdResult;

/// Errors returned by the .max decoder.
///
/// Hard errors (file-level) — soft errors (per-line decode failures) are
/// reported via `crate::DecodeStats`, never as `Err`.
#[derive(Debug, thiserror::Error)]
pub enum MaxError {
    /// File magic check failed; the input is not a ViGBe `.max` file.
    #[error("not a ViGB file: bad magic at offset {offset:#x}")]
    BadMagic {
        /// Byte offset where magic was expected.
        offset: u64,
    },
    /// A chunk header claims more bytes than remain in the file.
    #[error("truncated chunk at {offset:#x}: need {need} bytes, have {have}")]
    Truncated {
        /// Chunk start offset.
        offset: u64,
        /// Bytes the chunk header advertised.
        need: usize,
        /// Bytes actually available from this offset to EOF.
        have: usize,
    },
    /// The CCITT bitstream ran out of bits mid-line.
    #[error("decoder bit underrun at line {y}, x={x}")]
    BitUnderrun {
        /// Row index in the image.
        y: u32,
        /// Pixel column where the underrun was detected.
        x: u32,
    },
    /// Underlying IO error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience alias: `Result<T, MaxError>`.
pub type Result<T> = StdResult<T, MaxError>;

// (insert tests here per Step 1)
```

Move the `#[cfg(test)] mod tests` block from Step 1 into the bottom of this file.

- [ ] **Step 3: Wire into `src/lib.rs`**

Add to `src/lib.rs`:

```rust
mod error;
pub use error::{MaxError, Result};
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib error`
Expected: `4 passed; 0 failed`

- [ ] **Step 5: Commit**

```powershell
git add src/error.rs src/lib.rs
git commit -m @'
feat: add MaxError + Result type

Hard errors only (BadMagic, Truncated, BitUnderrun, Io). Soft per-line
decode failures are surfaced via DecodeStats in later tasks.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 3: Config + ConfigBuilder

**Files:**
- Create: `src/config.rs`
- Modify: `src/lib.rs`

Why this is Task 3 (before bitstream/ccitt): later modules' functions take `&Config`. Defining the type first lets every subsequent task use it without forward references.

- [ ] **Step 1: Write the failing test**

Add this test module at the bottom of `src/config.rs` after Step 2:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_canonical() {
        let c = Config::default();
        // Canonical fixes: ON
        assert!(c.bug4);
        assert!(c.strict_t0);
        assert!(c.drop_blank_after_drift);
        assert!(c.suppress_t1_all);
        assert!(c.embed_preview);
        // Diagnostic / experimental flags: OFF
        assert!(!c.lazy_bit_loading);
        assert!(!c.t0_reset);
        assert_eq!(c.t0_drop_after_drift, T0DropMode::None);
        assert!(c.t0_drop_kinds.is_none());
        assert_eq!(c.fail_scan_forward, 0);
        assert!(!c.suppress_t2_fail_y_in_cascade);
        assert_eq!(c.fail_resync_max, 0);
        assert_eq!(c.fail_resync_lookahead, 5);
        assert_eq!(c.fail_resync_min_confidence, 0);
        assert_eq!(c.fail_resync_budget, 0);
        assert!(!c.reset_ref_after_drift);
    }

    #[test]
    fn builder_round_trip() {
        let c = Config::builder()
            .bug4(false)
            .fail_resync_max(4)
            .reset_ref_after_drift(true)
            .build();
        assert!(!c.bug4);
        assert_eq!(c.fail_resync_max, 4);
        assert!(c.reset_ref_after_drift);
        // Untouched fields keep defaults
        assert!(c.strict_t0);
    }

    #[test]
    fn t0_drop_mode_parsing() {
        assert_eq!("none".parse::<T0DropMode>().unwrap(), T0DropMode::None);
        assert_eq!("marker".parse::<T0DropMode>().unwrap(), T0DropMode::Marker);
        assert_eq!("full".parse::<T0DropMode>().unwrap(), T0DropMode::Full);
        assert!("bogus".parse::<T0DropMode>().is_err());
    }
}
```

- [ ] **Step 2: Create `src/config.rs`**

```rust
//! Decoder configuration (canonical defaults + heuristic flags).

use std::str::FromStr;

/// Behaviour for the type-0 marker `t0_drop_after_drift` heuristic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum T0DropMode {
    /// No type-0 drops (default).
    #[default]
    None,
    /// Drop the marker byte only.
    Marker,
    /// Drop the marker byte plus its declared payload bytes.
    Full,
}

impl FromStr for T0DropMode {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "" | "none" => Ok(T0DropMode::None),
            "marker" => Ok(T0DropMode::Marker),
            "full" => Ok(T0DropMode::Full),
            other => Err(format!("invalid t0-drop mode: {other}")),
        }
    }
}

/// Per-line dispatch outcome kinds (used by `t0_drop_kinds` filter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchKind {
    Ok,
    V0,
    T0,
    T1,
    Fail,
    Bad,
}

/// Decoder configuration.
///
/// `Config::default()` produces canonical behaviour (matches the Python
/// `max2pdf.py` defaults at corpus median IoU = 1.000). All other fields
/// are diagnostic or experimental — leave them as default unless you know
/// what you're flipping.
#[derive(Debug, Clone)]
pub struct Config {
    // --- Canonical fixes (default ON) ---
    /// 12th-session canonical reference-table walk. Default true.
    pub bug4: bool,
    /// 11th-session strict type-0 marker gate (only low6==1 raw, low6==3
    /// skip — drop everything else). Default true.
    pub strict_t0: bool,
    /// 6th-session: drop type-3 BLANK markers that follow a non-OK
    /// dispatch (sync-drift recovery). Default true.
    pub drop_blank_after_drift: bool,
    /// 6th-session: suppress all type-1 dispatches (99% sync-drift in
    /// ViGBe corpus). Default true.
    pub suppress_t1_all: bool,
    /// Embed the 102×146 grayscale preview thumbnail as a second PDF
    /// page per scanned page. Default true.
    pub embed_preview: bool,

    // --- Experimental / diagnostic (default OFF) ---
    /// 11th-session lazy bit loading (byte-by-byte refill). Diagnostic.
    pub lazy_bit_loading: bool,
    /// Reset reference table after each chunk. Diagnostic.
    pub t0_reset: bool,
    /// `t0_drop_after_drift` mode (None | Marker | Full).
    pub t0_drop_after_drift: T0DropMode,
    /// Optional: only apply t0 drop after drift for these dispatch kinds.
    pub t0_drop_kinds: Option<Vec<DispatchKind>>,
    /// Bytes to scan-forward after a FAIL looking for next valid marker.
    pub fail_scan_forward: u32,
    /// 7th-session: in cascade FAIL runs, do not advance y on each FAIL.
    pub suppress_t2_fail_y_in_cascade: bool,

    // --- Smart resync (10th-session) ---
    /// Search range ±K for resync probe after isolated FAIL. 0 disables.
    pub fail_resync_max: u32,
    /// Probe lookahead in lines. Default 5.
    pub fail_resync_lookahead: u32,
    /// Minimum (n_ok - n_drift) margin to accept a resync candidate.
    pub fail_resync_min_confidence: u32,
    /// Maximum total resync probes per page. 0 = unlimited.
    pub fail_resync_budget: u32,
    /// Reset reference table to all-white after a drift event.
    pub reset_ref_after_drift: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bug4: true,
            strict_t0: true,
            drop_blank_after_drift: true,
            suppress_t1_all: true,
            embed_preview: true,
            lazy_bit_loading: false,
            t0_reset: false,
            t0_drop_after_drift: T0DropMode::None,
            t0_drop_kinds: None,
            fail_scan_forward: 0,
            suppress_t2_fail_y_in_cascade: false,
            fail_resync_max: 0,
            fail_resync_lookahead: 5,
            fail_resync_min_confidence: 0,
            fail_resync_budget: 0,
            reset_ref_after_drift: false,
        }
    }
}

impl Config {
    /// Start building a custom Config (defaults to canonical).
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder { inner: Self::default() }
    }
}

/// Fluent builder for `Config`.
pub struct ConfigBuilder { inner: Config }

macro_rules! setter {
    ($field:ident, $type:ty) => {
        /// Set the `$field` field.
        pub fn $field(mut self, value: $type) -> Self {
            self.inner.$field = value;
            self
        }
    };
}

impl ConfigBuilder {
    setter!(bug4, bool);
    setter!(strict_t0, bool);
    setter!(drop_blank_after_drift, bool);
    setter!(suppress_t1_all, bool);
    setter!(embed_preview, bool);
    setter!(lazy_bit_loading, bool);
    setter!(t0_reset, bool);
    setter!(t0_drop_after_drift, T0DropMode);
    setter!(t0_drop_kinds, Option<Vec<DispatchKind>>);
    setter!(fail_scan_forward, u32);
    setter!(suppress_t2_fail_y_in_cascade, bool);
    setter!(fail_resync_max, u32);
    setter!(fail_resync_lookahead, u32);
    setter!(fail_resync_min_confidence, u32);
    setter!(fail_resync_budget, u32);
    setter!(reset_ref_after_drift, bool);

    /// Finalize the configuration.
    pub fn build(self) -> Config { self.inner }
}

// (insert tests here per Step 1)
```

- [ ] **Step 3: Wire into `src/lib.rs`**

Append:

```rust
mod config;
pub use config::{Config, ConfigBuilder, DispatchKind, T0DropMode};
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib config`
Expected: `3 passed; 0 failed`

- [ ] **Step 5: Commit**

```powershell
git add src/config.rs src/lib.rs
git commit -m @'
feat: add Config + ConfigBuilder

Default = canonical decoder behaviour (corpus median IoU = 1.000).
Heuristic flags wired in but used only by later tasks.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 4: BitCursor

**Files:**
- Create: `src/bitstream.rs`
- Modify: `src/lib.rs`

Mirrors Python `_refill` (eager 16-bit) and `_refill_lazy` (byte-by-byte). Backed by a single struct with a flag, since both paths share state.

**Reference:** `max2pdf.py:174-198`.

- [ ] **Step 1: Write the failing tests**

Add to bottom of `src/bitstream.rs` after Step 2:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eager_peek_consume_round_trip() {
        // 0xAB 0xCD = 1010_1011 1100_1101
        let data = &[0xAB, 0xCD, 0xEF, 0x12];
        let mut bc = BitCursor::new(data, false);
        assert_eq!(bc.peek(8).unwrap(), 0xAB);
        bc.consume(4);
        assert_eq!(bc.peek(8).unwrap(), 0xBC);
        bc.consume(8);
        assert_eq!(bc.peek(4).unwrap(), 0xC);
    }

    #[test]
    fn lazy_and_eager_consume_match() {
        let data = &[0x80, 0xF8, 0x42, 0x17, 0xC0, 0x00];
        let mut e = BitCursor::new(data, false);
        let mut l = BitCursor::new(data, true);
        for n in [3, 5, 7, 13, 8, 4] {
            assert_eq!(e.peek(n).unwrap(), l.peek(n).unwrap(),
                       "peek({}) diverges", n);
            e.consume(n);
            l.consume(n);
        }
        // Both should report identical bits-consumed totals.
        assert_eq!(e.consumed_bits(), l.consumed_bits());
    }

    #[test]
    fn underrun_returns_none() {
        let data = &[0xFF];
        let mut bc = BitCursor::new(data, false);
        bc.consume(8);
        assert!(bc.peek(1).is_none());
    }

    #[test]
    fn pos_bytes_advances_on_consume() {
        let data = &[0x12, 0x34, 0x56, 0x78];
        let mut bc = BitCursor::new(data, false);
        let _ = bc.peek(8).unwrap();
        bc.consume(8);
        // After consuming 8 bits, the conceptual position is 1 byte in.
        assert_eq!(bc.consumed_bits(), 8);
    }
}
```

- [ ] **Step 2: Create `src/bitstream.rs`**

```rust
//! Bit reader for CCITT-T.6 line decoding.
//!
//! Two refill modes:
//! - `eager`: load 2 bytes whenever the buffer holds ≤16 bits (matches
//!   Python `_refill`, the default and historical decoder behaviour).
//! - `lazy`: load 1 byte at a time only when the next peek would underrun
//!   (matches Python `_refill_lazy`, mirrors PaperPort 3.6's
//!   `MAXKER2.DLL` byte-by-byte refill timing).
//!
//! Both modes are correct on canonical files. `lazy` exists as a
//! diagnostic for sync-drift investigation — see `Config::lazy_bit_loading`.

/// MSB-first bit reader over a byte slice.
pub(crate) struct BitCursor<'a> {
    data: &'a [u8],
    /// Bit window (right-aligned in a u64 so we can hold up to 32 buffered bits
    /// after a refill without losing any when the next refill shifts in 16 more).
    bits: u64,
    /// Number of valid bits currently in `bits`, right-aligned.
    bits_left: u32,
    /// Byte offset into `data` of the next byte to load.
    pos: usize,
    /// True ⇒ byte-by-byte refill (`_refill_lazy` semantics).
    lazy: bool,
    /// Total bits consumed across the cursor's lifetime (for `consumed_bits`).
    total_consumed: u64,
}

impl<'a> BitCursor<'a> {
    /// Create a new cursor at byte offset 0 of `data`.
    ///
    /// `lazy = false` matches Python `_refill` (eager 16-bit refill).
    /// `lazy = true` matches Python `_refill_lazy` (byte-by-byte).
    pub fn new(data: &'a [u8], lazy: bool) -> Self {
        Self {
            data,
            bits: 0,
            bits_left: 0,
            pos: 0,
            lazy,
            total_consumed: 0,
        }
    }

    /// Create a cursor that starts at `start_pos` bytes into `data`.
    pub fn with_start(data: &'a [u8], start_pos: usize, lazy: bool) -> Self {
        Self { data, bits: 0, bits_left: 0, pos: start_pos, lazy, total_consumed: 0 }
    }

    /// Peek the next `n` bits (1..=32) without consuming them.
    /// Returns `None` if the stream cannot supply that many bits.
    pub fn peek(&mut self, n: u32) -> Option<u32> {
        debug_assert!(n >= 1 && n <= 32);
        self.refill_if_needed(n);
        if self.bits_left < n {
            return None;
        }
        Some(((self.bits >> (self.bits_left - n)) & ((1u64 << n) - 1)) as u32)
    }

    /// Consume `n` previously-peeked bits.
    pub fn consume(&mut self, n: u32) {
        debug_assert!(self.bits_left >= n, "consume({n}) with {} bits buffered", self.bits_left);
        self.bits_left -= n;
        self.total_consumed += n as u64;
    }

    /// Total bits consumed via `consume`.
    pub fn consumed_bits(&self) -> u64 {
        self.total_consumed
    }

    /// Byte offset of the next byte that *would* be loaded — useful for
    /// computing `pos - start_pos` after-the-fact (matches Python's
    /// `(pos - start_pos) * 8 - bits_left`).
    pub fn next_load_byte(&self) -> usize {
        self.pos
    }

    /// Bits currently buffered (used by callers that compute byte-position
    /// after a decode in the Python idiom).
    pub fn bits_buffered(&self) -> u32 {
        self.bits_left
    }

    fn refill_if_needed(&mut self, need: u32) {
        if self.lazy {
            // Byte-by-byte until we have `need` bits or run out.
            while self.bits_left < need && self.pos < self.data.len() {
                let b = self.data[self.pos] as u64;
                self.bits = ((self.bits << 8) | b) & 0xFFFF_FFFF_FFFF_FFFF;
                self.bits_left += 8;
                self.pos += 1;
            }
        } else {
            // Eager 16-bit refill matching Python `_refill`. Fires once when
            // bits_left drops to ≤16, regardless of `need`.
            if self.bits_left <= 16 {
                let b0 = if self.pos < self.data.len() { self.data[self.pos] as u64 } else { 0 };
                let b1 = if self.pos + 1 < self.data.len() { self.data[self.pos + 1] as u64 } else { 0 };
                self.bits = ((self.bits << 16) | (b0 << 8) | b1) & 0xFFFF_FFFF_FFFF_FFFF;
                self.bits_left += 16;
                self.pos += 2;
            }
        }
    }
}

// (insert tests here per Step 1)
```

- [ ] **Step 3: Wire into `src/lib.rs`**

Append:

```rust
mod bitstream;
```

(Not re-exported — internal.)

- [ ] **Step 4: Run tests**

Run: `cargo test --lib bitstream`
Expected: `4 passed; 0 failed`

- [ ] **Step 5: Commit**

```powershell
git add src/bitstream.rs src/lib.rs
git commit -m @'
feat: add BitCursor (eager + lazy refill)

Mirrors Python _refill (16-bit eager, default) and _refill_lazy
(byte-by-byte) in a single struct gated by a flag.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 5: CCITT-T.6 tables

**Files:**
- Create: `src/ccitt.rs`
- Modify: `src/lib.rs`

**Critical legal note:** the table values in this module MUST be derived from the ITU-T T.6 Recommendation directly, not from `paperman_btab.dat`. paperman is GPL-2-or-later; copying its arrays into this MIT/Apache crate contaminates downstream consumers. The numerical values themselves are facts from a public standard — same numbers, different provenance. See `docs/provenance.md` (Task 16).

**Source:** ITU-T Recommendation T.6 (07/88), Table 1/T.6 (terminating codes), Table 2/T.6 (make-up codes), Table 3/T.6 (two-dimensional codes). Free PDF at https://www.itu.int/rec/T-REC-T.6.

The Python file at `max2pdf.py:46-105` contains the table values currently in use, transcribed from paperman. Verify each row against the ITU PDF before transcribing — they SHOULD match, but the verification step is what makes this a clean-room derivation.

- [ ] **Step 1: Write the failing tests**

Add to bottom of `src/ccitt.rs` after Step 2:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_term_zero_is_8b_0x35() {
        // ITU-T T.6 Table 1: white terminating, run length 0 = 00110101 (8 bits)
        let (length, code, _payload) = WHITE_TERM_ENTRIES[0];
        assert_eq!(length, 8);
        assert_eq!(code, 0x35);
    }

    #[test]
    fn black_term_zero_is_10b_0x37() {
        // ITU-T T.6 Table 1: black terminating, run length 0 = 0000110111 (10 bits)
        let (length, code, _payload) = BLACK_TERM_ENTRIES[0];
        assert_eq!(length, 10);
        assert_eq!(code, 0x37);
    }

    #[test]
    fn two_d_v0_is_1b_1() {
        // ITU-T T.6 Table 3: V(0) = 1 (1 bit)
        let (length, code) = TWO_D[3]; // V(0) is index 3 in DISPATCH order
        assert_eq!(length, 1);
        assert_eq!(code, 0x01);
    }

    #[test]
    fn dispatch_order_matches_two_d() {
        assert_eq!(DISPATCH.len(), TWO_D.len());
        // Spot-check a few — full check would replicate the table.
        assert_eq!(DISPATCH[3], DispatchEntry::V(0));
        assert_eq!(DISPATCH[7], DispatchEntry::H);
        assert_eq!(DISPATCH[8], DispatchEntry::P);
    }

    #[test]
    fn tab7_lookup_resolves_v0() {
        // V(0) = code `1` of length 1. With 7-bit lookup, the entry at index
        // 0b1xxxxxx (= 0x40..=0x7F) should map to (DISPATCH idx 3, length 1).
        for top7 in 0x40..=0x7F {
            let entry = TAB7[top7 as usize].expect("V0 must populate top half");
            assert_eq!(entry.dispatch_idx, 3);
            assert_eq!(entry.code_len, 1);
        }
    }

    #[test]
    fn white_table_decodes_short_run() {
        // White run-length 2 = 0111 (4 bits) per ITU-T T.6 Table 1.
        // Top-13-bit lookup at 0b0111000000000 (0x0E00) should resolve.
        let entry = WHITE_TABLE[0x0E00].expect("white run 2 must lookup");
        assert_eq!(entry.run, 2);
        assert_eq!(entry.code_len, 4);
    }
}
```

- [ ] **Step 2: Create `src/ccitt.rs`**

```rust
//! CCITT-T.6 (Group 4 fax) Huffman tables, derived from the ITU-T T.6
//! Recommendation (07/88). Tables 1/T.6 (terminating run codes),
//! 2/T.6 (make-up run codes), and 3/T.6 (two-dimensional codes).
//!
//! Provenance: each table value here was verified against the ITU-T T.6
//! specification PDF (https://www.itu.int/rec/T-REC-T.6). The arrays are
//! NOT copied from paperman, max2pdf, or any other GPL source.

/// Two-dimensional code dispatch entries (V(-3)..V(+3), H, P).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DispatchEntry {
    /// Vertical mode V(n) where n ∈ {-3, -2, -1, 0, +1, +2, +3}.
    V(i8),
    /// Horizontal mode (followed by two run-length codes).
    H,
    /// Pass mode.
    P,
}

/// Two-dimensional codes from ITU-T T.6 Table 3 in Python `DISPATCH` order:
/// V_L3, V_L2, V_L1, V0, V_R1, V_R2, V_R3, H, P.
///
/// Each entry: `(length_bits, code_bits)`.
pub(crate) const TWO_D: [(u32, u32); 9] = [
    (7, 0x02), // V(-3) = 0000010
    (6, 0x02), // V(-2) = 000010
    (3, 0x02), // V(-1) = 010
    (1, 0x01), // V(0)  = 1
    (3, 0x03), // V(+1) = 011
    (6, 0x03), // V(+2) = 000011
    (7, 0x03), // V(+3) = 0000011
    (3, 0x01), // H     = 001
    (4, 0x01), // P     = 0001
];

/// Dispatch interpretation of each TWO_D entry, in matching index order.
pub(crate) const DISPATCH: [DispatchEntry; 9] = [
    DispatchEntry::V(-3),
    DispatchEntry::V(-2),
    DispatchEntry::V(-1),
    DispatchEntry::V(0),
    DispatchEntry::V(1),
    DispatchEntry::V(2),
    DispatchEntry::V(3),
    DispatchEntry::H,
    DispatchEntry::P,
];

/// White terminating codes, ITU-T T.6 Table 1, runs 0..=63.
/// Each entry: `(length_bits, code_bits, run_length_payload)`.
pub(crate) const WHITE_TERM_ENTRIES: &[(u32, u32, u32)] = &[
    // ⚠ Engineer: transcribe ALL 64 rows from ITU-T T.6 Table 1 white-runs
    // section. Cross-check against max2pdf.py:46-58. Do NOT copy from
    // paperman_btab.dat. Each row: (length, code_value, run_length=row_index).
    // Example for run 0:  (8, 0x35, 0)   // 00110101
    // Example for run 1:  (6, 0x07, 1)   // 000111
    // Example for run 63: (8, 0x34, 63)  // 00110100
];

/// White make-up codes, ITU-T T.6 Table 2, runs 64, 128, 192, ..., 1728
/// followed by extended codes 1792, 1856, ..., 2560.
pub(crate) const WHITE_MAKEUP_ENTRIES: &[(u32, u32, u32)] = &[
    // ⚠ Engineer: 27 standard + 13 extended = 40 entries. Run-length
    // payload = 64 * (index + 1) for indices 0..=26, then 1792, 1856,
    // 1920, 1984, 2048, 2112, 2176, 2240, 2304, 2368, 2432, 2496, 2560
    // for the extended group. Cross-check against max2pdf.py:59-68.
];

/// Black terminating codes, ITU-T T.6 Table 1, runs 0..=63.
pub(crate) const BLACK_TERM_ENTRIES: &[(u32, u32, u32)] = &[
    // ⚠ Engineer: transcribe ALL 64 rows from ITU-T T.6 Table 1 black-runs.
    // Cross-check against max2pdf.py:69-83.
];

/// Black make-up codes, ITU-T T.6 Table 2, same payload progression as
/// white make-ups (the codes differ; the run-length progression matches).
pub(crate) const BLACK_MAKEUP_ENTRIES: &[(u32, u32, u32)] = &[
    // ⚠ Engineer: 27 standard + 13 extended. Cross-check against
    // max2pdf.py:84-94. Note the extended codes are SHARED with white
    // (positions 27..40 in both tables are identical).
];

/// 7-bit dispatcher table entry: which DISPATCH index, and how many bits
/// the matched code consumed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DispatchHit {
    pub dispatch_idx: u8,
    pub code_len: u32,
}

/// 13-bit run-length lookup entry.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RunHit {
    pub run: u32,
    pub code_len: u32,
}

/// 7-bit dispatcher table built from TWO_D: index by top 7 bits of stream.
pub(crate) static TAB7: std::sync::LazyLock<[Option<DispatchHit>; 128]> =
    std::sync::LazyLock::new(|| {
        let mut table: [Option<DispatchHit>; 128] = [None; 128];
        for (idx, &(length, code)) in TWO_D.iter().enumerate() {
            // Pad code to 7 bits (left-shift), fill all suffixes.
            let pad = 7 - length;
            let base = (code << pad) as usize;
            let span = 1usize << pad;
            for j in 0..span {
                table[base + j] = Some(DispatchHit {
                    dispatch_idx: idx as u8,
                    code_len: length,
                });
            }
        }
        table
    });

/// 13-bit white run-length lookup. Combines terminating + make-up codes.
/// On a make-up hit, `run >= 64`; caller loops back to read another code.
pub(crate) static WHITE_TABLE: std::sync::LazyLock<Vec<Option<RunHit>>> =
    std::sync::LazyLock::new(|| build_run_table(WHITE_TERM_ENTRIES, WHITE_MAKEUP_ENTRIES));

/// 13-bit black run-length lookup.
pub(crate) static BLACK_TABLE: std::sync::LazyLock<Vec<Option<RunHit>>> =
    std::sync::LazyLock::new(|| build_run_table(BLACK_TERM_ENTRIES, BLACK_MAKEUP_ENTRIES));

fn build_run_table(
    term: &[(u32, u32, u32)],
    makeup: &[(u32, u32, u32)],
) -> Vec<Option<RunHit>> {
    let mut table: Vec<Option<RunHit>> = vec![None; 1 << 13];
    for entries in [term, makeup] {
        for &(length, code, run) in entries {
            let pad = 13 - length;
            let base = (code << pad) as usize;
            let span = 1usize << pad;
            for j in 0..span {
                if table[base + j].is_none() {
                    table[base + j] = Some(RunHit { run, code_len: length });
                }
            }
        }
    }
    table
}

// (insert tests here per Step 1)
```

- [ ] **Step 3: Transcribe the table values**

This is the manual step that justifies the legal posture. Open ITU-T T.6 Recommendation PDF (https://www.itu.int/rec/T-REC-T.6) → Tables 1 and 2. For each row, record `(code_length_bits, code_value, run_length)`. Cross-check against `max2pdf.py:46-94` — if a value differs, ITU PDF wins.

When complete, add a header comment to `src/ccitt.rs` immediately above `WHITE_TERM_ENTRIES`:

```rust
// Transcribed from ITU-T Recommendation T.6 (07/88), Table 1/T.6,
// verified <YYYY-MM-DD> by <name>. Cross-checked against
// max2pdf.py:46-94 — values match.
```

(Replace placeholders.)

- [ ] **Step 4: Wire into `src/lib.rs`**

Append:

```rust
mod ccitt;
```

(Not re-exported.)

- [ ] **Step 5: Run tests**

Run: `cargo test --lib ccitt`
Expected: `6 passed; 0 failed`

If any "must lookup"/"must populate" assertion fails, a table value is wrong — re-verify against the ITU PDF before continuing.

- [ ] **Step 6: Commit**

```powershell
git add src/ccitt.rs src/lib.rs
git commit -m @'
feat: add CCITT-T.6 tables (ITU-T T.6 derivation)

White/black terminating + make-up runs and 2D codes (V/H/P), each
verified against ITU-T Recommendation T.6 (07/88). 7-bit dispatcher
TAB7 and 13-bit run-length lookups WHITE_TABLE / BLACK_TABLE built
at first use via LazyLock.

No values copied from paperman or max2pdf (GPL-2-or-later) — the
tables are facts from a public ITU standard. See docs/provenance.md
(added in Task 16).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 6: Chunk discovery (`find_image_chunks`)

**Files:**
- Create: `src/chunks.rs`, `tests/chunks.rs`
- Modify: `src/lib.rs`

**Reference:** `max2pdf.py:809-822` (`find_image_chunks`).

The Python function finds all `DL`-tagged image chunks in the byte stream by scanning for the `b'DL'` magic and validating the chunk header's flags field.

- [ ] **Step 1: Write the failing unit test**

Add to bottom of `src/chunks.rs` after Step 2:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_no_chunks_in_empty_buffer() {
        let chunks = find_image_chunks(&[]);
        assert!(chunks.is_empty());
    }

    #[test]
    fn finds_no_chunks_when_magic_absent() {
        let data = vec![0u8; 256];
        let chunks = find_image_chunks(&data);
        assert!(chunks.is_empty());
    }

    #[test]
    fn finds_single_synthetic_chunk() {
        // DL header: b'DL' + length (u32 LE) + flags (u32 LE).
        // For a valid image chunk, (flags & 0xFFFF) must == 0x4000 AND
        // (flags >> 16) > 0 AND length must fit.
        let mut data = vec![0u8; 256];
        let chunk_offset = 0x40usize;
        data[chunk_offset] = b'D';
        data[chunk_offset + 1] = b'L';
        data[chunk_offset + 2..chunk_offset + 6].copy_from_slice(&64u32.to_le_bytes()); // length
        data[chunk_offset + 6..chunk_offset + 10].copy_from_slice(&0x0001_4000u32.to_le_bytes()); // flags

        let chunks = find_image_chunks(&data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].offset, chunk_offset);
        assert_eq!(chunks[0].length, 64);
    }

    #[test]
    fn skips_non_image_dl_chunks() {
        // Same DL magic but flags low-16 != 0x4000 ⇒ not an image chunk.
        let mut data = vec![0u8; 256];
        data[0x10] = b'D';
        data[0x11] = b'L';
        data[0x12..0x16].copy_from_slice(&64u32.to_le_bytes());
        data[0x16..0x1A].copy_from_slice(&0x0001_2000u32.to_le_bytes()); // wrong tag
        let chunks = find_image_chunks(&data);
        assert!(chunks.is_empty());
    }

    #[test]
    fn finds_two_back_to_back_chunks() {
        let mut data = vec![0u8; 512];
        for (i, off) in [0x00usize, 0x80usize].iter().enumerate() {
            data[*off] = b'D';
            data[*off + 1] = b'L';
            data[*off + 2..*off + 6].copy_from_slice(&0x80u32.to_le_bytes());
            data[*off + 6..*off + 10].copy_from_slice(&0x0001_4000u32.to_le_bytes());
            let _ = i;
        }
        let chunks = find_image_chunks(&data);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].offset, 0x00);
        assert_eq!(chunks[1].offset, 0x80);
    }
}
```

- [ ] **Step 2: Create `src/chunks.rs`**

```rust
//! `.max` container chunk discovery.
//!
//! PaperPort 2 stores each scanned page as a DL-tagged chunk. Image chunks
//! are identified by the low 16 bits of the flags word being `0x4000`
//! (image tag) AND the high 16 bits being non-zero (page index).

/// A discovered image chunk in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChunkRef {
    /// Byte offset of the `b'DL'` magic in the file.
    pub offset: usize,
    /// Total length of the chunk in bytes (including the 10-byte header).
    pub length: usize,
}

/// Scan `data` for image chunks. Mirrors `max2pdf.py:find_image_chunks`.
pub(crate) fn find_image_chunks(data: &[u8]) -> Vec<ChunkRef> {
    let mut out = Vec::new();
    let n = data.len();
    if n < 8 {
        return out;
    }
    let mut pos = 0usize;
    while pos + 10 <= n {
        if &data[pos..pos + 2] == b"DL" {
            let length = u32::from_le_bytes(data[pos + 2..pos + 6].try_into().unwrap()) as usize;
            let flags = u32::from_le_bytes(data[pos + 6..pos + 10].try_into().unwrap());
            let tag = flags & 0xFFFF;
            let page_index = flags >> 16;
            if tag == 0x4000 && page_index > 0 && length > 0 && length <= n - pos {
                out.push(ChunkRef { offset: pos, length });
                pos += length;
                continue;
            }
        }
        pos += 1;
    }
    out
}

// (insert tests here per Step 1)
```

- [ ] **Step 3: Wire into `src/lib.rs`**

Append:

```rust
mod chunks;
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib chunks`
Expected: `5 passed; 0 failed`

- [ ] **Step 5: Commit**

```powershell
git add src/chunks.rs src/lib.rs
git commit -m @'
feat: add image chunk discovery

Mirrors max2pdf.py:find_image_chunks. Image chunks identified by
DL magic + flags low16=0x4000 + page-index>0.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 7: `decomp_line` — port the per-line CCITT decoder

**Files:**
- Create: `src/decoder.rs`
- Modify: `src/lib.rs`

**Reference:** `max2pdf.py:201-329` (`_decomp_line`). This is the most subtle function in the codebase — it incorporates **bug4** (the canonical reference-table walk fix from session 12-2nd-pass) and **lazy bit loading** (session 11-6th-pass).

### Translation gotchas (read before writing)

The Python source captures every gotcha in inline comments. Pay special attention to:

1. **`first_iter` skip** (Python lines 240-253): on the very first iteration, the legacy non-bug4 path skips its scan-forward over the reference table. Without this, a black starting at column 0 (where `x == table_prev[1]`) would skip past the valid first b1.
2. **bug4 V code** (Python lines 303-319): canonical `lodsw` advances `tp_idx` by +1 per V code. Plus a b2-skip for V_R{1,2,3} (1 step for V_R1/V_R2, 2 steps for V_R3) when `voff > 0` and the colour change actually happened (`x < width`).
3. **bug4 P code** (Python lines 297-301): canonical `add si, 2; lodsw` ⇒ `tp_idx += 2`.
4. **bug4 H walk-forward** (Python lines 291-295): after H emits two run codes, walk `tp_idx` forward past consumed entries (`while ref[tp_idx] <= x`).
5. **Initial `x = 0`** (Python line 237): canonical `seg2:0xD68 sub ax,ax` zeros a0. Earlier (pre-12th-session) versions had `x = -1`.
6. **`safety` watchdog** (Python lines 241-243): if a malformed line hangs the decoder, return all-white after `width * 4 + 100` iterations.
7. **Return tuple** (Python line 329): `(out, (pos - start_pos) * 8 - bits_left)`. The bit count is `cursor.next_load_byte() * 8 - cursor.bits_buffered() - start_pos * 8`. **Note: `pos` in Python is the absolute byte offset; in our `BitCursor` we constructed with `with_start(data, start_pos)` and `next_load_byte()` returns the absolute offset, so the formula is `(cursor.next_load_byte() - start_pos) * 8 - cursor.bits_buffered() as i64`.**
8. **All-white fallback** (Python multiple sites): on FAIL the function returns `[-1, width, width, width]` (4 entries), not all-white pixels — the caller `_table_to_row` interprets this as "no transitions, all white".

### Output type

Python returns a *changing-elements table*: a list of x-coordinates where the colour changes, prefixed by `-1` and suffixed with `[width, width]`. The Rust port uses `Vec<i32>` with the same shape (i32 because of the leading `-1`).

- [ ] **Step 1: Write the failing tests**

Add to bottom of `src/decoder.rs` after Step 2:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_white_line_returns_minimal_table() {
        // A line that's entirely white relative to an all-white reference
        // emits a single V(0) code at end-of-line. With ref = all-white
        // sentinel [width, width, ...], the first iteration matches V(0)
        // and exits.
        let width: i32 = 16;
        let ref_table: Vec<i32> = vec![-1, width, width, width, width, width, width, width, width, width, width, width, width, width, width, width, width];
        // Encode V(0): 1 bit = 0b1, padded to a byte = 0x80.
        let data = [0x80u8];
        let (table, consumed) = decomp_line(&data, 0, width, &ref_table, false, true);
        assert!(consumed >= 1);
        // Result table: [-1, then either no real transitions, or width-flagged].
        // Sanity: must end in [..., width, width].
        let last_two = &table[table.len() - 2..];
        assert_eq!(last_two, &[width, width]);
    }

    #[test]
    fn fail_returns_all_white_fallback() {
        // Empty input must FAIL inside the decoder and return the [-1, w, w, w]
        // sentinel that the caller treats as "all white".
        let width: i32 = 16;
        let ref_table = vec![-1, width, width, width, width];
        let (table, _consumed) = decomp_line(&[], 0, width, &ref_table, false, true);
        assert_eq!(table, vec![-1, width, width, width]);
    }

    // Integration tests against synthetic fixture come in Task 9.
    // This module's correctness is fully gated by the synthetic round-trip.
}
```

- [ ] **Step 2: Create `src/decoder.rs`**

```rust
//! Per-line CCITT-T.6 decoder. Direct port of `max2pdf.py:_decomp_line`.

use crate::bitstream::BitCursor;
use crate::ccitt::{DispatchEntry, BLACK_TABLE, DISPATCH, TAB7, WHITE_TABLE};

/// Decode one CCITT-T.6 line starting at byte boundary `start_pos`.
///
/// Returns `(changing_elements_table, bits_consumed)`. On any decode
/// failure (bit underrun, unknown code, watchdog timeout) returns
/// `([-1, width, width, width], bits_consumed_so_far)` — the caller
/// `table_to_row` reads this as "all white".
///
/// - `lazy = true` ⇒ byte-by-byte refill (Python `_refill_lazy`).
/// - `bug4 = true` ⇒ canonical reference-table walk (default; produces
///   IoU=1.000 on the corpus). `false` reproduces the pre-12th-session
///   `tp_idx -= 1 + scan-forward` behaviour for diagnostic comparison.
pub(crate) fn decomp_line(
    data: &[u8],
    start_pos: usize,
    width: i32,
    table_prev: &[i32],
    lazy: bool,
    bug4: bool,
) -> (Vec<i32>, i64) {
    let mut bc = BitCursor::with_start(data, start_pos, lazy);
    // ⚠ Engineer: port max2pdf.py:201-329 here. Preserve every gotcha called
    // out in the "Translation gotchas" section above. Use Self::fail_table()
    // helper for early returns.
    //
    // The full port is straightforward once each gotcha is internalised;
    // it's ~120 lines of Rust matching the Python line-for-line. Compute
    // bits_consumed at exit as:
    //
    //   ((bc.next_load_byte() - start_pos) as i64) * 8
    //       - bc.bits_buffered() as i64
    //
    // Skeleton:
    let mut out: Vec<i32> = vec![-1];
    let mut tp_idx: usize = 1;
    let mut colour: u32 = 0;
    let mut x: i32 = 0;  // canonical seg2:0xD68 sub ax,ax
    let mut safety: i32 = 0;
    let mut first_iter = true;
    let safety_limit = width * 4 + 100;

    while x < width {
        safety += 1;
        if safety > safety_limit {
            return fail_table(width, &bc, start_pos);
        }

        if !bug4 && !first_iter {
            // Legacy scan-forward at iteration start.
            while (tp_idx < table_prev.len()) && (table_prev[tp_idx] <= x) {
                tp_idx += 2;
            }
        }
        first_iter = false;

        let top7 = match bc.peek(7) {
            Some(v) => v,
            None => return fail_table(width, &bc, start_pos),
        };
        let entry = match TAB7[top7 as usize] {
            Some(e) => e,
            None => return fail_table(width, &bc, start_pos),
        };
        bc.consume(entry.code_len);
        let dispatch = DISPATCH[entry.dispatch_idx as usize];

        match dispatch {
            DispatchEntry::H => {
                // Read two run codes (alternating colour).
                for _ in 0..2 {
                    loop {
                        let top13 = match bc.peek(13) {
                            Some(v) => v,
                            None => return fail_table(width, &bc, start_pos),
                        };
                        let table = if colour == 0 { &*WHITE_TABLE } else { &*BLACK_TABLE };
                        let hit = match table[top13 as usize] {
                            Some(h) => h,
                            None => return fail_table(width, &bc, start_pos),
                        };
                        bc.consume(hit.code_len);
                        x += hit.run as i32;
                        if hit.run <= 63 {
                            break;
                        }
                    }
                    out.push(x);
                    colour ^= 1;
                }
                if bug4 {
                    // H walk-forward (canonical seg2:0x154D).
                    while (tp_idx < table_prev.len()) && (table_prev[tp_idx] <= x) {
                        tp_idx += 2;
                    }
                }
            }
            DispatchEntry::P => {
                if tp_idx + 1 >= table_prev.len() {
                    return fail_table(width, &bc, start_pos);
                }
                x = table_prev[tp_idx + 1];
                if bug4 {
                    tp_idx += 2;  // canonical add si,2; lodsw
                }
            }
            DispatchEntry::V(voff) => {
                if bug4 {
                    if tp_idx >= table_prev.len() {
                        return fail_table(width, &bc, start_pos);
                    }
                    let b1 = table_prev[tp_idx];
                    x = b1 + voff as i32;
                    out.push(x);
                    tp_idx += 1; // canonical lodsw
                    if voff > 0 && x < width {
                        let max_skips = if voff == 3 { 2 } else { 1 };
                        for _ in 0..max_skips {
                            if (tp_idx < table_prev.len()) && (x >= table_prev[tp_idx]) {
                                tp_idx += 2;
                            } else {
                                break;
                            }
                        }
                    }
                    if x < width {
                        colour ^= 1;
                    }
                } else {
                    if tp_idx >= table_prev.len() {
                        return fail_table(width, &bc, start_pos);
                    }
                    x = table_prev[tp_idx] + voff as i32;
                    out.push(x);
                    if x < width {
                        if tp_idx >= 1 {
                            tp_idx -= 1;
                        }
                        colour ^= 1;
                    }
                }
            }
        }
    }

    out.push(width);
    out.push(width);
    let consumed = ((bc.next_load_byte() - start_pos) as i64) * 8 - bc.bits_buffered() as i64;
    (out, consumed)
}

fn fail_table(width: i32, bc: &BitCursor<'_>, start_pos: usize) -> (Vec<i32>, i64) {
    let consumed = ((bc.next_load_byte() - start_pos) as i64) * 8 - bc.bits_buffered() as i64;
    (vec![-1, width, width, width], consumed)
}

/// Convert a changing-elements table to a packed 1-bit MSB-first row.
///
/// Mirrors `max2pdf.py:_table_to_row` (line 332). `row_bytes` is the
/// padded byte width of the output row.
pub(crate) fn table_to_row(table: &[i32], width: i32, row_bytes: usize) -> Vec<u8> {
    let mut out = vec![0u8; row_bytes];
    let mut i = 1usize;
    let n = table.len();
    while i + 1 < n {
        let start = table[i].max(0);
        let mut end = table[i + 1];
        if start >= width { break; }
        if end > width { end = width; }
        if end > start {
            let sb = (start >> 3) as usize;
            let eb = ((end - 1) >> 3) as usize;
            if sb == eb {
                let lo = (start & 7) as u32;
                let hi = if (end & 7) == 0 { 8 } else { (end & 7) as u32 };
                let mask = ((0xFFu8 >> lo) & ((0xFFu8 << (8 - hi)) & 0xFF)) & 0xFF;
                out[sb] |= mask;
            } else {
                out[sb] |= (0xFFu8 >> (start & 7) as u32) & 0xFF;
                for b in (sb + 1)..eb {
                    out[b] = 0xFF;
                }
                let rem = (end & 7) as u32;
                out[eb] = if rem == 0 {
                    0xFF
                } else {
                    out[eb] | ((0xFFu8 << (8 - rem)) & 0xFF)
                };
            }
        }
        i += 2;
    }
    out
}

/// Build a changing-elements table from a packed 1-bit MSB-first row
/// (1 = black). Mirrors `max2pdf.py:_table_from_raw` (line 360).
pub(crate) fn table_from_raw(row: &[u8], width: i32) -> Vec<i32> {
    let mut out: Vec<i32> = vec![-1];
    let mut colour: u32 = 0;
    for x in 0..width {
        let bit = (row[(x >> 3) as usize] >> (7 - (x & 7) as u32)) & 1;
        if bit as u32 != colour {
            out.push(x);
            colour ^= 1;
        }
    }
    out.push(width);
    out.push(width);
    out
}

// (insert tests here per Step 1)
```

- [ ] **Step 3: Wire into `src/lib.rs`**

Append:

```rust
mod decoder;
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib decoder`
Expected: `2 passed; 0 failed`

(Full correctness gating happens in Task 9 via the synthetic round-trip.)

- [ ] **Step 5: Commit**

```powershell
git add src/decoder.rs src/lib.rs
git commit -m @'
feat: add decomp_line + table_to_row + table_from_raw

Direct port of max2pdf.py:201-370. bug4 (default true) implements the
canonical reference-table walk; lazy (default false) is for diagnostic
matching of PaperPort 3.6 byte-by-byte refill timing.

Unit tests are minimal — full correctness is gated by the synthetic
round-trip in Task 9.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 8: Test-only encoder + synthetic fixture

**Files:**
- Create: `tests/common/mod.rs`, `tests/common/encoder.rs`, `tools/encode-fixture/main.rs` (a small dev binary), `tests/fixtures/synthetic.max`, `tests/fixtures/synthetic.pbm`

**Reference:** `C:\Users\chris\Desktop\Alte Scans\encoder_validator.py:encode_row` (and surrounding ~298 lines).

### Bootstrap rationale

The synthetic test depends on the encoder being correct. The encoder is bootstrapped by:
1. Writing the encoder.
2. Generating `tests/fixtures/synthetic.max` once locally.
3. Validating the fixture by running the **Python decoder** (`max2pdf.py`) on it and comparing output against the source bitmap.
4. Committing the validated fixture.

After bootstrap, the Rust decoder is tested against the encoder's output transitively via the fixture. If the Rust decoder regresses, the fixture-based test fails. If both encoder+decoder regress in compensating ways, Python verification still catches it.

- [ ] **Step 1: Create the fixture binary at `tools/encode-fixture/main.rs`**

This is a one-time dev tool. Add to `Cargo.toml` under existing `[[bin]]`:

```toml
[[bin]]
name = "encode-fixture"
path = "tools/encode-fixture/main.rs"
required-features = []
```

Then create `tools/encode-fixture/main.rs`:

```rust
//! One-time fixture generator. Produces `tests/fixtures/synthetic.max` +
//! `tests/fixtures/synthetic.pbm` from a programmatic 200x100 bitmap that
//! exercises the white-run, black-run, V/P/H mode mix used by the decoder.
//!
//! After running this once and validating the output via the Python
//! decoder (`python max2pdf.py tests/fixtures/synthetic.max -o /tmp`),
//! commit both files.
//!
//! Re-run only if the synthetic pattern needs to change.

use std::path::Path;

fn build_synthetic_bitmap(width: usize, height: usize) -> Vec<u8> {
    // Pack 1-bit MSB-first, bit=1 means BLACK (matches decoder polarity).
    let row_bytes = (width + 7) / 8;
    let mut bits = vec![0u8; row_bytes * height];

    let set = |bits: &mut [u8], x: usize, y: usize| {
        bits[y * row_bytes + (x >> 3)] |= 0x80 >> (x & 7);
    };

    // Pattern that exercises every dispatch mode:
    // - Top quarter: 8x8 checkerboard (lots of V codes).
    // - Middle: horizontal black bars 2 px tall (long white runs + H mode).
    // - Bottom: sparse single black pixels (long white runs).
    for y in 0..height {
        if y < height / 4 {
            // Checkerboard
            for x in 0..width {
                if ((x / 8) + (y / 8)) & 1 == 0 {
                    set(&mut bits, x, y);
                }
            }
        } else if y < 3 * height / 4 {
            // Horizontal bars every 10 rows
            if (y - height / 4) % 10 < 2 {
                for x in 0..width {
                    set(&mut bits, x, y);
                }
            }
        } else {
            // Sparse pixels at deterministic positions
            for x in (5..width).step_by(17) {
                set(&mut bits, x, y);
            }
        }
    }
    bits
}

fn write_pbm_p4(path: &Path, width: usize, height: usize, bits: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    write!(f, "P4\n{} {}\n", width, height)?;
    f.write_all(bits)?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let width = 200;
    let height = 100;
    let bits = build_synthetic_bitmap(width, height);

    std::fs::create_dir_all("tests/fixtures")?;
    write_pbm_p4(Path::new("tests/fixtures/synthetic.pbm"), width, height, &bits)?;

    // Encode to .max using the test-only encoder. Write minimal ViGBe
    // wrapper + DL chunk + CCITT-T.6 line stream.
    let max_bytes = encode_synthetic_max(&bits, width as u32, height as u32);
    std::fs::write("tests/fixtures/synthetic.max", &max_bytes)?;

    eprintln!("wrote tests/fixtures/synthetic.max ({} bytes) and synthetic.pbm",
              max_bytes.len());
    eprintln!();
    eprintln!("Verify with the Python decoder:");
    eprintln!("  python C:/Users/chris/Desktop/'Alte Scans'/max2pdf.py \\");
    eprintln!("    tests/fixtures/synthetic.max -o tests/fixtures/");
    eprintln!("  # then check the produced PDF visually matches synthetic.pbm");
    Ok(())
}

// ⚠ Engineer: implement encode_synthetic_max here. It must produce a
// minimally-conforming ViGBe file:
//   1. 5-byte magic "ViGBe"
//   2. Padding/header bytes to reach the chunk start (study a real .max
//      hex dump or the chunk-discovery code in src/chunks.rs to know
//      what comes between magic and the first DL chunk — most fields
//      can be zeros/defaults).
//   3. One DL chunk:
//      a. b"DL"
//      b. length (u32 LE) — total chunk bytes
//      c. flags (u32 LE) — low16 = 0x4000, high16 = 1 (page index)
//      d. chunk header (study max2pdf.py:decode_image_chunk for the
//         offsets it reads: width at +0x26, height at +0x28, dpi_x/y
//         around +0x2a, preview metadata at +0x3c..0x40, etc. Set width,
//         height, dpi=300, preview_size/x/y = 0)
//      e. CCITT-T.6 line stream: for each row, emit a 1-byte marker
//         (top2=2, low6=0) followed by the encoded line bytes. End-of-
//         line at marker boundary.
//   4. The encoder for a single line takes the previous line's table and
//      the current line's table, walks them computing a0/a1/b1/b2, and
//      emits V/P/H codes per CCITT-T.6 rules (PASS when b2 < a1; V when
//      |a1 - b1| <= 3; H otherwise). Reference: encoder_validator.py:
//      encode_row in C:\Users\chris\Desktop\Alte Scans\.
//
// The encoder is test-only — perfection isn't required. It just has to
// produce output that the Python decoder reads correctly.
//
// Helper: a separate file `tests/common/encoder.rs` (Step 2 below) holds
// the line-encoder; this binary just wraps it with the file/chunk header.

fn encode_synthetic_max(_bits: &[u8], _width: u32, _height: u32) -> Vec<u8> {
    todo!("call into the encoder in tests/common/encoder.rs")
}
```

- [ ] **Step 2: Create `tests/common/mod.rs`**

```rust
//! Shared test helpers (currently: minimal CCITT-T.6 encoder).
pub mod encoder;
```

- [ ] **Step 3: Create `tests/common/encoder.rs`**

Port `encoder_validator.py:encode_row` to Rust. The function signature:

```rust
//! Test-only CCITT-T.6 line encoder. Mirrors
//! `C:\Users\chris\Desktop\Alte Scans\encoder_validator.py:encode_row`.
//!
//! Not optimised, not exposed as part of the library API. Its sole job is
//! to produce `.max` line streams that the Rust decoder can round-trip.

/// Bit-writer that accumulates MSB-first bits and flushes whole bytes.
pub struct BitWriter {
    pub bytes: Vec<u8>,
    pub bit_buf: u32,
    pub bits_in_buf: u32,
}

impl BitWriter {
    pub fn new() -> Self { Self { bytes: Vec::new(), bit_buf: 0, bits_in_buf: 0 } }

    pub fn write(&mut self, code: u32, length: u32) {
        debug_assert!(length >= 1 && length <= 16);
        self.bit_buf = (self.bit_buf << length) | (code & ((1 << length) - 1));
        self.bits_in_buf += length;
        while self.bits_in_buf >= 8 {
            let shift = self.bits_in_buf - 8;
            self.bytes.push(((self.bit_buf >> shift) & 0xFF) as u8);
            self.bits_in_buf -= 8;
            self.bit_buf &= (1 << self.bits_in_buf) - 1;
        }
    }

    /// Pad with zeros to the next byte boundary and return the bytes.
    pub fn finish(mut self) -> Vec<u8> {
        if self.bits_in_buf > 0 {
            let pad = 8 - self.bits_in_buf;
            self.write(0, pad);
        }
        self.bytes
    }
}

/// Encode a single line given the current line's transitions and the
/// previous line's transitions.
///
/// Both tables are in the same shape produced by `decoder::table_from_raw`:
/// `[-1, x0, x1, ..., width, width]`.
///
/// Returns the encoded byte stream for the body of a type-2 line (no
/// marker byte; the caller prepends `0x80`).
pub fn encode_row(curr: &[i32], prev: &[i32], width: i32) -> Vec<u8> {
    // ⚠ Engineer: port encode_row from encoder_validator.py.
    // Key state: a0 = -1 initially; colour = 0 (white); walk b1/b2 from
    // prev based on a0 + colour parity. PASS when b2 < a1; V when
    // |a1 - b1| <= 3 and the V code's b2-skip semantics work out;
    // otherwise H (emit two run-length codes).
    //
    // Use the constants from src/ccitt.rs::TWO_D for V/H/P codes and
    // *_TERM_ENTRIES + *_MAKEUP_ENTRIES for run codes. Note tests/common/
    // is in the integration-test crate, which CAN access pub(crate)
    // items via `vigb_decoder::__test_exports::ccitt::*` only if we
    // re-export them. Simplest path: duplicate the table values inside
    // tests/common/encoder.rs (this is test-only code, the duplication
    // is acceptable; the values come from the same ITU spec).
    //
    // The full implementation is ~150 lines. encoder_validator.py is the
    // direct reference.
    let _ = (curr, prev, width);
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_writer_packs_msb_first() {
        let mut w = BitWriter::new();
        w.write(0b101, 3);     // 101_____
        w.write(0b11, 2);      // 10111___
        w.write(0b001, 3);     // 10111001 -> 0xB9
        let out = w.finish();
        assert_eq!(out, vec![0xB9]);
    }

    #[test]
    fn bit_writer_pads_to_byte() {
        let mut w = BitWriter::new();
        w.write(0b1, 1);
        let out = w.finish();
        // 1 followed by 7 zeros = 0x80
        assert_eq!(out, vec![0x80]);
    }
}
```

- [ ] **Step 4: Implement `encode_synthetic_max` in `tools/encode-fixture/main.rs`**

Replace the `todo!` body with the file-format wrapper. Reference: study a real `.max` file via hex dump (`Format-Hex (Get-Item ...).FullName` in PowerShell on any of `C:\Users\chris\Desktop\Alte Scans\*.max`) to see what bytes precede the first DL chunk and what the chunk header layout looks like. Cross-check against `max2pdf.py:decode_image_chunk` for which header offsets are read.

Provisional structure (verify against a real file):

```rust
fn encode_synthetic_max(bits: &[u8], width: u32, height: u32) -> Vec<u8> {
    let row_bytes_padded = (((width as usize + 7) / 8) + 3) & !3;

    // Build the line stream first (we need its length for the chunk header).
    let mut line_stream: Vec<u8> = Vec::new();
    let src_row_bytes = (width as usize + 7) / 8;
    let prev_white = vec![0u8; src_row_bytes];
    let mut prev_table = vigb_decoder_test_decoder::table_from_raw(&prev_white, width as i32);
    for y in 0..height as usize {
        let row = &bits[y * src_row_bytes .. (y + 1) * src_row_bytes];
        let curr_table = vigb_decoder_test_decoder::table_from_raw(row, width as i32);
        let body = crate::tests::common::encoder::encode_row(&curr_table, &prev_table, width as i32);
        line_stream.push(0x80); // type-2 marker, low6=0
        line_stream.extend_from_slice(&body);
        prev_table = curr_table;
    }

    // ⚠ Engineer: assemble final output:
    //   bytes 0..5: b"ViGBe"
    //   bytes 5..0x40: zero-fill (or copy from a real .max template)
    //   bytes 0x40..: DL chunk:
    //     0x40..0x42: b"DL"
    //     0x42..0x46: length (u32 LE) = 10 (header) + 0x60 (chunk header) + line_stream.len()
    //     0x46..0x4A: flags = 0x0001_4000  (page 1, image tag)
    //     0x4A..0x4A+0x60: chunk header — width@+0x26, height@+0x28,
    //                     dpi_x@+0x2a, dpi_y@+0x2c, preview_size@+0x3c=0
    //     remaining: line_stream
    todo!("assemble file per the comment above")
}
```

(The reference to `vigb_decoder_test_decoder::table_from_raw` won't work directly — the `tools/` binary cannot reach into the integration-test module. Adjust by inlining `table_from_raw` or making it `pub fn` re-exported from the lib crate behind a `#[doc(hidden)]` `pub mod __test` module. Simpler: copy the 12-line `table_from_raw` body inline in this file.)

- [ ] **Step 5: Run the fixture binary and validate**

Run:
```
cargo run --bin encode-fixture
```
Then validate via the Python decoder (assuming Python repo is at the documented location):
```
python "C:/Users/chris/Desktop/Alte Scans/max2pdf.py" tests/fixtures/synthetic.max -o tests/fixtures/
```

Open the produced PDF, visually compare against `tests/fixtures/synthetic.pbm` (open PBM in any image viewer). Patterns must match (checkerboard top, horizontal bars middle, sparse dots bottom).

If the Python decoder errors out or the visual comparison fails, the encoder needs fixing — iterate until both pass.

- [ ] **Step 6: Commit**

```powershell
git add tools/ tests/common/ tests/fixtures/ Cargo.toml
git commit -m @'
feat: add test-only encoder + synthetic fixture

Minimal CCITT-T.6 encoder under tests/common/encoder.rs (not part of
the library API). Standalone fixture binary at tools/encode-fixture
generates tests/fixtures/synthetic.max + .pbm; both committed for
deterministic test runs in CI.

Bootstrap validated against the Python decoder (max2pdf.py).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 9: `decode_image_chunk` — canonical defaults only

**Files:**
- Create: `src/dispatch.rs`, `tests/synthetic.rs`
- Modify: `src/lib.rs`, `src/decoder.rs` (export Page-related types)

**Reference:** `max2pdf.py:437-808` (`decode_image_chunk`). 370 lines. The algorithmic complexity is the per-line marker dispatcher (top-2-bits = type, low-6 = count/payload) with all the heuristic gates.

This task implements **only the canonical-defaults path** — every heuristic flag is left at its default for now. Heuristic flags get wired in Task 10.

### Page struct (refines spec)

```rust
pub struct Page {
    pub width: u32,
    pub height: u32,
    pub dpi_x: u32,
    pub dpi_y: u32,
    pub row_bytes: u32,
    /// 1-bit packed, MSB-first per byte. **Bit value 1 means BLACK.**
    /// Length = `row_bytes * height`.
    pub bitmap: Vec<u8>,
    pub preview: Option<Preview>,   // populated by Task 11
    pub stats: DecodeStats,
}
```

### DecodeStats

```rust
#[derive(Debug, Default, Clone)]
pub struct DecodeStats {
    pub n_ok: u32, pub n_v0: u32, pub n_t0: u32, pub n_t1: u32,
    pub n_fail: u32, pub max_consecutive_fail: u32,
    pub first_fail_y: Option<u32>,
    pub resync_probes: u32, pub resync_hits: u32,
    pub blank_drops_after_drift: u32,
}
```

- [ ] **Step 1: Write the failing integration test**

Create `tests/synthetic.rs`:

```rust
//! Synthetic round-trip integration test.
//!
//! Reads tests/fixtures/synthetic.max + tests/fixtures/synthetic.pbm,
//! decodes the .max via the canonical decoder, and asserts pixel-for-pixel
//! equality.

use std::fs;
use std::path::Path;

use vigb_decoder::{decode_max, Config};

fn read_pbm_p4(path: &Path) -> (u32, u32, Vec<u8>) {
    let bytes = fs::read(path).expect("read synthetic.pbm");
    // Parse P4 header: "P4\n{w} {h}\n" then raw bits.
    let nl1 = bytes.iter().position(|&b| b == b'\n').unwrap();
    assert_eq!(&bytes[..nl1], b"P4");
    let nl2 = bytes[nl1 + 1..].iter().position(|&b| b == b'\n').unwrap() + nl1 + 1;
    let dims = std::str::from_utf8(&bytes[nl1 + 1..nl2]).unwrap();
    let mut parts = dims.split_whitespace();
    let w: u32 = parts.next().unwrap().parse().unwrap();
    let h: u32 = parts.next().unwrap().parse().unwrap();
    (w, h, bytes[nl2 + 1..].to_vec())
}

#[test]
fn synthetic_round_trip_canonical() {
    let max_path = Path::new("tests/fixtures/synthetic.max");
    let pbm_path = Path::new("tests/fixtures/synthetic.pbm");
    let max_bytes = fs::read(max_path).expect("read synthetic.max");
    let (pbm_w, pbm_h, pbm_bits) = read_pbm_p4(pbm_path);

    let cfg = Config::default();
    let pages = decode_max(&max_bytes, &cfg).expect("decode");
    assert_eq!(pages.len(), 1, "synthetic .max has exactly one image chunk");
    let p = &pages[0];

    assert_eq!(p.width, pbm_w);
    assert_eq!(p.height, pbm_h);

    // PBM rows are tightly packed at (w+7)/8 bytes; decoder rows are padded
    // to row_bytes. Compare row-by-row up to the meaningful width.
    let pbm_row_bytes = ((pbm_w + 7) / 8) as usize;
    let dec_row_bytes = p.row_bytes as usize;
    for y in 0..pbm_h as usize {
        let pbm_row = &pbm_bits[y * pbm_row_bytes .. (y + 1) * pbm_row_bytes];
        let dec_row = &p.bitmap[y * dec_row_bytes .. y * dec_row_bytes + pbm_row_bytes];
        assert_eq!(dec_row, pbm_row, "row {y} mismatch");
    }

    // Canonical decoder must produce zero FAIL events on the synthetic.
    assert_eq!(p.stats.n_fail, 0, "FAIL events on synthetic: {:?}", p.stats);
}
```

- [ ] **Step 2: Define `Page`, `Preview`, `DecodeStats` in `src/decoder.rs`**

Add to top of `src/decoder.rs` (above existing `decomp_line`):

```rust
/// A single decoded page.
#[derive(Debug, Clone)]
pub struct Page {
    /// Image width in pixels (significant pixels per row).
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Horizontal DPI from the file (default 300 if unset).
    pub dpi_x: u32,
    /// Vertical DPI from the file (default 300 if unset).
    pub dpi_y: u32,
    /// Bytes per row in `bitmap` (padded to a 4-byte multiple).
    pub row_bytes: u32,
    /// Raw 1-bit packed bitmap, MSB-first per byte.
    /// **Bit value 1 means BLACK.** Length = `row_bytes * height`.
    pub bitmap: Vec<u8>,
    /// Optional embedded preview thumbnail (populated when
    /// `Config::embed_preview` is true and the chunk has one).
    pub preview: Option<Preview>,
    /// Per-page decoder statistics.
    pub stats: DecodeStats,
}

/// Embedded preview thumbnail, decoded and (optionally) upscaled to
/// match the main image's pixel dimensions.
#[derive(Debug, Clone)]
pub struct Preview {
    pub width: u32,
    pub height: u32,
    pub row_bytes: u32,
    /// 1-bit packed, MSB-first, bit=1 means BLACK.
    pub bitmap: Vec<u8>,
}

/// Per-page decoder statistics. Soft-error counters (FAIL events,
/// drift drops, resync probes) live here, not in `MaxError`.
#[derive(Debug, Default, Clone)]
pub struct DecodeStats {
    pub n_ok: u32,
    pub n_v0: u32,
    pub n_t0: u32,
    pub n_t1: u32,
    pub n_fail: u32,
    pub max_consecutive_fail: u32,
    pub first_fail_y: Option<u32>,
    pub resync_probes: u32,
    pub resync_hits: u32,
    pub blank_drops_after_drift: u32,
}
```

- [ ] **Step 3: Create `src/dispatch.rs`**

```rust
//! Per-line marker dispatcher for an image chunk. Direct port of
//! `max2pdf.py:decode_image_chunk`.

use crate::config::Config;
use crate::decoder::{decomp_line, table_from_raw, table_to_row, DecodeStats, Page};

/// Decode one image chunk starting at `chunk_start` in `data`. Returns the
/// rendered `Page` (preview field unset — populated separately in Task 11).
pub(crate) fn decode_image_chunk(
    data: &[u8],
    chunk_start: usize,
    cfg: &Config,
) -> Page {
    // Read chunk header. Mirror max2pdf.py:decode_image_chunk lines that
    // unpack the header. Key offsets (relative to chunk_start):
    //   +0x26 (u16 LE): width
    //   +0x28 (u16 LE): height
    //   +0x2a (u16 LE): dpi_x
    //   +0x2c (u16 LE): dpi_y
    //   +0x60         : start of CCITT-T.6 line stream
    //
    // ⚠ Engineer: confirm these offsets by reading max2pdf.py:437-460
    // (the header-unpack section at the top of decode_image_chunk).
    let width = u16::from_le_bytes(data[chunk_start + 0x26..chunk_start + 0x28].try_into().unwrap()) as u32;
    let height = u16::from_le_bytes(data[chunk_start + 0x28..chunk_start + 0x2a].try_into().unwrap()) as u32;
    let dpi_x = (u16::from_le_bytes(data[chunk_start + 0x2a..chunk_start + 0x2c].try_into().unwrap()) as u32).max(300);
    let dpi_y = (u16::from_le_bytes(data[chunk_start + 0x2c..chunk_start + 0x2e].try_into().unwrap()) as u32).max(300);

    let line_bytes = ((width + 7) / 8) as usize;
    let row_bytes = (line_bytes + 3) & !3;
    let mut bitmap = vec![0u8; row_bytes * height as usize];
    let mut stats = DecodeStats::default();

    // Reference table starts as all-white sentinel.
    let mut ref_table: Vec<i32> = {
        let mut v = vec![-1i32];
        v.extend(std::iter::repeat(width as i32).take(16));
        v
    };

    let mut pos = chunk_start + 0x60; // ⚠ verify offset against Python
    let chunk_end = chunk_start + read_chunk_length(data, chunk_start);
    let mut y: u32 = 0;
    let mut consecutive_fail: u32 = 0;
    let mut prev_dispatch_was_drift = false;

    while y < height && pos < chunk_end {
        // ⚠ Engineer: port the per-line marker dispatcher from
        // max2pdf.py:437-808. Key structure:
        //
        //   marker = data[pos]; pos += 1
        //   type = marker >> 6
        //   low6 = marker & 0x3F
        //
        //   match type {
        //     0 => raw uncompressed line. If cfg.strict_t0 and low6 not in
        //          {1, 3}: drop (count as t0 in stats, set drift flag).
        //          low6 == 1: read line_bytes raw bytes into bitmap row.
        //          low6 == 3: skip-line (consume line_bytes, no output, no
        //                     y advance — verified against seg2:0xD32-0xD38).
        //     1 => single-pixel positions. If cfg.suppress_t1_all: drop and
        //          mark drift. Else: read low6 positions (each u16 LE),
        //          set those pixels.
        //     2 => CCITT-T.6 compressed. low6 should be 0 normally; non-zero
        //          treated per existing Python heuristics. Call
        //          decomp_line(data, pos, width as i32, &ref_table, cfg.lazy_bit_loading, cfg.bug4).
        //          On OK: rasterize via table_to_row, write into bitmap,
        //                 update ref_table = curr_table, advance y, count n_ok.
        //          On FAIL: count n_fail, leave row blank, advance y,
        //                   set drift flag.
        //          pos advance: pos += (consumed_bits + 7) / 8.
        //     3 => blank-line run. If cfg.drop_blank_after_drift and
        //          prev_dispatch_was_drift: drop (count blank_drops_after_drift).
        //          Else: advance y by (low6 + 1) (canonical seg2:0xC68 inc ax).
        //   }
        //
        // For Task 9, only the canonical defaults branch matters; flag
        // branches are no-ops here (config defaults make them inert).
        // Task 10 adds the diagnostic-flag bodies.
        //
        // Stats accounting:
        //   - consecutive_fail++ on FAIL; reset to 0 on OK.
        //   - max_consecutive_fail = max(max_consecutive_fail, consecutive_fail).
        //   - first_fail_y = Some(y) on first FAIL.
        //
        // The full port is ~250 lines of straight-line Rust matching the
        // Python line-for-line. Use match arms aggressively and helper
        // functions for the type-3 advance and the type-2 OK path.
        let _ = (cfg, &mut bitmap, &mut stats, &mut ref_table, &mut pos, &mut y,
                 &mut consecutive_fail, &mut prev_dispatch_was_drift);
        unimplemented!("port decode_image_chunk per the comment above");
    }

    Page {
        width,
        height,
        dpi_x,
        dpi_y,
        row_bytes: row_bytes as u32,
        bitmap,
        preview: None,
        stats,
    }
}

fn read_chunk_length(data: &[u8], chunk_start: usize) -> usize {
    u32::from_le_bytes(data[chunk_start + 2..chunk_start + 6].try_into().unwrap()) as usize
}
```

- [ ] **Step 4: Provide a top-level `decode_max` that ties chunks → dispatch**

Create `src/lib.rs` additions at the bottom (re-exports + a thin orchestration layer):

```rust
mod dispatch;

pub use decoder::{DecodeStats, Page, Preview};

/// Decode all image chunks in a `.max` byte buffer.
pub fn decode_max(data: &[u8], cfg: &Config) -> Result<Vec<Page>> {
    if data.len() < 5 || &data[..5] != b"ViGBe" {
        return Err(MaxError::BadMagic { offset: 0 });
    }
    let chunks = chunks::find_image_chunks(data);
    if chunks.is_empty() {
        return Err(MaxError::Truncated { offset: 0, need: 0x40, have: data.len() });
    }
    let mut out = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let page = dispatch::decode_image_chunk(data, chunk.offset, cfg);
        out.push(page);
    }
    Ok(out)
}

/// Decode a `.max` file from disk.
pub fn decode_max_file<P: AsRef<std::path::Path>>(path: P, cfg: &Config) -> Result<Vec<Page>> {
    let data = std::fs::read(path)?;
    decode_max(&data, cfg)
}
```

- [ ] **Step 5: Run the test**

Run: `cargo test --test synthetic synthetic_round_trip_canonical -- --nocapture`
Expected: PASS, with `n_fail == 0`.

If FAIL: the synthetic fixture decodes wrong. Diagnostic path:
1. Compare with Python: `python C:/Users/chris/Desktop/'Alte Scans'/max2pdf.py tests/fixtures/synthetic.max -o /tmp` and verify Python still decodes correctly.
2. Most likely a chunk-header offset wrong, or a bug4/lazy flag mishandled in `decomp_line`.

- [ ] **Step 6: Run the full test suite**

Run: `cargo test`
Expected: All previous unit tests still pass + the synthetic round-trip passes.

- [ ] **Step 7: Commit**

```powershell
git add src/dispatch.rs src/decoder.rs src/lib.rs tests/synthetic.rs
git commit -m @'
feat: add canonical decode_image_chunk + decode_max entry points

Per-line marker dispatcher (4 line types: raw, single-pixel,
CCITT-T.6 compressed, blank-run) with canonical defaults only —
heuristic flag branches stubbed for Task 10.

Synthetic round-trip integration test passes (n_fail == 0).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 10: Heuristic flag branches

**Files:**
- Modify: `src/dispatch.rs`, `src/decoder.rs` (add `_resync_probe`)

**Reference:** `max2pdf.py:383-435` (`_resync_probe`) and 437-808 (`decode_image_chunk` heuristic branches).

This task fleshes out every diagnostic flag stubbed in Task 9. The synthetic test must continue to pass with default config; new tests verify each flag's documented behaviour.

### Heuristic taxonomy (in order of implementation complexity)

| Flag | Branch | Behaviour |
|---|---|---|
| `t0_reset` | per-chunk | Reset reference table to all-white at chunk start (default behaviour anyway — flag is a no-op vestige; document and leave). |
| `lazy_bit_loading` | passed to `decomp_line` | Already wired in Task 7. Confirm it flows through. |
| `t0_drop_after_drift` (None/Marker/Full) | type-0 branch | After a drift-flag-set dispatch, the next type-0 marker is consumed but its payload may also be dropped. Marker = drop 1 byte; Full = drop 1 + line_bytes. |
| `t0_drop_kinds` | type-0 branch | Restrict `t0_drop_after_drift` to apply only when previous dispatch was one of these kinds. |
| `fail_scan_forward` | type-2 FAIL branch | Up to N bytes after FAIL, scan for next byte that looks like a valid type-2 marker (top2 == 2). Resync pos to that. |
| `suppress_t2_fail_y_in_cascade` | type-2 FAIL branch | If the previous dispatch was also a FAIL (cascade), do not advance y on this FAIL. |
| `fail_resync_max` + `lookahead` + `min_confidence` + `budget` | type-2 FAIL branch | Smart-resync probe: try `[-K..+K]` byte offsets, score each by running `_resync_probe` for `lookahead` lines; pick offset with `n_ok - n_drift >= min_confidence`. Budget caps total probes per page. |
| `reset_ref_after_drift` | post-FAIL | After a FAIL or drift-flagged dispatch, reset `ref_table` to all-white sentinel. |

- [ ] **Step 1: Port `_resync_probe` into `src/decoder.rs`**

Add as a `pub(crate) fn`:

```rust
/// Lookahead probe used by `fail_resync_max`. Walks the dispatcher for up
/// to `n_steps` lines from `start_pos` against `table_prev`; returns the
/// pair `(n_ok, n_drift)` where n_ok counts type-2 OK decodes and n_drift
/// counts FAIL/V0/BAD/T1 events. Self-contained: writes no output.
///
/// Mirrors `max2pdf.py:_resync_probe` (line 383).
pub(crate) fn resync_probe(
    data: &[u8],
    start_pos: usize,
    table_prev: &[i32],
    width: i32,
    line_bytes: usize,
    n_steps: u32,
    bug4: bool,
    lazy: bool,
) -> (u32, u32) {
    // ⚠ Engineer: port the Python implementation. Loop n_steps times,
    // dispatch a line, count outcomes:
    //   type-2 OK         => n_ok++
    //   type-2 FAIL/V0    => n_drift++
    //   type-1            => n_drift++
    //   type-0 not in {1,3} => n_drift++
    //   type-0 == 1/3, type-3, type-2 OK => valid (no n_drift)
    //
    // The probe must NOT modify any external state — keep its own pos
    // and ref_table copies.
    let _ = (data, start_pos, table_prev, width, line_bytes, n_steps, bug4, lazy);
    (0, 0)
}
```

- [ ] **Step 2: Implement each flag branch in `src/dispatch.rs`**

Replace the `unimplemented!("port decode_image_chunk per the comment above")` body with the full dispatcher (the canonical-only Task 9 stub becomes the full version here). The canonical defaults must continue to produce identical output to the Task 9 result on the synthetic fixture.

For each flag, port the Python branch directly. The Python source is the authoritative reference for the exact predicates and side-effect ordering.

- [ ] **Step 3: Write per-flag unit tests**

Create `tests/heuristics.rs`:

```rust
//! Per-flag tests verifying each heuristic flag's documented behaviour.
//! Each test runs against the synthetic fixture (which has zero FAILs
//! under canonical defaults) plus a synthetic edge-case where the flag
//! actually fires.

use std::fs;
use vigb_decoder::{decode_max, Config, T0DropMode};

fn fixture() -> Vec<u8> {
    fs::read("tests/fixtures/synthetic.max").expect("read synthetic.max")
}

#[test]
fn lazy_bit_loading_matches_eager_on_synthetic() {
    let data = fixture();
    let p_eager = &decode_max(&data, &Config::default()).unwrap()[0];
    let cfg_lazy = Config::builder().lazy_bit_loading(true).build();
    let p_lazy = &decode_max(&data, &cfg_lazy).unwrap()[0];
    assert_eq!(p_eager.bitmap, p_lazy.bitmap, "lazy != eager on canonical input");
}

#[test]
fn no_bug4_changes_bitmap_or_matches() {
    // On the synthetic (which has no V_R{1,2,3} runs in extreme positions
    // where the schemes diverge), bug4=false MIGHT match. Just assert it
    // doesn't panic and produces *some* bitmap of the right shape.
    let data = fixture();
    let cfg = Config::builder().bug4(false).build();
    let p = &decode_max(&data, &cfg).unwrap()[0];
    assert_eq!(p.bitmap.len() as u32, p.row_bytes * p.height);
}

#[test]
fn embed_preview_false_yields_no_preview() {
    let data = fixture();
    let cfg = Config::builder().embed_preview(false).build();
    let p = &decode_max(&data, &cfg).unwrap()[0];
    assert!(p.preview.is_none());
}

#[test]
fn t0_drop_mode_parses() {
    use std::str::FromStr;
    assert_eq!(T0DropMode::from_str("marker").unwrap(), T0DropMode::Marker);
    assert_eq!(T0DropMode::from_str("full").unwrap(), T0DropMode::Full);
}

// Smart-resync tests require a synthetic with deliberate FAIL events.
// For v0.1 we accept that smart resync is exercised mainly by the
// local-only corpus tests (Cargo feature `corpus`), and rely on the
// per-flag branch's unit tests in src/dispatch.rs to gate logic
// correctness here.
```

- [ ] **Step 4: Run the full test suite**

Run: `cargo test`
Expected: All Task 9 tests still pass + 4 new heuristic tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src/dispatch.rs src/decoder.rs tests/heuristics.rs
git commit -m @'
feat: wire all decoder heuristic flags

Implements t0-reset, t0-drop-after-drift (None/Marker/Full),
t0-drop-kinds, fail-scan-forward, suppress-t2-fail-y-in-cascade,
smart-resync (fail-resync-max + lookahead + min-confidence + budget),
reset-ref-after-drift. Each branch ports max2pdf.py:437-808 verbatim.

Canonical defaults still produce zero FAIL events on synthetic.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 11: Preview thumbnail decoder

**Files:**
- Create: `src/preview.rs`
- Modify: `src/lib.rs`, `src/dispatch.rs` (call preview decoder when `cfg.embed_preview`)

**Reference:** `max2pdf.py:840-947` (`_decode_preview_rle` + `decode_preview_chunk`).

The preview is a 102×146 (sometimes 105×147) grayscale thumbnail RLE-encoded at the end of each image chunk. Format:
- Top 2 bits = type, low 6 = count.
- Type 0: emit `count*4` zero pixels.
- Type 1: emit `count*4` 0xFF pixels.
- Type 2: read `count` literal bytes; 4 grayscale pixels per byte (`(b >> j) & 3) * 85` for `j ∈ {6,4,2,0}`).
- Type 3: skip.

After decoding, the result is vertically flipped, thresholded to 1-bit, and (optionally) upscaled with nearest-neighbor to A4 dimensions matching the main page.

The Python implementation uses PIL's `Image.frombytes + resize(NEAREST) + point(mode='1')` for a 5–6× speedup over per-pixel loops. The Rust port uses a direct nearest-neighbor loop — no PIL — which is fine: the thumbnail is small (102×146 = 14,892 pixels) so even a naive Rust loop is fast.

- [ ] **Step 1: Write the failing tests**

Add to bottom of `src/preview.rs` after Step 2:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_type0_emits_zeros() {
        // Marker 0b00_000010 = 0x02 ⇒ type=0, count=2 ⇒ 2*4 = 8 zero pixels.
        let buf = [0x02u8];
        let (out, type3) = decode_preview_rle(&buf, 8, 1);
        assert_eq!(out, vec![0u8; 8]);
        assert_eq!(type3, 0);
    }

    #[test]
    fn rle_type1_emits_ff() {
        // 0b01_000001 = 0x41 ⇒ type=1, count=1 ⇒ 4 bytes of 0xFF.
        let buf = [0x41u8];
        let (out, _) = decode_preview_rle(&buf, 4, 1);
        assert_eq!(out, vec![0xFFu8; 4]);
    }

    #[test]
    fn rle_type2_emits_grayscale_quartets() {
        // 0b10_000001 = 0x81 ⇒ type=2, count=1; followed by 1 literal byte.
        // Literal 0xC0 = 0b11_00_00_00 ⇒ pixels = (3,0,0,0)*85 = (255,0,0,0).
        let buf = [0x81u8, 0xC0];
        let (out, _) = decode_preview_rle(&buf, 4, 2);
        assert_eq!(out, vec![255, 0, 0, 0]);
    }

    #[test]
    fn rle_stops_at_total_pixels() {
        // Same type-1 marker but request only 2 pixels.
        let buf = [0x41u8];
        let (out, _) = decode_preview_rle(&buf, 2, 1);
        assert_eq!(out, vec![0xFFu8; 2]);
    }
}
```

- [ ] **Step 2: Create `src/preview.rs`**

```rust
//! Preview thumbnail decoder. Mirrors `max2pdf.py:840-947`.

use crate::decoder::Preview;

/// Decode a preview RLE byte stream. Returns `(grayscale_pixels, type3_count)`.
/// Each pixel is 8 bits, `0` = white, `0xFF` = black.
pub(crate) fn decode_preview_rle(
    buf: &[u8],
    total_pixels: usize,
    max_bytes: usize,
) -> (Vec<u8>, u32) {
    let mut out = Vec::with_capacity(total_pixels);
    let mut pos = 0usize;
    let mut type3 = 0u32;
    let end = max_bytes.min(buf.len());
    while pos < end && out.len() < total_pixels {
        let ch = buf[pos];
        pos += 1;
        let type_ = ch >> 6;
        let count = (ch & 0x3F) as usize;
        match type_ {
            0 => out.extend(std::iter::repeat(0u8).take(count * 4)),
            1 => out.extend(std::iter::repeat(0xFFu8).take(count * 4)),
            2 => {
                for _ in 0..count {
                    if pos >= end { break; }
                    let cb = buf[pos];
                    pos += 1;
                    for j in [6, 4, 2, 0] {
                        out.push(((cb >> j) & 3) * 85);
                    }
                }
            }
            _ => type3 += 1,
        }
    }
    out.truncate(total_pixels);
    (out, type3)
}

/// Decode the preview thumbnail at the end of an image chunk and (when
/// `scale_to_a4`) upscale to the main image's pixel dimensions. Returns
/// `None` if the chunk has no preview metadata.
pub(crate) fn decode_preview_chunk(
    data: &[u8],
    chunk_start: usize,
    chunk_length: usize,
    scale_to_a4: bool,
) -> Option<Preview> {
    let read_u16 = |off: usize| u16::from_le_bytes(data[chunk_start + off..chunk_start + off + 2].try_into().unwrap()) as u32;

    let preview_size = read_u16(0x3c) as usize;
    let preview_x = read_u16(0x3e) as usize;
    let preview_y = read_u16(0x40) as usize;
    if preview_size == 0 || preview_x == 0 || preview_y == 0 {
        return None;
    }
    let main_w = read_u16(0x26) as usize;
    let main_h = read_u16(0x28) as usize;

    let padded_x = (preview_x + 3) & !3;
    let target_pixels = padded_x * preview_y;
    let offset = chunk_start + chunk_length - preview_size;
    let (mut pixels, _type3) = decode_preview_rle(
        &data[offset..chunk_start + chunk_length],
        target_pixels,
        preview_size,
    );
    if pixels.len() < target_pixels {
        pixels.resize(target_pixels, 128);
    }

    // Vertical flip
    let mut rows: Vec<Vec<u8>> = (0..preview_y)
        .map(|i| pixels[i * padded_x..(i + 1) * padded_x].to_vec())
        .collect();
    rows.reverse();
    let flipped: Vec<u8> = rows.into_iter().flatten().collect();

    let (target_w, target_h) = if scale_to_a4 {
        (main_w, main_h)
    } else {
        (preview_x, preview_y)
    };

    let line_bytes = (target_w + 7) / 8;
    let row_bytes = (line_bytes + 3) & !3;
    let mut bitmap = vec![0u8; row_bytes * target_h];

    // Nearest-neighbor upscale + threshold at 128 → 1-bit (1=black).
    for ty in 0..target_h {
        let sy = ty * preview_y / target_h.max(1);
        let src_row = &flipped[sy * padded_x..(sy + 1) * padded_x];
        let dst_row = &mut bitmap[ty * row_bytes..(ty + 1) * row_bytes];
        for tx in 0..target_w {
            let sx = tx * preview_x / target_w.max(1);
            if src_row[sx] >= 128 {
                dst_row[tx >> 3] |= 0x80 >> (tx & 7);
            }
        }
    }

    Some(Preview {
        width: target_w as u32,
        height: target_h as u32,
        row_bytes: row_bytes as u32,
        bitmap,
    })
}

// (insert tests here per Step 1)
```

- [ ] **Step 3: Wire into `src/lib.rs` and `src/dispatch.rs`**

In `src/lib.rs`, append:

```rust
mod preview;
```

In `src/dispatch.rs`, after constructing `Page` but before returning, attach the preview if requested:

```rust
let preview = if cfg.embed_preview {
    crate::preview::decode_preview_chunk(data, chunk_start, chunk_length, true)
} else {
    None
};
let mut page = Page { /* ... */, preview, /* ... */ };
```

This requires `decode_image_chunk` to know the chunk_length — pass it as an additional argument from `decode_max` (which has the `ChunkRef`).

Update `src/lib.rs::decode_max`:

```rust
for chunk in chunks {
    let page = dispatch::decode_image_chunk(data, chunk.offset, chunk.length, cfg);
    out.push(page);
}
```

And update the `decode_image_chunk` signature to take `chunk_length: usize`.

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: All previous + 4 new preview unit tests pass.

The synthetic fixture has `preview_size == 0`, so `decode_preview_chunk` returns `None` for it — `synthetic_round_trip_canonical`'s assertions on `bitmap` are unaffected. If desired, add a separate preview-bearing fixture later.

- [ ] **Step 5: Commit**

```powershell
git add src/preview.rs src/lib.rs src/dispatch.rs
git commit -m @'
feat: add preview thumbnail decoder

102x146 grayscale RLE decoded, vertically flipped, upscaled
nearest-neighbor to main image dimensions, thresholded to 1-bit.
Wired into decode_image_chunk via cfg.embed_preview (default on).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 12: PDF writer

**Files:**
- Create: `src/pdf.rs`
- Modify: `src/lib.rs`

**Reference:** `max2pdf.py:992-1057` (`write_pdf`). 65 lines, hand-written PDF — no PDF crate used or needed.

Each page becomes one PDF page with a 1-bit FlateDecode image XObject scaled to the page's DPI. When a page has a `preview`, it's emitted as a second PDF page right after the main page.

For zlib compression, Rust's standard library does not include it. Options:
1. `flate2` crate (~700 LOC, well-maintained, the de-facto Rust choice).
2. Hand-roll deflate (no — too much code).
3. Skip compression and use `/Filter null` (works, larger PDFs).

**Decision: add `flate2` as a dep.** It's stable, ubiquitous, and one transitive dep (`miniz_oxide` only). Add to `Cargo.toml`:

```toml
[dependencies]
flate2 = "1"
```

This is the only runtime dep beyond `clap` + `thiserror`.

- [ ] **Step 1: Add `flate2` to `Cargo.toml`**

Insert under `[dependencies]`:

```toml
flate2 = "1"
```

- [ ] **Step 2: Write the failing test**

Add to bottom of `src/pdf.rs` after Step 3:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::{DecodeStats, Page};

    fn make_test_page(width: u32, height: u32) -> Page {
        let row_bytes = (((width as usize + 7) / 8) + 3) & !3;
        let bitmap = vec![0u8; row_bytes * height as usize];
        Page {
            width,
            height,
            dpi_x: 300,
            dpi_y: 300,
            row_bytes: row_bytes as u32,
            bitmap,
            preview: None,
            stats: DecodeStats::default(),
        }
    }

    #[test]
    fn pdf_has_valid_header_and_trailer() {
        let pages = vec![make_test_page(100, 100)];
        let bytes = write_pdf_bytes(&pages, &PdfOptions::default());
        assert!(bytes.starts_with(b"%PDF-1.4\n"));
        assert!(bytes.windows(7).any(|w| w == b"trailer"));
        assert!(bytes.windows(9).any(|w| w == b"startxref"));
        assert!(bytes.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn pdf_xref_is_well_formed() {
        let pages = vec![make_test_page(50, 50)];
        let bytes = write_pdf_bytes(&pages, &PdfOptions::default());
        // Find xref section
        let xref_pos = bytes.windows(5).position(|w| w == b"xref\n").unwrap();
        let xref_section = &bytes[xref_pos..];
        // First object is "0000000000 65535 f " (free)
        assert!(xref_section.windows(20).any(|w| w == b"0000000000 65535 f \n"));
    }

    #[test]
    fn write_pdf_to_disk_round_trips() {
        let pages = vec![make_test_page(20, 20)];
        let tmp = std::env::temp_dir().join("vigb_decoder_test.pdf");
        write_pdf(&pages, &tmp).expect("write PDF");
        let bytes = std::fs::read(&tmp).expect("read PDF back");
        assert!(bytes.starts_with(b"%PDF-1.4\n"));
        std::fs::remove_file(&tmp).ok();
    }
}
```

- [ ] **Step 3: Create `src/pdf.rs`**

```rust
//! Hand-written PDF writer. No PDF crate dependency.
//! Mirrors `max2pdf.py:write_pdf` (lines 992-1057).

use std::io::Write;
use std::path::Path;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::decoder::{Page, Preview};
use crate::error::Result;

/// Configuration for PDF generation.
#[derive(Debug, Clone)]
pub struct PdfOptions {
    /// Embed each page's preview thumbnail as a second page after the main page.
    /// When false, only main-image pages are written.
    pub include_previews: bool,
}

impl Default for PdfOptions {
    fn default() -> Self { Self { include_previews: true } }
}

/// Write `pages` to `path` as a single PDF. Convenience wrapper for
/// `write_pdf_bytes`.
pub fn write_pdf(pages: &[Page], path: &Path) -> Result<()> {
    let bytes = write_pdf_bytes(pages, &PdfOptions::default());
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Build a PDF as a Vec<u8>.
pub fn write_pdf_bytes(pages: &[Page], options: &PdfOptions) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = vec![Vec::new()];  // 1-based indexing
    let palette = [0xFFu8, 0x00];                       // /Indexed [0=white, 1=black]
    let mut page_ids: Vec<usize> = Vec::new();

    let mut emit = |obj: Vec<u8>, objs: &mut Vec<Vec<u8>>| -> usize {
        objs.push(obj);
        objs.len() - 1
    };

    for p in pages {
        page_ids.push(emit_page_for_bitmap(
            &p.bitmap,
            p.width,
            p.height,
            p.dpi_x,
            p.dpi_y,
            p.row_bytes,
            &palette,
            &mut objects,
            &mut emit,
        ));
        if options.include_previews {
            if let Some(prev) = &p.preview {
                page_ids.push(emit_page_for_preview(prev, p.dpi_x, p.dpi_y, &palette, &mut objects, &mut emit));
            }
        }
    }

    // /Pages object
    let mut pages_obj = Vec::new();
    write!(pages_obj, "<< /Type /Pages /Count {} /Kids [", page_ids.len()).unwrap();
    for (i, pid) in page_ids.iter().enumerate() {
        if i > 0 { pages_obj.push(b' '); }
        write!(pages_obj, "{pid} 0 R").unwrap();
    }
    pages_obj.extend_from_slice(b"] >>");
    let pages_id = emit(pages_obj, &mut objects);

    // Patch each page's /Parent reference
    for &pid in &page_ids {
        let placeholder = b"/Parent 0 0 R";
        let replacement = format!("/Parent {pages_id} 0 R");
        if let Some(start) = find_subslice(&objects[pid], placeholder) {
            objects[pid].splice(start..start + placeholder.len(), replacement.bytes());
        }
    }

    // /Catalog
    let catalog = format!("<< /Type /Catalog /Pages {pages_id} 0 R >>").into_bytes();
    let catalog_id = emit(catalog, &mut objects);

    // Assemble the PDF
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");
    let mut offsets: Vec<usize> = vec![0; objects.len()];
    for i in 1..objects.len() {
        offsets[i] = buf.len();
        write!(buf, "{i} 0 obj\n").unwrap();
        buf.extend_from_slice(&objects[i]);
        buf.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = buf.len();
    write!(buf, "xref\n0 {}\n0000000000 65535 f \n", objects.len()).unwrap();
    for &off in &offsets[1..] {
        write!(buf, "{off:010} 00000 n \n").unwrap();
    }
    write!(buf, "trailer\n<< /Size {} /Root {} 0 R >>\nstartxref\n{}\n%%EOF\n",
           objects.len(), catalog_id, xref_pos).unwrap();
    buf
}

fn emit_page_for_bitmap(
    raw: &[u8],
    width: u32,
    height: u32,
    dpi_x: u32,
    dpi_y: u32,
    row_bytes: u32,
    palette: &[u8; 2],
    objects: &mut Vec<Vec<u8>>,
    emit: &mut impl FnMut(Vec<u8>, &mut Vec<Vec<u8>>) -> usize,
) -> usize {
    let stored_width = row_bytes * 8;
    let compressed = zlib_compress(raw);

    let mut img_dict = Vec::new();
    let pal_hex: String = palette.iter().map(|b| format!("{b:02X}")).collect();
    write!(
        img_dict,
        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /BitsPerComponent 1 \
         /ColorSpace [/Indexed /DeviceGray 1 <{}>] /Filter /FlateDecode /Length {} >>\nstream\n",
        stored_width, height, pal_hex, compressed.len()
    ).unwrap();
    img_dict.extend_from_slice(&compressed);
    img_dict.extend_from_slice(b"\nendstream");
    let img_id = emit(img_dict, objects);

    let page_w = width as f64 * 72.0 / dpi_x as f64;
    let page_h = height as f64 * 72.0 / dpi_y as f64;
    let scale_x = stored_width as f64 * 72.0 / dpi_x as f64;
    let scale_y = page_h;
    let content_str = format!("q\n{scale_x:.4} 0 0 {scale_y:.4} 0 0 cm\n/Im0 Do\nQ\n");
    let mut content = Vec::new();
    write!(content, "<< /Length {} >>\nstream\n", content_str.len()).unwrap();
    content.extend_from_slice(content_str.as_bytes());
    content.extend_from_slice(b"endstream");
    let content_id = emit(content, objects);

    let page_obj = format!(
        "<< /Type /Page /Parent 0 0 R /MediaBox [0 0 {page_w:.4} {page_h:.4}] \
         /Contents {content_id} 0 R \
         /Resources << /XObject << /Im0 {img_id} 0 R >> /ProcSet [/PDF /ImageB] >> >>"
    ).into_bytes();
    emit(page_obj, objects)
}

fn emit_page_for_preview(
    prev: &Preview,
    dpi_x: u32,
    dpi_y: u32,
    palette: &[u8; 2],
    objects: &mut Vec<Vec<u8>>,
    emit: &mut impl FnMut(Vec<u8>, &mut Vec<Vec<u8>>) -> usize,
) -> usize {
    emit_page_for_bitmap(
        &prev.bitmap, prev.width, prev.height,
        dpi_x, dpi_y, prev.row_bytes, palette, objects, emit,
    )
}

fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
    e.write_all(data).unwrap();
    e.finish().unwrap()
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

// (insert tests here per Step 2)
```

- [ ] **Step 4: Wire into `src/lib.rs`**

Append:

```rust
mod pdf;
pub use pdf::{write_pdf, write_pdf_bytes, PdfOptions};
```

- [ ] **Step 5: Run tests**

Run: `cargo test --lib pdf && cargo test`
Expected: 3 new pdf tests pass + all prior tests still pass.

Sanity check: open one of the produced test PDFs in any PDF viewer (`Start-Process (Test-Path $env:TEMP\vigb_decoder_test.pdf)`) and confirm a blank white page renders.

- [ ] **Step 6: Commit**

```powershell
git add src/pdf.rs src/lib.rs Cargo.toml
git commit -m @'
feat: add hand-written PDF writer

Mirrors max2pdf.py:write_pdf. One PDF page per main-image page;
preview thumbnails (when present) emitted as additional pages.

Adds flate2 v1 as the only runtime compression dep — required for
the FlateDecode image XObjects.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 13: Public API consolidation

**Files:**
- Modify: `src/lib.rs`

The lib has accumulated re-exports task by task. This task consolidates them, adds the crate-level docs, and verifies the public surface matches the spec.

- [ ] **Step 1: Replace `src/lib.rs` with the consolidated version**

```rust
//! Decoder for PaperPort 2 (`.max`) image scans.
//!
//! The PaperPort 2 file format ("ViGBe") is a proprietary container used
//! by ScanSoft's PaperPort 2 (1996) for 1-bit scanned documents. Each
//! image chunk wraps a CCITT-T.6 (Group 4 fax) compressed bitmap with a
//! custom per-line marker dispatcher.
//!
//! # Quick start
//!
//! ```no_run
//! use vigb_decoder::{decode_max_file, write_pdf, Config};
//! let pages = decode_max_file("scan.max", &Config::default())?;
//! write_pdf(&pages, std::path::Path::new("scan.pdf"))?;
//! # Ok::<(), vigb_decoder::MaxError>(())
//! ```
//!
//! # Format documentation
//!
//! See `docs/format.md` and `docs/decoder.md` in the repo for the format
//! specification and the canonical decoder behaviour.
//!
//! # Output bitmap polarity
//!
//! [`Page::bitmap`] is 1-bit packed, MSB-first per byte. **Bit value 1
//! means BLACK.** This matches the PDF `/Indexed [/DeviceGray 1 <FF 00>]`
//! convention used by [`write_pdf`]. If you're comparing against a PNG
//! ground-truth in PIL `'1'` mode, be aware that PIL `'1'` uses the
//! opposite convention (bit 1 = white) — invert before comparing.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod bitstream;
mod ccitt;
mod chunks;
mod config;
mod decoder;
mod dispatch;
mod error;
mod pdf;
mod preview;

pub use config::{Config, ConfigBuilder, DispatchKind, T0DropMode};
pub use decoder::{DecodeStats, Page, Preview};
pub use error::{MaxError, Result};
pub use pdf::{write_pdf, write_pdf_bytes, PdfOptions};

/// Decode all image chunks in a `.max` byte buffer.
///
/// Returns one [`Page`] per image chunk in document order.
///
/// # Errors
///
/// Returns [`MaxError::BadMagic`] if the input does not begin with the
/// `ViGBe` magic. Returns [`MaxError::Truncated`] if no valid image
/// chunks are found in the file.
///
/// Per-line decode failures are recorded in [`DecodeStats`], not
/// surfaced as errors — the decoder always produces a [`Page`] when the
/// file structure is valid.
pub fn decode_max(data: &[u8], cfg: &Config) -> Result<Vec<Page>> {
    if data.len() < 5 || &data[..5] != b"ViGBe" {
        return Err(MaxError::BadMagic { offset: 0 });
    }
    let chunks = chunks::find_image_chunks(data);
    if chunks.is_empty() {
        return Err(MaxError::Truncated { offset: 0, need: 0x40, have: data.len() });
    }
    let mut out = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        out.push(dispatch::decode_image_chunk(data, chunk.offset, chunk.length, cfg));
    }
    Ok(out)
}

/// Decode a `.max` file from disk. Convenience wrapper for [`decode_max`].
pub fn decode_max_file<P: AsRef<std::path::Path>>(path: P, cfg: &Config) -> Result<Vec<Page>> {
    let data = std::fs::read(path)?;
    decode_max(&data, cfg)
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test && cargo doc --no-deps`
Expected: All tests pass; `cargo doc` succeeds without warnings (the `missing_docs` lint catches incomplete API docs).

If `missing_docs` complains, add doc comments to the offending items in `src/decoder.rs` and elsewhere.

- [ ] **Step 3: Commit**

```powershell
git add src/lib.rs src/decoder.rs
git commit -m @'
feat: consolidate public API + crate-level docs

Documents the bit=1-means-BLACK polarity invariant on Page::bitmap
(was the source of the 6th-session GT-comparison bug in Python).
cargo doc passes with missing_docs lint enabled.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 14: CLI binary (`max2pdf`)

**Files:**
- Modify: `src/bin/max2pdf.rs` (replace placeholder)

**Reference:** `max2pdf.py:1064-1156` (`main` + argparse).

`clap` derive parser. Every Python flag preserved with the same long name (muscle memory transfers). Defaults match Python defaults.

- [ ] **Step 1: Replace `src/bin/max2pdf.rs` with the full CLI**

```rust
//! `max2pdf` — convert PaperPort 2 (.max) files to PDF.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{ArgAction, Parser, ValueEnum};

use vigb_decoder::{decode_max_file, write_pdf, Config, T0DropMode};

#[derive(Debug, Parser)]
#[command(name = "max2pdf", version, about = "Convert PaperPort 2 (.max) files to PDF")]
struct Cli {
    /// One or more .max files
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    /// Write PDFs into this directory (default: alongside each input)
    #[arg(short = 'o', long = "output-dir")]
    output_dir: Option<PathBuf>,
    /// Print per-file decode statistics
    #[arg(long)]
    stats: bool,

    /// Skip embedding the preview thumbnail page
    #[arg(long = "no-preview", action = ArgAction::SetTrue)]
    no_preview: bool,
    /// Disable the canonical reference-table walk fix (diagnostic)
    #[arg(long = "no-bug4", action = ArgAction::SetTrue)]
    no_bug4: bool,
    /// Disable the strict type-0 marker gate (diagnostic)
    #[arg(long = "no-strict-t0", action = ArgAction::SetTrue)]
    no_strict_t0: bool,
    /// Use byte-by-byte bit refill (diagnostic)
    #[arg(long)]
    lazy_bit_loading: bool,
    /// Reset reference table at chunk start (diagnostic vestige)
    #[arg(long)]
    t0_reset: bool,
    /// Type-0 drop-after-drift mode
    #[arg(long, value_enum, default_value_t = T0DropArg::None)]
    t0_drop_after_drift: T0DropArg,
    /// Restrict t0-drop to comma-separated dispatch kinds (e.g. "fail,v0")
    #[arg(long)]
    t0_drop_kinds: Option<String>,
    /// Bytes to scan-forward after a FAIL looking for next valid marker
    #[arg(long, default_value_t = 0)]
    fail_scan_forward: u32,
    /// In FAIL cascades, do not advance y on each FAIL
    #[arg(long)]
    suppress_t2_fail_y_in_cascade: bool,

    /// Smart-resync probe range ±K after isolated FAIL (0 disables)
    #[arg(long, default_value_t = 0)]
    fail_resync_max: u32,
    /// Smart-resync probe lookahead in lines
    #[arg(long, default_value_t = 5)]
    fail_resync_lookahead: u32,
    /// Smart-resync minimum confidence margin (n_ok - n_drift)
    #[arg(long, default_value_t = 0)]
    fail_resync_min_confidence: u32,
    /// Maximum total resync probes per page (0 = unlimited)
    #[arg(long, default_value_t = 0)]
    fail_resync_budget: u32,
    /// Reset reference table after a drift event
    #[arg(long)]
    reset_ref_after_drift: bool,

    /// Keep type-3 BLANK markers that follow drift (diagnostic — disables the 6th-session fix)
    #[arg(long, action = ArgAction::SetTrue)]
    keep_drift_blanks: bool,
    /// Keep type-1 dispatches (diagnostic — disables the 6th-session fix)
    #[arg(long, action = ArgAction::SetTrue)]
    keep_t1_dispatches: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum T0DropArg { None, Marker, Full }

impl From<T0DropArg> for T0DropMode {
    fn from(a: T0DropArg) -> Self {
        match a {
            T0DropArg::None => T0DropMode::None,
            T0DropArg::Marker => T0DropMode::Marker,
            T0DropArg::Full => T0DropMode::Full,
        }
    }
}

fn build_config(cli: &Cli) -> Config {
    Config::builder()
        .embed_preview(!cli.no_preview)
        .bug4(!cli.no_bug4)
        .strict_t0(!cli.no_strict_t0)
        .lazy_bit_loading(cli.lazy_bit_loading)
        .t0_reset(cli.t0_reset)
        .t0_drop_after_drift(cli.t0_drop_after_drift.into())
        .fail_scan_forward(cli.fail_scan_forward)
        .suppress_t2_fail_y_in_cascade(cli.suppress_t2_fail_y_in_cascade)
        .fail_resync_max(cli.fail_resync_max)
        .fail_resync_lookahead(cli.fail_resync_lookahead)
        .fail_resync_min_confidence(cli.fail_resync_min_confidence)
        .fail_resync_budget(cli.fail_resync_budget)
        .reset_ref_after_drift(cli.reset_ref_after_drift)
        .drop_blank_after_drift(!cli.keep_drift_blanks)
        .suppress_t1_all(!cli.keep_t1_dispatches)
        .build()
}

fn process_one(input: &std::path::Path, out_dir: Option<&std::path::Path>, cfg: &Config, want_stats: bool) -> Result<()> {
    let pages = decode_max_file(input, cfg)
        .with_context(|| format!("decode {}", input.display()))?;

    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let parent = out_dir.map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| input.parent().map(std::path::Path::to_path_buf).unwrap_or_else(|| PathBuf::from(".")));
    std::fs::create_dir_all(&parent)
        .with_context(|| format!("create output dir {}", parent.display()))?;
    let out_path = parent.join(format!("{stem}.pdf"));

    write_pdf(&pages, &out_path)
        .with_context(|| format!("write {}", out_path.display()))?;
    println!("{} -> {}", input.display(), out_path.display());

    if want_stats {
        for (i, p) in pages.iter().enumerate() {
            let s = &p.stats;
            println!("  page {i}: {}x{} ok={} v0={} t0={} t1={} fail={} max_consec_fail={} first_fail_y={:?} resync_probes={} resync_hits={} blank_drops_drift={}",
                p.width, p.height, s.n_ok, s.n_v0, s.n_t0, s.n_t1, s.n_fail,
                s.max_consecutive_fail, s.first_fail_y, s.resync_probes, s.resync_hits,
                s.blank_drops_after_drift);
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let cfg = build_config(&cli);
    let out_dir = cli.output_dir.as_deref();
    let mut had_error = false;
    for input in &cli.inputs {
        if let Err(e) = process_one(input, out_dir, &cfg, cli.stats) {
            eprintln!("error: {e:#}");
            had_error = true;
        }
    }
    if had_error { ExitCode::from(1) } else { ExitCode::SUCCESS }
}
```

- [ ] **Step 2: Verify `--help` output renders sensibly**

Run: `cargo run --bin max2pdf -- --help`
Expected: full flag list with descriptions; no panics.

- [ ] **Step 3: End-to-end smoke test**

Run:
```powershell
cargo run --release --bin max2pdf -- tests/fixtures/synthetic.max -o $env:TEMP --stats
```

Expected:
- Prints `tests/fixtures/synthetic.max -> $env:TEMP\synthetic.pdf`
- Stats line shows `n_fail=0`
- A valid PDF appears at the output path

- [ ] **Step 4: Commit**

```powershell
git add src/bin/max2pdf.rs
git commit -m @'
feat: add max2pdf CLI binary

Full flag parity with max2pdf.py:1064-1156 via clap derive. Defaults
match Python defaults (canonical fixes ON, experimental flags OFF).
Per-file errors do not stop a batch run; exit 1 if any file failed.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 15: Benches (criterion)

**Files:**
- Create: `benches/decoder.rs`

- [ ] **Step 1: Create `benches/decoder.rs`**

```rust
use std::fs;
use std::path::Path;

use criterion::{criterion_group, criterion_main, Criterion};
use vigb_decoder::{decode_max, Config};

fn bench_decode_synthetic(c: &mut Criterion) {
    let data = fs::read(Path::new("tests/fixtures/synthetic.max"))
        .expect("synthetic.max — run cargo run --bin encode-fixture if missing");
    let cfg = Config::default();
    c.bench_function("decode_max synthetic", |b| {
        b.iter(|| {
            let pages = decode_max(&data, &cfg).unwrap();
            criterion::black_box(pages);
        });
    });
}

criterion_group!(benches, bench_decode_synthetic);
criterion_main!(benches);
```

- [ ] **Step 2: Run benches**

Run: `cargo bench`
Expected: Criterion reports a baseline time. Note the nanoseconds-per-iteration figure for future regression comparison.

If `cargo bench` errors with "the harness for this bench is not configured", verify `Cargo.toml` has:

```toml
[[bench]]
name = "decoder"
harness = false
```

- [ ] **Step 3: Commit**

```powershell
git add benches/decoder.rs
git commit -m @'
feat: add criterion bench on synthetic fixture

Single bench (decode_max on synthetic.max) — establishes the
baseline. Per-function benches (decomp_line, decode_image_chunk)
can be added later when optimising specific hot paths.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 16: Docs port

**Files:**
- Create: `docs/format.md`, `docs/decoder.md`, `docs/cli.md`, `docs/credits.md`, `docs/provenance.md`, `docs/release-checklist.md`

Each doc is ported from the Python repo's wiki where available; net-new where not.

- [ ] **Step 1: `docs/format.md`** — port from `C:\Users\chris\Desktop\Alte Scans\wiki\topics\format.md`

Read the source file and copy the relevant content. Replace any references to Python-specific tooling (e.g., `python max2pdf.py`) with Rust equivalents (`max2pdf` binary). Cross-references to other wiki pages (e.g., `wiki/topics/decoder-heuristics.md`) become `docs/decoder.md`.

- [ ] **Step 2: `docs/decoder.md`** — synthesize from Python wiki

Combine content from:
- `C:\Users\chris\Desktop\Alte Scans\wiki\topics\decoder-heuristics.md`
- `C:\Users\chris\Desktop\Alte Scans\wiki\topics\bridge-and-bug-fixes.md` (especially §"Bug 4 — canonical si advance" and §"Bug 5")

Document each canonical fix the Rust port inherits: bug4 (reference-table walk), strict_t0, drop_blank_after_drift, suppress_t1_all. Note how to flip each one off via CLI (`--no-bug4`, etc.) and what the documented effect is.

- [ ] **Step 3: `docs/cli.md`** — flag mapping table

```markdown
# CLI flag reference

This table maps every `max2pdf` Rust CLI flag to its Python source-of-truth
in `max2pdf.py`. The flag long names match exactly so muscle memory transfers.

| Rust flag | Python flag | Default | Source line |
|---|---|---|---|
| `-o`/`--output-dir` | `-o`/`--output-dir` | (none) | max2pdf.py:1067 |
| `--no-preview` | `--no-preview` | off | max2pdf.py:1126 |
| `--no-bug4` | `--no-bug4` | off | max2pdf.py:1150 |
| `--no-strict-t0` | `--no-strict-t0` | off | max2pdf.py:1137 |
| `--lazy-bit-loading` | `--lazy-bit-loading` | off | max2pdf.py:1144 |
| `--t0-reset` | `--t0-reset` | off | max2pdf.py:1069 |
| `--t0-drop-after-drift` | `--t0-drop-after-drift` | none | max2pdf.py:1073 |
| `--t0-drop-kinds` | `--t0-drop-kinds` | (none) | max2pdf.py:1078 |
| `--fail-scan-forward` | `--fail-scan-forward` | 0 | max2pdf.py:1083 |
| `--suppress-t2-fail-y-in-cascade` | `--suppress-t2-fail-y-in-cascade` | off | max2pdf.py:1088 |
| `--fail-resync-max` | `--fail-resync-max` | 0 | max2pdf.py:1092 |
| `--fail-resync-lookahead` | `--fail-resync-lookahead` | 5 | max2pdf.py:1106 |
| `--fail-resync-min-confidence` | `--fail-resync-min-confidence` | 0 | max2pdf.py:1109 |
| `--fail-resync-budget` | `--fail-resync-budget` | 0 | max2pdf.py:1117 |
| `--reset-ref-after-drift` | `--reset-ref-after-drift` | off | max2pdf.py:1122 |
| `--keep-drift-blanks` | `--keep-drift-blanks` | off | max2pdf.py:1129 |
| `--keep-t1-dispatches` | `--keep-t1-dispatches` | off | max2pdf.py:1133 |
| `--stats` | _(not in Python)_ | off | net-new |

`--stats` is the one CLI-only addition — Python has no equivalent flag;
its decoder always returns DecodeStats internally but the CLI doesn't
print them.

## Examples

Convert one file with default settings:
    max2pdf scan.max

Convert several files into a directory, with stats:
    max2pdf -o out/ --stats *.max

Diagnose a problematic file (turn off canonical fixes one at a time):
    max2pdf bad.max --no-bug4
    max2pdf bad.max --no-strict-t0

Try smart resync on a file with FAIL events:
    max2pdf bad.max --fail-resync-max 4 --reset-ref-after-drift --fail-resync-min-confidence 2
```

- [ ] **Step 4: `docs/credits.md`**

```markdown
# Credits

This decoder builds on prior reverse-engineering and standardization work.

## Reverse-engineering bridge

PaperPort 3.6 (ScanSoft, 1996) — the canonical implementation matched
here is `MAXKER2.DLL` from PaperPort 3.6, extracted from the Visioneer
Deluxe 5.2 installer ISO (publicly hosted on archive.org since
[2020-03](https://archive.org/details/PaperPort_Deluxe_Visioneer_Version_5.2_1997)).

[otvdm/winevdm](https://github.com/otya128/winevdm) (otya128) — runs
the 16-bit/Win9x PaperPort 3.6 binary on modern Windows, used as the
test oracle during decoder development.

## Standards

ITU-T Recommendation T.6 (07/88) — Facsimile coding schemes (Group 4).
Source for all CCITT-T.6 lookup tables in `src/ccitt.rs`. Free PDF at
https://www.itu.int/rec/T-REC-T.6.

## Prior-art OSS projects

These projects implement partial PaperPort decoders. They are
**GPL-2-or-later** and no code from either project is copied into this
crate (see `docs/provenance.md`); they were used only as cross-checks
during reverse engineering.

- [paperman](https://github.com/sjg20/paperman) — Java PaperPort browser
  by Simon Glass. Active. Does not support PaperPort 2 era files.
- [max2pdf](https://github.com/orangeturtle739/max2pdf) — Python
  PaperPort-to-PDF by orangeturtle739. Dormant. Does not support
  PaperPort 2 era files.

## Author

Christian Regg, 2026. Reverse engineering done over 12 sessions across
sessions logged in the research repo (the parallel Python implementation
that informs this Rust port).
```

- [ ] **Step 5: `docs/provenance.md`**

```markdown
# Provenance and clean-room separation

This document records where every component of the decoder came from, to
support the MIT/Apache-2.0 license posture against any GPL-contamination
claim from prior-art OSS projects.

## Component sources

| Component | Source | Provenance |
|---|---|---|
| CCITT-T.6 lookup tables (`src/ccitt.rs`) | ITU-T Recommendation T.6 (07/88), Tables 1, 2, 3 | Numerical values transcribed from the public ITU standard. Cross-checked against `__pp_src__/paperman_btab.dat` in the research repo for sanity, but the canonical values come from the ITU PDF. Same numbers, different source. |
| Per-line decoder (`src/decoder.rs`) | Disassembly of `MAXKER2.DLL` (PaperPort 3.6, 1996) + bit-traces of self-owned `.max` files | Reverse-engineered for interoperability. The Python `max2pdf.py` in the research repo is the parallel Rust implementation's reference; algorithmic logic ported here from author's own Python source, not from paperman or max2pdf. |
| Per-line dispatcher (`src/dispatch.rs`) | Same as decoder | Same as decoder. |
| Chunk discovery (`src/chunks.rs`) | Bit-trace of `.max` file structure | Author's own RE; trivial DL-magic scan, also documented in JustSolve wiki at http://fileformats.archiveteam.org/wiki/PaperPort_(MAX). |
| Preview RLE (`src/preview.rs`) | Bit-trace + reading paperman as cross-check | Author's own RE. paperman has the only other documented preview decoder; no code copied. |
| PDF writer (`src/pdf.rs`) | PDF 1.4 specification | Hand-written; PDF format itself is an Adobe specification, not encumbered. |
| Test-only encoder (`tests/common/encoder.rs`) | ITU-T T.6 Tables + RE notes | Author's own implementation of standard CCITT-T.6 encoding. |

## Reverse-engineering legal basis

Reverse engineering for interoperability is permitted under:
- Switzerland: URG Art. 21 (decompilation for interface info).
- EU: Software Directive 2009/24/EC Art. 6 (RE for interoperability).
- US: DMCA §1201(f) safe harbour and *Sega v. Accolade*, 977 F.2d 1510
  (9th Cir. 1992) (disassembly for interop is fair use).

The decoder ships zero bytes from PaperPort. The MAXKER2.DLL extraction
used [`idecomp`](https://github.com/<idecomp_url>) on InstallShield V3
`.Z` archives, which is a publicly-known format operation.

## What we deliberately did NOT do

- Did not copy any source from `paperman` (GPL-2-or-later).
- Did not copy any source from `max2pdf` Python (GPL-2-or-later).
- Did not embed `MAXKER2.DLL`, any other PaperPort binary, or any
  bytes from the Visioneer 5.2 ISO in this repository.
- Did not copy CCITT table values from `paperman_btab.dat`. The numbers
  in `src/ccitt.rs` came from ITU-T T.6.

If you find any code in this repository that appears to be a 1:1 port
of paperman or max2pdf logic, please open an issue — that's a
provenance bug we want to fix.
```

- [ ] **Step 6: `docs/release-checklist.md`**

```markdown
# v0.1 release checklist

Marketing channels documented for first release (per the 2026-05-10
demand research). Modest audience — niche-archival, ~50–500 useful
users per year — so picking the right channels matters.

## Pre-release

- [ ] `cargo test` green on Linux + Windows + macOS (CI).
- [ ] `cargo doc --no-deps` clean (no `missing_docs` warnings).
- [ ] `cargo clippy -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.
- [ ] CHANGELOG.md has a v0.1.0 entry.
- [ ] README.md renders correctly on GitHub.
- [ ] `git grep -i maxker` returns zero hits (no PaperPort binaries committed).
- [ ] `tests/fixtures/synthetic.max` is a non-personal synthetic file.

## Publish

- [ ] `git tag v0.1.0`
- [ ] `git push origin v0.1.0`
- [ ] `cargo publish` (dry-run first: `cargo publish --dry-run`)
- [ ] GitHub release created from the tag with auto-built binaries
      (Linux x86_64, Windows x86_64, macOS aarch64).

## Marketing channels (in priority order)

1. **JustSolve / fileformats.archiveteam.org wiki** — add a "Software"
   entry to http://fileformats.archiveteam.org/wiki/PaperPort_(MAX).
   Highest-leverage move: the page already lists `paperman` and
   `max2pdf` with the PP2-not-supported caveat. Adding `vigb-decoder`
   as the first PP2-capable tool makes it discoverable to anyone
   googling the format.

2. **GitHub issue on `sjg20/paperman`** — Simon Glass is active.
   Title: "PaperPort 2 era support via vigb-decoder". Body: brief
   pointer to this repo + offer to coordinate on shared format docs.

3. **Forum thread replies** (one-line "FYI, here's a Rust tool that
   handles PP2"):
   - https://www.bleepingcomputer.com/forums/t/688796/how-to-open-max-file/
   - https://learn.microsoft.com/en-us/answers/questions/2493531/help-for-files-with-max-extension-unable-to-open-t
   - https://www.windowsbbs.com/threads/how-to-convert-legacy-files-paperport-max.108861/
   - https://forums.linuxmint.com/viewtopic.php?t=194479
   - https://www.techguy.org/threads/retrieving-paperport-files-using-the-max-extension.1268281/
   - https://newsgroup.xnview.com/viewtopic.php?t=43432
   These threads still rank on Google for "open .max file".

4. **Reddit posts** — single post each to r/DataHoarder,
   r/datarecovery, r/genealogy. Title format:
   "I built a Rust tool for the dead PaperPort 2 (.max) format —
   first decoder that handles 1986–87 era files".

5. **Open Preservation Foundation** — submit to their
   disappearing-file-formats blog series via blog@openpreservation.org.

6. **Optional: contact DiskTransfer.co.uk** — they openly market paid
   PP1/2/3 recovery. Offering to license/cite the decoder turns a
   competitor into a referrer.
```

- [ ] **Step 7: Run lint + commit**

Run: `cargo doc --no-deps`
Expected: succeeds; no warnings.

```powershell
git add docs/
git commit -m @'
docs: add format / decoder / cli / credits / provenance / release-checklist

Ports the relevant content from the Python research repo's wiki and
adds the new release-checklist with concrete marketing channels from
the 2026-05-10 demand research.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 17: README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write `README.md`**

```markdown
# vigb-decoder

[![ci](https://github.com/creggch/vigb-decoder/actions/workflows/ci.yml/badge.svg)](https://github.com/creggch/vigb-decoder/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/vigb-decoder.svg)](https://crates.io/crates/vigb-decoder)
[![docs.rs](https://docs.rs/vigb-decoder/badge.svg)](https://docs.rs/vigb-decoder)

Decoder for PaperPort 2 (`.max`) image scans from 1986–87.

## Why this exists

The PaperPort 2 file format ("ViGBe") is dead. The only way to open
these files used to be PaperPort 3.6 — a 1996 Windows app that doesn't
run natively on modern Windows and isn't supported by any current
PaperPort version (Tungsten / Kofax / Nuance — see
[Nuance KB 1473](https://nuance.custhelp.com/app/answers/detail/a_id/1473/)).

`vigb-decoder` is the first known tool that decodes PaperPort 2 era
files. The closest existing OSS projects ([paperman](https://github.com/sjg20/paperman)
and [max2pdf](https://github.com/orangeturtle739/max2pdf)) explicitly do
not support this format.

## Install

    cargo install vigb-decoder

This installs the `max2pdf` binary in `~/.cargo/bin/`.

Pre-built binaries for Linux x86_64, Windows x86_64, and macOS aarch64
are attached to each [release](https://github.com/creggch/vigb-decoder/releases).

## Use

Convert a single file:

    max2pdf scan.max

Convert a batch into a directory:

    max2pdf -o out/ *.max

Print per-file decode stats:

    max2pdf --stats scan.max

See `docs/cli.md` for the full flag list.

## Library use

    use vigb_decoder::{decode_max_file, write_pdf, Config};
    use std::path::Path;

    let pages = decode_max_file("scan.max", &Config::default())?;
    write_pdf(&pages, Path::new("scan.pdf"))?;

`Page::bitmap` is 1-bit packed, MSB-first per byte. **Bit value 1 means
BLACK** (matches the PDF `/Indexed [/DeviceGray 1 <FF 00>]` convention).

## Status

Bit-perfect against the canonical PaperPort 3.6 reference on every file
we have ground truth for. Median IoU = 1.000 across a 159-page test
corpus (private — the test corpus is the author's personal document
archive).

## Format reverse-engineering

See [`docs/format.md`](docs/format.md) for the file structure and
[`docs/decoder.md`](docs/decoder.md) for the canonical decoder behaviour
(including the four canonical fixes the decoder implements).

## Reverse-engineering legal basis

This decoder was reverse-engineered for interoperability under:
- Switzerland: [URG Art. 21](https://www.fedlex.admin.ch/eli/cc/1993/1798_1798_1798/en) (decompilation for interface info).
- EU: [Software Directive 2009/24/EC Art. 6](https://eur-lex.europa.eu/legal-content/EN/TXT/PDF/?uri=CELEX:32009L0024).
- US: [DMCA §1201(f)](https://www.law.cornell.edu/uscode/text/17/1201) safe harbour and [_Sega v. Accolade_, 977 F.2d 1510 (9th Cir. 1992)](https://www.copyright.gov/fair-use/summaries/segaenters-accolade-9thcir1992.pdf).

The decoder ships zero bytes from PaperPort. CCITT-T.6 lookup tables
are derived from the [ITU-T T.6 Recommendation](https://www.itu.int/rec/T-REC-T.6)
(a public standard); format dispatch logic was developed against
bit-traces of the author's own `.max` files cross-checked against the
disassembly of ScanSoft's `MAXKER2.DLL` (extracted from the publicly
distributed Visioneer 5.2 installer ISO, archive.org, 2020).

See [`docs/provenance.md`](docs/provenance.md) for component-level
clean-room separation notes.

## Credits

- PaperPort 3.6 (ScanSoft, 1996) — bridge that made the RE possible.
- ITU-T T.6 Recommendation — source for CCITT Group 4 table values.
- [paperman](https://github.com/sjg20/paperman) (Simon Glass) and
  [max2pdf](https://github.com/orangeturtle739/max2pdf) (orangeturtle739)
  — prior-art OSS projects, used as cross-checks during RE. Both
  GPL-2-or-later; no code is copied from either project.
- [otvdm](https://github.com/otya128/winevdm) (otya128) — runs the
  PP3.6 bridge under modern Windows.

## License

Licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.
```

- [ ] **Step 2: Verify it renders**

Push to a private GitHub repo or use a Markdown previewer
(`code --markdown.previewfront-matter=disabled README.md` if VS Code
is available) and confirm:
- All links resolve (badge URLs may 404 until the repo is public — OK).
- Code blocks render.
- No raw `<owner>` placeholders left.

- [ ] **Step 3: Commit**

```powershell
git add README.md
git commit -m @'
docs: add README

First-known-decoder pitch, install/use/library examples, RE legal
basis section, full credits.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
```

---

## Task 18: Release infrastructure + first publish

**Files:**
- Create: `CHANGELOG.md`, `.github/workflows/release.yml`

- [ ] **Step 1: Create `CHANGELOG.md`**

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-05-XX

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
- CCITT-T.6 lookup tables derived from ITU-T T.6 Recommendation
  (clean-room: not copied from paperman or max2pdf).

[Unreleased]: https://github.com/creggch/vigb-decoder/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/creggch/vigb-decoder/releases/tag/v0.1.0
```

- [ ] **Step 2: Create `.github/workflows/release.yml`**

```yaml
name: release

on:
  push:
    tags: ['v*']

permissions:
  contents: write

jobs:
  build:
    name: build (${{ matrix.target }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - { os: ubuntu-latest,  target: x86_64-unknown-linux-gnu, ext: '' }
          - { os: windows-latest, target: x86_64-pc-windows-msvc,   ext: '.exe' }
          - { os: macos-latest,   target: aarch64-apple-darwin,     ext: '' }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.75
        with: { targets: ${{ matrix.target }} }
      - uses: Swatinem/rust-cache@v2
      - name: build
        run: cargo build --release --target ${{ matrix.target }} --bin max2pdf
      - name: rename binary
        shell: bash
        run: |
          src=target/${{ matrix.target }}/release/max2pdf${{ matrix.ext }}
          dst=max2pdf-${{ github.ref_name }}-${{ matrix.target }}${{ matrix.ext }}
          cp "$src" "$dst"
          echo "ASSET=$dst" >> $GITHUB_ENV
      - uses: softprops/action-gh-release@v2
        with:
          files: ${{ env.ASSET }}
          generate_release_notes: true

  publish:
    name: cargo publish
    runs-on: ubuntu-latest
    needs: build
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.75
      - name: publish
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
        run: cargo publish
```

- [ ] **Step 3: Pre-publish dry runs**

Before tagging, verify everything is publishable:

Run:
```
cargo publish --dry-run
```

Expected: succeeds and reports the expected file list. Common failures:
- Missing `description` / `license` in Cargo.toml → fixed in Task 1.
- Missing `README.md` → fixed in Task 17.
- File too large → check `.gitignore` excludes `target/` and any test corpus.

- [ ] **Step 4: Set the date in CHANGELOG.md**

Replace `2026-05-XX` with the actual release date.

- [ ] **Step 5: Add `CARGO_REGISTRY_TOKEN` to GitHub Secrets**

Manual step (not scriptable here): on https://github.com/creggch/vigb-decoder/settings/secrets/actions, add a secret named `CARGO_REGISTRY_TOKEN` with a token from https://crates.io/me.

- [ ] **Step 6: Run release checklist (`docs/release-checklist.md` Pre-release section)**

Tick each item — `cargo test`, `cargo doc`, `cargo clippy`, `cargo fmt`, no MAXKER bytes committed, etc.

- [ ] **Step 7: Tag and push**

```powershell
git add CHANGELOG.md .github/workflows/release.yml
git commit -m @'
chore: add release infrastructure (CHANGELOG + release workflow)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
'@
git tag -a v0.1.0 -m "v0.1.0 — first public release"
git push origin master
git push origin v0.1.0
```

The `release.yml` workflow runs on the tag push, builds platform binaries, attaches them to a GitHub Release, and runs `cargo publish`.

- [ ] **Step 8: Verify the release landed**

Wait ~5 minutes for the workflow to complete, then check:
- https://github.com/creggch/vigb-decoder/releases/tag/v0.1.0 has 3 binaries attached.
- https://crates.io/crates/vigb-decoder shows version 0.1.0.
- `cargo install vigb-decoder` from a fresh shell installs successfully.
- `max2pdf --help` works on the installed binary.

- [ ] **Step 9: Execute the marketing checklist**

Work through `docs/release-checklist.md` § Marketing channels in order:
1. Edit JustSolve wiki page.
2. Open the paperman issue.
3. Reply to the 6 forum threads.
4. Post to r/DataHoarder, r/datarecovery, r/genealogy.
5. Email Open Preservation Foundation.

This is the audience-acquisition step. The decoder is technically complete; this is what makes it discoverable.

---

## Self-review checklist (run after the plan is fully executed)

- [ ] `cargo test` passes on Linux, Windows, macOS.
- [ ] `cargo doc --no-deps` produces clean docs.
- [ ] `cargo clippy --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.
- [ ] `cargo bench` reports a baseline time on synthetic.
- [ ] Synthetic round-trip: `n_fail == 0` under canonical defaults.
- [ ] Default `Config` values match `max2pdf.py` defaults exactly.
- [ ] Every Python CLI flag has a Rust equivalent with the same long name.
- [ ] `Page::bitmap` polarity documented (bit=1 means BLACK).
- [ ] `docs/provenance.md` cites ITU-T T.6 as the table source.
- [ ] No paperman code copied; `git grep -i paperman` returns only docs and credits.
- [ ] No `MAXKER2.DLL` bytes in the repo; `git grep -i maxker` returns only docs.
- [ ] Repo URL in Cargo.toml matches the actual GitHub URL.
- [ ] License files (MIT and Apache-2.0) present at repo root.
- [ ] `vigb-decoder` reserved on crates.io.

---

## What's intentionally out of scope for v0.1

- `serde` impls on public types (add later as a feature flag if asked).
- Async / streaming decode (Python is fully synchronous — matches use case).
- WASM target (could be added behind a feature flag if there's demand).
- Encoder for `.max` (only the test-only minimal encoder is needed for fixtures; full-spec encoder would be a separate crate).
- Color image support (PaperPort 2 was 1-bit only).
- GUI / drag-and-drop wrapper.
- Local-only corpus tests behind `cargo test --features corpus` — placeholder feature is wired in Task 1, but the actual `tests/corpus.rs` integration is left as a follow-up because it requires the user's private archive at a configured path (`VIGB_DECODER_CORPUS` env var).





