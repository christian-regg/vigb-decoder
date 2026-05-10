# Decoder behaviour and canonical fixes

This document describes the decoder pipeline, the four canonical fixes
that bring it to bit-perfect accuracy on the reference test suite, the
smart-resync heuristic, and the reverse-engineering basis from
`MAXKER2.DLL`.

## Decoder pipeline overview

Processing one page from a `.max` file:

```
chunk_start
    │
    ▼
find_image_chunks          — scan DL stream for type-0x4000 chunks
    │
    ▼
decode_image_chunk         — per-page outer loop
    │  reads one marker byte per iteration
    │  dispatches on top-2-bits:
    │    0x01  → raw_copy_line (width/8 bytes verbatim)
    │    0x03  → skip_line (advance pos, no output)
    │    0x80..0xBF → decomp_line (CCITT-T.6 decompressor)
    │    0xC0..0xFF → blank_run (low6+1 white lines)
    │    else  → sync-drift byte, suppressed
    │
    ▼
decomp_line                — CCITT-T.6 inner decoder
    │  consumes variable-length bit codes
    │  populates transition list for this row
    │  feeds transitions into reference table for next row
    │
    ▼
table_to_row               — transition list → packed 1-bit row bytes
    │
    ▼
PDF writer                 — collects rows into JBIG2-style 1-bit image,
                             embeds in PDF with optional preview page
```

The reference table carries transition positions from the current row
into the next. After a successful CCITT decode, the new transitions
replace the table. After a FAIL, the previous table is preserved (the
FAIL row renders white, but downstream rows have a meaningful reference).

## The four canonical fixes

These fixes were identified by diffing the decoder's output against
pixel-accurate bitmaps exported from PaperPort 3.6 under otvdm. All
four are applied by default; each has a CLI opt-out.

### Fix 1: BLANK skip count (`low6 + 1`, not `low6`)

**What it does.** When the dispatcher reads a type-3 BLANK marker
(`0xC0..0xFF`), the number of white lines to emit is `low6 + 1`,
not `low6`.

**Canonical evidence.** `seg2:0xC68 inc ax` in the canonical dispatcher's
type-3 handler adds 1 to the low-6 field before using it as the line
count.

**Impact.** Without this fix, the first bytes of dense pages caused a
3-row y-shift that propagated through the rest of the page. With the fix,
content lands at the correct vertical position.

**Opt-out.** This fix is always active; no opt-out flag.

### Fix 2: Initial a0 in `decomp_line`

**What it does.** The initial position `a0` at the start of each CCITT-T.6
line is `0`, not `-1`.

**Canonical evidence.** `seg2:0xD68 sub ax,ax` in the canonical inner
CCITT decoder zeroes AX (= initial a0) before the bit-decision tree.

**Impact.** With `a0 = -1`, every H-mode transition was emitted one pixel
left of the correct position. The x-shift corrupted the reference table
for the next line, cascading through the page.

**Opt-out.** This fix is always active; no opt-out flag.

### Fix 3: Skip scan-forward on first iteration

**What it does.** The reference-table scan-forward loop
(`while ref[tp_idx] <= x: tp_idx += 2`) is skipped on the **first**
iteration of each line. On subsequent iterations it runs normally.

**Canonical evidence.** The canonical inner decoder jumps directly to the
bit-decision body on iteration 1 (skipping the walk + parity correction),
with `lodsw` reading the reference entry blindly.

**Impact.** Files where the first transition of a line is at column 0
(a valid `b1 = 0`) triggered the scan-forward spuriously, skipping the
valid b1 and silently dropping the first row of body content in affected
files.

**Opt-out.** This fix is always active; no opt-out flag.

### Fix 4: Canonical reference-table walk (Bug 4) — the most subtle fix

**What it does.** The reference-table pointer advances by consuming
(`lodsw`-style) one entry per code, rather than scanning forward to
the first entry that satisfies the CCITT-T.6 "first ref > a0 of opposite
color" criterion.

**Why this matters.** CCITT-T.6 allows two interpretations of the
reference walk: "consume-style" (advance past each used entry) vs
"scan-style" (find the first valid entry from the current position).
PaperPort 3.6 uses consume-style; because the encoder and decoder are in
lock-step, the encoded stream *assumes* consume-style on the decoder side.
Using scan-style produces a different b1 on lines with consecutive V-mode
codes, which corrupts transitions and cascades.

**Canonical evidence.** Disassembly of `seg2:0xD5E`:

