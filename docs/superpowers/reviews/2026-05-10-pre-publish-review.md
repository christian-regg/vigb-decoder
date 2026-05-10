# Pre-Publish Review — `vigb-decoder` 0.1.0

**Date:** 2026-05-10
**Reviews run:** Security (untrusted-input threat model) · Rust code quality · Legal / IP
**Trigger:** pre-`cargo publish` go/no-go.
**Outcome:** Conditional GO. Critical and High items below should land before publish; everything else can ship in 0.1.1.

## Status legend

| Symbol | Meaning |
|--------|---------|
| ⬜ | Open |
| 🔄 | In progress |
| ✅ | Done |
| ⏭️ | Deferred / won't fix |
| 💬 | Needs discussion |

## Recommended order

| # | Action | Closes |
|---|--------|--------|
| 1 | Extract `ChunkHeader::parse` with bounds checks; convert `decode_image_chunk` → `Result` | CRIT-01, SEC-H01 |
| 2 | Cap `width × height` and `padded_x × preview_y`; use `checked_mul` everywhere | CRIT-02, SEC-H02 |
| 3 | Forward-progress guard in dispatcher (type-2 with 0 bits consumed) | SEC-M01 |
| 4 | Cap `Config` resync params in `ConfigBuilder::build()` | SEC-M02 |
| 5 | Rename `max2pdf` binary; rewrite `max2pdf.py` cross-references | LEGAL-S01 |
| 6 | Add `NOTICE` file; add Trademark + input-legality lines to README | LEGAL-S02, LEGAL-N02, LEGAL-N03 |
| 7 | Exclude `tools/` from crate or document `encode-fixture` | LEGAL-S03 |
| 8 | Eliminate per-line allocations in dispatch + decoder hot path | PERF-01, PERF-02, PERF-03 |
| 9 | Clean `pdf.rs` `.unwrap()` storm; standardize on `div_ceil` | API-01, API-02 |
| 10 | Add `cargo-fuzz` target + 1 h smoke run | (hardening) |
| 11 | `CONTRIBUTING.md` with PR license posture; minor README polish | LEGAL-N06, LEGAL-N04 |
| 12 | Defense-in-depth and nits | API-03..09, SEC-L* |

---

## Critical (blockers under untrusted-input model)

### CRIT-01 — Chunk-header parsing panics on truncated input  ✅

**Resolved 2026-05-10** (commit pending). `find_image_chunks` now requires `length >= IMAGE_CHUNK_MIN_LEN` (0x42), making the dispatcher's header reads safe-by-invariant rather than safe-by-luck. A new `read_u16_at` helper in `chunks.rs` provides defense-in-depth for the preview decoder (which previously also had unchecked indexing). Six new integration tests in `tests/malformed.rs` exercise the public `decode_max` API on malformed inputs (short chunk, exact-min chunk, gigantic-length chunk, no `DL` magic, no `ViGBe` magic, empty buffer) — all return `Err(_)` instead of panicking. Decision: kept `decode_image_chunk` returning `Page` (rather than `Result<Page>` as the security review suggested) because the invariant is now enforced at chunk-discovery time; the smaller API churn is preferable pre-publish. If a future caller bypasses `find_image_chunks`, header reads are still safe via the same invariant on `ChunkRef`.

**Source:** Security C1 · Rust quality C1, C2
**Files:** `src/chunks.rs:32`, `src/dispatch.rs:58-66`, `src/preview.rs:52-67`

`find_image_chunks` accepts a chunk advertising `length = 1`. `decode_image_chunk` then reads at relative offsets up to `0x42` and panics on slice OOB. `decode_preview_chunk` has the same pattern and additionally underflows `chunk_start + chunk_length - preview_size` if `preview_size > chunk_length`. The library docs explicitly disclaim hard panics.

**Trigger:** `b"ViGBe" + zeros + b"DL" + 0x00000010_u32 + 0x00014000_u32` (chunk claims 16 bytes total).

**Fix:** Centralize in a single helper to remove the duplicated panic surface:

```rust
// src/chunks.rs
pub(crate) struct ChunkHeader {
    pub width: u16, pub height: u16, pub dpi_x: u16, pub dpi_y: u16,
    pub bits_per_pixel: u16,
    pub preview_size: u16, pub preview_width: u16, pub preview_height: u16,
}

impl ChunkHeader {
    pub fn parse(data: &[u8], chunk_start: usize, chunk_length: usize)
        -> Option<Self>
    {
        if chunk_length < 0x42 { return None; }
        if chunk_start.checked_add(chunk_length)? > data.len() { return None; }
        let r = |off| u16::from_le_bytes(
            data[chunk_start + off..chunk_start + off + 2].try_into().ok()?
        );
        // ... build struct
    }
}
```

Also tighten `find_image_chunks`: require `length >= 0x42`. Change `decode_image_chunk` return type to `Result<Page>`; surface a new `MaxError::TruncatedChunk { offset, length }`.

---

### CRIT-02 — Unbounded `width × height` allocation  ✅

**Resolved 2026-05-10** (commit pending). Added `MAX_IMAGE_PIXELS = 200 * 1024 * 1024` constant (exported at the crate root) and `MaxError::ImageTooLarge { width, height, pixels, max }` variant. `decode_image_chunk` now returns `Result<Page, MaxError>`; checks `width as u64 * height as u64 <= MAX_IMAGE_PIXELS` before allocating, and uses `checked_mul` on `row_bytes * height` for 32-bit safety. `lib.rs::decode_max` propagates via `?`. Cap chosen to comfortably exceed 600-DPI A4 (~35 MP) while bounding the worst-case bitmap allocation at ~25 MB. New tests in `tests/malformed.rs` cover the pathological 65535×65535 case, the just-over-cap 16384×16384 case, and verify realistic A4 dimensions still pass.

**Source:** Security C2
**Files:** `src/dispatch.rs:77-79`

```rust
let line_bytes  = width.div_ceil(8) as usize;
let row_bytes   = (line_bytes + 3) & !3usize;
let mut bitmap  = vec![0u8; row_bytes * height as usize];
```

`width` and `height` are `u16` (max 65 535 each). A 64-byte chunk header can request ~537 MB. On 32-bit targets `row_bytes * height` can wrap and produce a small allocation followed by OOB writes via `bitmap[y * row_bytes ..]`.

**Trigger:** chunk header with `width = 0xFFFF, height = 0xFFFF`.

**Fix:**

```rust
const MAX_PIXELS: usize = 200 * 1024 * 1024; // 200 MP
let pixels = (width as usize).checked_mul(height as usize)
    .ok_or(MaxError::ImageTooLarge { width, height })?;
if pixels > MAX_PIXELS {
    return Err(MaxError::ImageTooLarge { width, height });
}
let bytes = row_bytes.checked_mul(height as usize)
    .ok_or(MaxError::ImageTooLarge { width, height })?;
let mut bitmap = vec![0u8; bytes];
```

Apply the same cap to `padded_x × preview_y` in `preview.rs:69-79` (this is SEC-H02; combine the fix with CRIT-02 so the cap lives in one place).

---

## High

### SEC-H01 — Short preview chunk panic + underflow  ✅

**Resolved 2026-05-10** (commit pending). `decode_preview_chunk` now reads header fields via `read_u16_at` (returns `None` on OOB rather than panicking) and explicitly bails when `preview_size > chunk_length` (avoiding `usize` underflow on the `chunk_start + chunk_length - preview_size` offset arithmetic). Two new tests in `preview.rs::tests` cover the underflow case and the undersized-chunk case.

**Source:** Security H1
**Files:** `src/preview.rs:52-67`

Preview reader unconditionally indexes `data[chunk_start + 0x3c .. 0x42]`; underflows `chunk_start + chunk_length - preview_size` if `preview_size > chunk_length`.

**Trigger:** chunk with `chunk_length = 0x40, preview_size = 0xFFFF`.

**Fix:** subsumed by CRIT-01 if `ChunkHeader::parse` is the gateway. Additionally:

```rust
let offset = chunk_start.checked_add(chunk_length)
    .and_then(|end| end.checked_sub(preview_size))?;
```

---

### SEC-H02 — `padded_x × preview_y` allocation overflow  ✅

**Resolved 2026-05-10** (commit pending). Added `MAX_PREVIEW_PIXELS = 16 * 1024 * 1024` (exported alongside `MAX_IMAGE_PIXELS`). `decode_preview_chunk` now uses `checked_mul` on `padded_x * preview_y`, returns `None` if either the multiplication overflows or the product exceeds the cap. New test `pathological_preview_dimensions_skip_preview_no_panic` verifies the main image still decodes when the preview metadata is malicious — the preview is just skipped silently.

**Source:** Security H2
**Files:** `src/preview.rs:69-79`

A 6-byte preview header can request ~4 GB on 64-bit; on 32-bit it wraps and corrupts.

**Trigger:** `preview_x = 0xFFFF, preview_y = 0xFFFF, preview_size = 1`.

**Fix:** combine with CRIT-02 cap. Apply `checked_mul` and `MAX_PREVIEW_PIXELS`.

---

### SEC-H03 — Skip-line `pos += 1 + line_bytes` 32-bit overflow  ⬜

**Source:** Security H3
**Files:** `src/dispatch.rs:127, 150`

Bounded today (`line_bytes ≤ 8192`) but kept as defense-in-depth for 32-bit targets.

**Fix:**
```rust
pos = pos.checked_add(1 + line_bytes).unwrap_or(n);
```

---

## Medium

### SEC-M01 — Zero-progress dispatcher loop  ✅

**Resolved 2026-05-10** (commit pending) — **investigated and found to be already mitigated**. The security review's analysis of this finding contained an error: it claimed `pos += 0` on a zero-bit FAIL with no other advance, but the dispatcher does `pos += 1` for the marker consume *before* calling `decomp_line`. Net advance per type-2 iteration is `1 + consumed_bytes`, which is always ≥1. Combined with the same `pos += 1` lower-bound on every other dispatch arm (type 0 stray / raw-copy / skip; type 1 suppress; type 3 BLANK), the dispatcher loop is bounded at `O(chunk_length)` iterations.

To prevent future regressions, added (a) a **forward-progress-invariant comment** at the top of the main decode loop documenting which arm advances `pos` by what minimum, and (b) a regression test `zero_consume_type2_fails_terminate_in_bounded_time` that constructs a 16 KiB chunk filled with `0x80 0x00` pairs (each pair: type-2 marker + bytes whose top-7 prefix has no TAB7 match — `0b0000000`) and asserts decode completes in under 5 seconds. If a future change accidentally drops the marker `pos += 1`, the test catches it.

No production code change needed beyond the comment.

**Source:** Security M3
**Files:** `src/dispatch.rs` (after the type-2 arm, around line 195-205)

A type-2 marker followed by bytes that fail to match TAB7 returns `consumed = 0`. `pos += 0`; `y += 1`. The loop terminates eventually (when `y >= height`) but does `O(height)` wasted work for one byte of input.

**Trigger:** 64-byte chunk header with `width = height = 65535`, body = single `0x80`. Decoder does 65 535 FAILs.

**Fix:**

```rust
if consumed_bytes == 0 {
    pos += 1; // forward progress on degenerate FAIL
}
```

---

### SEC-M02 — Unbounded `fail_resync_max × fail_resync_lookahead`  ✅

**Resolved 2026-05-10** (commit pending). Added internal caps `MAX_RESYNC_K = 32`, `MAX_RESYNC_LOOKAHEAD = 64`, `MAX_RESYNC_BUDGET = 1024` at the top of `decode_image_chunk`; the user-supplied `cfg.fail_resync_*` values are clamped to local variables `cfg_fail_resync_max` / `cfg_fail_resync_lookahead` / `resync_budget_remaining` that are used throughout the resync block. `cfg.fail_resync_budget == 0` (previously meant "unlimited" via `u32::MAX`) now means "use the cap" (= 1024) — semantically harmless because real workflows never approach 1024 isolated FAILs per page.