| Code | Canonical si advance |
|------|---------------------|
| V_0 | `lodsw` → +1 index |
| V_L1, V_L2, V_L3 | `lodsw` → +1 index |
| V_R1, V_R2 | `lodsw` + up to 1 optional b2-skip (`add si, 4`) |
| V_R3 | `lodsw` + up to 2 optional b2-skips |
| P | `add si, 2; lodsw` → +2 indices |
| H | `while ref[si] ≤ a2: add si, 4` (walk-forward) |

The b2-skip on V_R{1,2,3} advances the reference pointer past the next
entry if `a1 >= ref[si]` — keeping `si` aligned to the same color class.

**The fix in `decomp_line`.** When `bug4 = true` (default):

1. The scan-forward loop at iteration start is skipped entirely.
2. V codes do `tp_idx += 1` (mirror `lodsw`). For V_R{1,2,3}, after
   `+= 1`, do up to 1 (V_R1/V_R2) or 2 (V_R3) b2-skip checks of
   `if x >= ref[tp_idx]: tp_idx += 2`.
3. P does `tp_idx += 2`.
4. H runs the canonical walk-forward: `while ref[tp_idx] <= x: tp_idx += 2`.

**Validation.** The regression test (`test_bug4` integration test) decodes
four files against PP 3.6 BMP exports and confirms:

| File | Before fix | After fix | T2 FAILs |
|------|------------|-----------|---------|
| Einzugsanzeige | 27.8% IoU | 100.0% IoU | 26 → 0 |
| Mietvertrag | 17.4% IoU | 100.0% IoU | 44 → 0 |
| Nachtrag p1 | 71.9% IoU | 100.0% IoU | 5 → 0 |
| Nachtrag p2 | 15.0% IoU | 100.0% IoU | 50 → 0 |

**CLI opt-out.** `--no-bug4` restores the pre-fix scan-forward scheme.
Useful for regression comparison against historical output.

## Heuristic defaults (sync-drift suppression)

These three heuristics defend against sync-drift: a misread byte gets
parsed as a marker and propagates errors through subsequent lines. All
are confirmed correct at the format-specification level by the canonical
disassembly.

### drop_blank_after_drift

When the dispatcher sees a type-3 BLANK marker AND the immediately-
preceding dispatch was low-confidence (`V0`, `FAIL`, `BAD`, `T1`, `T0`),
the marker byte is consumed but `y` is not advanced and the reference
table is not reset. The next byte gets a fresh dispatch.

**Why it works.** A sync-drift byte with top-2-bits `11` parses as a
"blank-line run × low6". With `low6` averaging ~32, each false BLANK
marker swallows ~32 rows of real content. Suppressing BLANK markers that
follow low-confidence dispatches eliminates the bulk of this loss.

**CLI opt-out.** `--keep-drift-blanks` restores the raw behavior.

### suppress_t1_all

All type-1 markers (`0x40..0x7F`) are treated as stray bytes: consume
the marker byte only, no `y` advance, no reference change.

**Why it works.** Type-1 markers are structurally invalid in ViGBe
(the canonical decoder returns error `-2` for the entire `0x40..0x7F`
range). A corpus scan found 4360 out-of-range type-1 dispatches vs 46
that coincidentally provided in-range position values — and those 46 are
individually 1-per-file coincidental drift bytes, not real type-1 markers.

**CLI opt-out.** `--keep-t1-dispatches` restores raw behavior.

### strict_t0

The type-0 dispatch (`top2 == 00`) is gated to only the two byte values
the canonical PaperPort 3.6 reader accepts:

- `0x01` → raw-copy one line (width/8 bytes from input).
- `0x03` → skip-line: consume marker + width/8 bytes, do NOT emit a row,
  do NOT advance `y`.
- Any other type-0 byte → drop as single stray byte; no `y` advance, no
  reference change. (Canonical aborts with error -2; the decoder
  approximates this as drop-and-continue for best-effort recovery.)

**Why it works.** The canonical reader explicitly rejects every type-0
byte except `0x01` and `0x03`. The pre-RE decoder treated every type-0
byte as a raw-copy, so a single sync-drift byte with top-2-bits `00`
would consume the next `width/8` bytes and emit them as a bitmap row,
advancing `y`. On real files, essentially all type-0 dispatches (96.5%
per corpus survey) are sync-drift artefacts. Strict mode drops them
without disturbing downstream dispatch.

**Note on IoU metric.** Strict mode regresses the raw IoU metric on
many pages because the pre-RE raw-copy behavior produced ~50%-density
bit patterns that *coincidentally* overlapped ground-truth text pixels.
Visual inspection confirms that the strict-mode output is cleaner and
more useful for document identification. PDF output size also drops
4–10× (spurious noise bands disappear).

**CLI opt-out.** `--no-strict-t0` restores the pre-RE raw-copy behavior.
Useful for A/B comparison against historical corpus runs.

## Smart-resync (opt-in)