Caps chosen to comfortably exceed any value with corpus utility (`fail_resync_max = 4` was the 10th-session champion; `fail_resync_lookahead = 5` is the default). New test `pathological_resync_config_does_not_hang` builds a chunk that triggers an isolated FAIL after a BLANK (so `prev_kind = Ok` opens the resync gate), runs `decode_max` with `fail_resync_max = u32::MAX` and similar, and asserts completion in <5 s. Without the cap the loop would iterate ~16 quintillion times and the test would hang the test binary.

Decision: clamp at use site rather than in `ConfigBuilder::build()` — the public `Config` fields are `pub`, so callers can bypass the builder via direct field access. Use-site clamping is the only enforcement that holds.

**Source:** Security M2
**Files:** `src/config.rs` (in `ConfigBuilder::build()`), `src/dispatch.rs:315`

Default is off (`fail_resync_max = 0`), but `Config` accepts arbitrary `u32`. With pathological values: 2 × 10¹² CCITT decodes per FAIL.

**Fix:** clamp in `ConfigBuilder::build()`:

```rust
fail_resync_max:       cfg.fail_resync_max.min(32),
fail_resync_lookahead: cfg.fail_resync_lookahead.min(64),
fail_resync_budget:    cfg.fail_resync_budget.min(1024),
```

---

### SEC-M03 — `make_sentinel` allocates per page  ⬜

**Source:** Security M5 / Rust M2
**Files:** `src/dispatch.rs:14-19, 87, 164, 278, 310, 353, 391`

`sentinel.clone()` (~10 KB) on every type-3 BLANK reset, every `reset_ref_after_drift`, every `t0_reset`, every smart-resync. Defense-in-depth + perf.

**Fix:** allocate once per page; reuse via `ref_table.clear(); ref_table.extend_from_slice(&sentinel);`. The `ref_for_probe = ref_table.clone()` at `dispatch.rs:310` is redundant — `resync_probe` already does its own clone at line 320.

---

### SEC-M04 — CLI path traversal note  ⬜

**Source:** Security M6
**Files:** `src/bin/max2pdf.rs:177-188`

Not a security issue for the binary (user owns their args), but embedders reusing the path-construction logic in a service should know. **Action:** add a doc-comment to `decode_max_file` warning embedders not to take `output_dir` from untrusted input.

---

## Performance

### PERF-01 — `decomp_line` allocates a `Vec<i32>` per line  ⬜

**Source:** Rust quality M3
**Files:** `src/decoder.rs:79-223`

3508-row page = 3508 short-lived ~10 KB allocations.

**Fix:** take `out: &mut Vec<i32>` as scratch; `out.clear()` per line.

---

### PERF-02 — Ref-table rebuild via `chain().collect()` per OK line  ⬜

**Source:** Rust quality M4
**Files:** `src/dispatch.rs:281-284`, `src/decoder.rs:372-375`

```rust
let ref_table: Vec<i32> = once(-1)
    .chain(table[1..].iter().copied())
    .chain(repeat_n(width, 16))
    .collect();
```

~10 KB allocated per OK line. Bench fixture is too narrow to expose.

**Fix:**
```rust
ref_table.clear();
ref_table.push(-1);
ref_table.extend_from_slice(&table[1..]);
ref_table.extend(std::iter::repeat_n(width, 16));
```

Pre-publish: add a wider bench fixture (or a `cfg(feature = "corpus")` bench against a real `.max`) so the fix is measurable.

---

### PERF-03 — `sentinel.clone()` in dispatch hot loop  ⬜

**Source:** Rust quality M2
**Files:** see SEC-M03 above (same fix)

---

## API / Code-quality

### API-01 — `pdf.rs` `.unwrap()` storm on `write!` to `Vec<u8>`  ⬜

**Source:** Rust quality M1
**Files:** `src/pdf.rs:65, 70, 97, 102, 104, 113, 144, 155, 183, 184`

Writes are infallible but read as panic surface. Once CRIT-01/CRIT-02 are fixed, the lib advertises panic-freedom — `pdf.rs` should match.

**Fix:** `expect("Vec<u8> writes are infallible")` or a tiny helper macro. `let _ = write!(buf, ...)` also acceptable.

---

### API-02 — Mixed `(x + 7) / 8` vs `x.div_ceil(8)`  ⬜

**Source:** Rust quality M5
**Files:** `src/decoder.rs:352`, `src/dispatch.rs:205` (manual); `src/dispatch.rs:77` (correct)

`div_ceil` stable since 1.73; clippy `manual_div_ceil` will fire.

**Fix:** standardize on `div_ceil`.

---

### API-03 — `BitCursor` visibility / `#[allow(dead_code)]` cleanup  ⬜

**Source:** Rust quality Mi1
**Files:** `src/bitstream.rs:14, 35, 48, 55, 66, 74, 82, 89, 94`

`pub fn` on a `pub(crate) struct`; pervasive `#[allow(dead_code)]` from staged Tasks 1–10. Most are now reachable.

**Fix:** drop `#[allow(dead_code)]` after `cargo check`; convert `pub` → `pub(crate)` for consistency.

---

### API-04 — `Config` not `PartialEq`  ⬜

**Source:** Rust quality Mi5
**Files:** `src/config.rs:53`

One-line ergonomics win for tests and downstream users.

**Fix:** `#[derive(Debug, Clone, PartialEq, Eq)]`.

---

### API-05 — `parse_dispatch_kinds` duplicates `T0DropMode::FromStr` pattern  ⬜

**Source:** Rust quality Mi6
**Files:** `src/bin/max2pdf.rs:113-128`

**Fix:** implement `FromStr for DispatchKind` in `config.rs`; binary calls `.split(',').map(str::trim).map(DispatchKind::from_str).collect()`.

---

### API-06 — `flate2` not feature-gated  💬

**Source:** Rust quality Mi7
**Files:** `Cargo.toml`, `src/pdf.rs`

Users who only want `decode_max(...) -> Vec<Page>` still pay for `flate2`.

**Fix (proposal):**
```toml
[features]
default = ["pdf"]
pdf = ["dep:flate2"]
```

Discussion needed: pre-1.0 fine to add; once published, harder to undo if someone depends on the unfeatured API. Probably ship as-is in 0.1.0 and add the feature in 0.2.0 with the unfeatured being a re-export shim.

---

### API-07 — `MaxError::Truncated` field naming  ⬜

**Source:** Rust quality Mi8
**Files:** `src/error.rs`, `src/lib.rs:67`

`MaxError::Truncated { need: 0x40, have: data.len() }` is misleading when emitted because no chunks were found.

**Fix:** add `MaxError::NoImageChunks` variant.

---

### API-08 — `unreachable!()` on `marker >> 6` arms  ⬜

**Source:** Rust quality Mi2
**Files:** `src/dispatch.rs:399`, `src/decoder.rs:386`, `src/dispatch.rs:142`

Correct but heavy. Stylistic.

**Fix:** restructure `match` to bind `let typ = (marker >> 6) & 0b11;` and exhaust 0..=3, OR keep `_ => unreachable!("marker top-2-bits is 0..3")` for the comment.

---

### API-09 — Various nits  ⬜

**Source:** Rust quality Mi3, Mi4, N1–N5
- `try_into().unwrap()` after explicit bounds check in `chunks.rs:28-29` — cosmetic.
- `&'static [(u32, u32, u32)]` for const tables — cosmetic.
- Magic `85 = 0xFF/3` constant in `preview.rs:32`.
- `dpi_x: u32` defaulting silently → `Option<u32>` would be more honest.
- `#[allow(clippy::too_many_arguments)]` on `resync_probe` and `emit_page_for_bitmap` → struct-of-args.

---

## Security defense-in-depth

### SEC-L01 — Document panic-freedom contract  ⬜

After CRIT-01/CRIT-02/SEC-H01/SEC-M01 are fixed, add to `lib.rs`:

> This crate guarantees no panic on any `&[u8]` input. Malformed `.max` data returns `Err(MaxError::*)`.

Add a `tests/no_panic_smoke.rs` that feeds 10 000 random buffers and asserts no panic.

---

### SEC-L02 — `cargo-fuzz` target + CI step  ⬜

**Source:** Security §3 fuzzing recommendation

```rust
// fuzz/fuzz_targets/decode_max.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use vigb_decoder::{decode_max, Config};

fuzz_target!(|data: &[u8]| {
    if data.len() > 4 * 1024 * 1024 { return; }
    let _ = decode_max(data, &Config::default());
});
```