Smart-resync is an optional heuristic for files where FAIL events are
isolated (not cascade-style dense files). It is disabled by default.

**The problem it solves.** A type-2 FAIL may over-consume bits, leaving
the stream position 1 or more bytes past the real next marker. This
single-byte misalignment can destroy all downstream IoU — the decoder
reads every subsequent line against the wrong reference.

**The mechanism.** After a type-2 FAIL (only when the preceding dispatch
was not also a FAIL/V0/BAD/T1 — i.e., on isolated FAILs), probe byte
offsets in `[-K, +K]` from the naive position. For each candidate, run a
non-mutating lookahead probe for `M` lines and score `n_ok - n_drift`.
Pick the offset with the highest score (tie-break: smaller `|offset|`).
Only commit the resync if the winning score is at least `C` (the
confidence gate).

**Results.** With `K=4, M=5, C=2`:
- Corpus-neutral (median Δ ≈ 0 pp, mean Δ ≈ 0 pp, roughly 50/50 wins/losses).
- Real per-file wins: up to +6 pp on files with a single large FAIL event.
- Real per-file losses: up to −3 pp on cascade-dense files.
- No file-level feature reliably predicts wins vs losses.

**Recommended use.** Try on a single file and visually inspect the output;
keep whichever version looks better.

**CLI flags.**

```
vigb-max2pdf bad.max --fail-resync-max 4 --reset-ref-after-drift --fail-resync-min-confidence 2
```

| Flag | Default | Effect |
|------|---------|--------|
| `--fail-resync-max K` | 0 (off) | Probe range ±K |
| `--fail-resync-lookahead M` | 5 | Lookahead lines per probe |
| `--fail-resync-min-confidence C` | 0 | Minimum score to commit |
| `--reset-ref-after-drift` | off | Reset reference table after resync |

## What was reverse-engineered from `MAXKER2.DLL`

PaperPort 3.6's `MAXKER2.DLL` (74 KB 16-bit NE DLL, "PaperPort Version 2
file read/write") contains the canonical ViGBe decoder and encoder. It was
extracted from the Visioneer 5.2 installer ISO using `idecomp` (the
InstallShield V3 `.Z` archive unpacker) without running the installer.

Key ordinals:

```
ord 505  PAXDOC2_PAGEREADIMAGE         — top-level decoder
ord  22  PAXFLT_IMAGEDECOMPRESS        — CCITT decode core
ord  29  PAXFLT_IMAGECOMPRESS          — CCITT encode core
ord  63  PAXFLT_IMAGEDECOMPRESSSETUP   — setup / strip layout
```

Findings from the disassembly:

1. **Per-line dispatcher** (seg2:0xBC0): reads one marker byte, extracts
   top-2-bits, dispatches to one of four handlers. All other marker values
   return error `-2`. This canonically validates the `suppress_t1_all` and
   `strict_t0` heuristics at the format-spec level.

2. **CCITT-T.6 inner decoder** (seg2:0xD5E): hand-coded bit-decision tree.
   Register `ch` holds an 8-bit shift register; `add ch, ch; jc/jnc`
   extracts the top bit. The code → action table matches the ITU-T T.6
   standard exactly. Reference pointer `ds:si` advances via `lodsw`
   (consume-style) — this is the source of Fix 4 above.

3. **CCITT-T.6 inner encoder** (seg2:0x7C5): PASS criterion = `b2 < a1`,
   V range = `|a1 − b1| ≤ 3`, H otherwise — **standard T.6**, no novel
   choice rule. The encoder always writes `0x80` as the line marker.

4. **Reference table**: initialized with `0x7FFF` end-of-list sentinel,
   4-byte stride, buffer size = `stripWidth × 16`.

5. **BLANK count**: `low6 + 1` (the `inc ax` at 0xC68 — source of Fix 1).

6. **Type-0 gate**: only `0x01` (raw-copy) and `0x03` (skip-line) are
   accepted. All other type-0 bytes are error — source of the `strict_t0`
   fix.

## Preview thumbnail

Each image chunk contains a 102×146 grayscale preview at the end, RLE-
compressed with a different dispatch from the main image stream. The
preview is **off by default** since the canonical decoder is bit-perfect
on the corpus; enable with `--preview` to append it as an extra PDF page
per source page. Useful for recovering layout when the main CCITT decode
fails (hand-drawn content, stamps).

See [format.md](format.md) for the preview RLE specification.

## See also

- [format.md](format.md) — file container structure and per-line dispatch spec
- [cli.md](cli.md) — all CLI flags
- [credits.md](credits.md) — canonical sources and prior art
- ITU-T Recommendation T.6 (1988) — the Group 4 fax coding standard