Seed: 4 canonical pages + minimised `b"ViGBe" + b"DL" + len + flags + ...` skeletons + all-zero / all-`0xff` files at 64 / 256 / 1 KiB / 64 KiB. Run for 1 h with ASan before tagging.

---

### SEC-L03 — `#![deny(clippy::indexing_slicing)]`  ⬜

After switching to `.get()? / Option` patterns in `chunks.rs` + `preview.rs` + `dispatch.rs`, this lint catches future regressions of CRIT-01 / SEC-H01 at compile time.

---

### SEC-L04 — Document amplification ratio in README  ⬜

Once the megapixel cap is in place, add to README: "A `.max` file of N bytes can produce up to M MB of output bitmap." Embedders need this for size-limit policy.

---

## Legal

### LEGAL-S01 — `max2pdf` binary name collides with GPL `orangeturtle739/max2pdf`  ✅

**Resolved 2026-05-10** (commit pending). Renamed binary `max2pdf` → `vigb-max2pdf`:
- `Cargo.toml` `[[bin]]` name + path
- `src/bin/max2pdf.rs` → `src/bin/vigb-max2pdf.rs` (via `git mv` to preserve history)
- `#[command(name = ...)]` and module doc-comment in the renamed file
- README install snippet ("the `vigb-max2pdf` binary") + 3 usage examples
- `docs/cli.md` heading + 5 example commands
- `.github/workflows/release.yml` build + artifact rename steps
- Verified: `cargo run --bin vigb-max2pdf -- --version` outputs `vigb-max2pdf 0.1.0`

Also disambiguated cross-reference comments so readers can't confuse the in-repo Python sibling with the GPL `orangeturtle739/max2pdf` project: `max2pdf.py:` → `python-reference/max2pdf.py:` across `src/decoder.rs`, `src/preview.rs`, `src/chunks.rs`, `src/pdf.rs`, `src/dispatch.rs`, `src/config.rs`, `src/ccitt.rs`, `tests/common/encoder.rs`, `tools/encode-fixture/main.rs`, `tests/corpus.rs`. The `ccitt.rs` provenance note was further clarified to spell out "not the GPL `paperman` or `orangeturtle739/max2pdf` projects" rather than the bare "not max2pdf".

`python-reference/max2pdf.py` itself was NOT renamed — kept the .py name to avoid breaking existing Python users; the path prefix in cross-references provides the disambiguation.

**Follow-up 2026-05-10**: per user request for full consistency, the Python sibling was also renamed to `python-reference/vigb_max2pdf.py` (underscore variant chosen to keep `import vigb_max2pdf` working — hyphens would break Python's module-import grammar). Updated: root README "Pure-Python alternative" section + invocation example; `python-reference/README.md` heading + comparison table + import example + usage block; `docs/cli.md` link target; the Python file's own `Usage:` docstring; and all `python-reference/max2pdf.py:` cross-references in `src/`, `tests/`, `tools/` to `python-reference/vigb_max2pdf.py:`. Verified `python python-reference/vigb_max2pdf.py --help` runs.

**Source:** Legal S-1
**Files:** `Cargo.toml:25`, `src/bin/max2pdf.rs:11`, `README.md` install snippet, `docs/cli.md`, every `src/*.rs` cross-reference comment

`docs/credits.md:34` explicitly distances from this GPL project. Cross-reference comments like "mirrors `max2pdf.py:_decomp_line`" then read as if this crate ports the GPL Python project. PATH-shadowing on `cargo install`.

**Fix:**
1. Rename binary to `vigb-max2pdf` (or `vigbdec`). Update `Cargo.toml`, `#[command(name=...)]`, README install instructions, `docs/cli.md` heading.
2. Search-and-replace in `src/*.rs`: `max2pdf.py:` → `python-reference/max2pdf.py:`.
3. Optionally rename `python-reference/max2pdf.py` → `python-reference/vigb_decode.py`.

---

### LEGAL-S02 — Missing `NOTICE` file  ⬜

**Source:** Legal S-2

Future-proofs Apache-2.0 §4(d) reciprocity once any future contribution requires NOTICE attribution.

**Fix:** create `NOTICE` at repo root:
```
vigb-decoder
Copyright 2026 Christian Regg
```

---

### LEGAL-S03 — `tools/` shipped to crates.io  ⬜

**Source:** Legal S-3
**Files:** `Cargo.toml:12, 28-30`

`tools/encode-fixture/` is published; `[[bin]] name = "encode-fixture"` declares an extra binary that's only meaningful inside the dev workflow.

**Fix:** either remove the `[[bin]]` and add `"tools/"` to `exclude`, or keep both with a `//!` doc clarifying it's a dev-only tool not intended for end users.

---

### LEGAL-N01 — `paperport` keyword  ⏭️

**Source:** Legal N-1

Pure nominative use. Keep as-is. Optionally add `"vigbe"` as a fifth keyword.

---

### LEGAL-N02 — Trademarks disclaimer in README  ⬜

**Source:** Legal N-2

Standard practice for nominative-use RE projects.

**Fix:** add above or below License section:
> *PaperPort and Visioneer are trademarks of their respective owners. This project is not affiliated with, endorsed by, or sponsored by Tungsten Automation, Kofax, Nuance Communications, ScanSoft, or Visioneer.*

---

### LEGAL-N03 — Input-file legality disclaimer in README  ⬜

**Source:** Legal N-3

**Fix:** append to License section:
> *Users are responsible for the legality of input `.max` files they decode and the resulting PDF output they redistribute. The decoder does not assert or transfer any copyright in the decoded content.*

---

### LEGAL-N04 — Wayback snapshot of archive.org Visioneer ISO link  ⬜

**Source:** Legal N-4
**Files:** `README.md:97-98`, `docs/credits.md:9-11`

Belt-and-suspenders against future takedown.

**Fix:** capture `https://web.archive.org/web/2026*/https://archive.org/details/PaperPort_Deluxe_Visioneer_Version_5.2_1997` and cite alongside the bare archive.org URL.

---

### LEGAL-N05 — T.6 vs T.4 pedantic note  ⏭️

**Source:** Legal N-5

`lib.rs:5`, `bin/max2pdf.rs:1` say "CCITT-T.6 (Group 4 fax)" — correct framing for the format; `ccitt.rs:13-17` correctly cites T.4 (1988) for the table values. No legal issue. Leave as-is.

---

### LEGAL-N06 — `CONTRIBUTING.md` with PR license posture  ⬜

**Source:** Legal N-6

**Fix:** create `CONTRIBUTING.md` at repo root:
```markdown
# Contributing

By submitting a pull request, you agree that your contribution is licensed
under the same terms as this project (MIT OR Apache-2.0). No CLA required.
```

---

### LEGAL-N07 — Optional courtesy heads-up to Tungsten Automation legal  💬

**Source:** Legal pre-publish checklist item 10

Not asking permission — informing them. Creates paper trail of good-faith disclosure that strengthens safe-harbor posture if ever questioned. User had previously chosen "no prior correspondence" deliberately; reopening only if comfortable.

---

## Pre-publish checklist (mechanical)

| # | Item | Status |
|---|------|--------|
| 1 | `cargo clippy --all-targets -- -D warnings` clean | ⬜ |
| 2 | `cargo audit` clean | ⬜ |
| 3 | `cargo package --list` reviewed (no `python-reference/`, no `docs/superpowers/`, includes `tests/fixtures/synthetic.max`) | ⬜ |
| 4 | `cargo doc --no-deps` renders cleanly; `Preview` polarity warning matches `Page` | ⬜ |
| 5 | `cargo publish --dry-run` succeeds | ⬜ |
| 6 | MSRV claim 1.85 verified (or downshifted to 1.82) | ⬜ |
| 7 | `#![deny(missing_docs)]` (currently `warn`) | ⬜ |
| 8 | `#[must_use]` on builder methods + top-level decode/write functions | ⬜ |
| 9 | `git grep -i "MAXKER\|ViGBe.*\\(license\\|copyright\\)"` returns nothing surprising | ⬜ |
| 10 | `git tag v0.1.0` + push | ⬜ |
