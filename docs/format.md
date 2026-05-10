# ViGBe (.max) Format Specification

ViGBe is the PaperPort 2 (~1985–1990) container for scanned 1-bit images.
The format was reverse-engineered by bit-tracing `.max` files and confirmed
against the disassembly of PaperPort 3.6's `MAXKER2.DLL`
(`PAXFLT_IMAGEDECOMPRESS` at seg2:0xBC0). The per-line dispatch model
is proved correct by a CCITT-T.6 round-trip encoder that achieves 96%
byte-identical re-encoding on clean pages.

## File header

Magic bytes at offset 0: `ViGBe`. The trailing letter encodes the
PaperPort version: `ViGAe` = PaperPort 1, **`ViGBe` = PaperPort 2**,
`ViGCj`/`ViGFk` = later versions (not decoded by this implementation).

The file header runs from offset `0x00` to `0x90`. User-relevant fields
are inside each image chunk (see below).

## DL chunks

After the header, the file is a stream of `DL`-tagged chunks:

```
+----+--------+--------+---------+
| DL | length | flags  | payload |
| 2B | 4B LE  | 4B LE  | length-10 bytes |
+----+--------+--------+---------+
```

- `length` is the total chunk size including the `DL` header.
- `flags` low 16 bits = chunk type tag. `0x4000` = image chunk.
- `flags` high 16 bits = chunk index / page number (> 0 for image chunks).

Multi-page `.max` files contain multiple image chunks plus a page
directory chunk (`flags & 0xFFFF == 0x8000`).

The decoder scans for image chunks by walking the DL stream and
collecting all chunks with type `0x4000`.

## Image chunk header

Within each image chunk, offsets are relative to `chunk_start`:

| Offset | Field | Notes |
|-------:|-------|-------|
| 0x26 | `width` (uint16 LE) | typically 2464 (= 8.21″ × 300 dpi) |
| 0x28 | `height` (uint16 LE) | typically 3508 (= 11.69″ × 300 dpi) |
| 0x2A | `dpi_x` (uint16 LE) | usually 300 |
| 0x2C | `dpi_y` (uint16 LE) | usually 300 |
| 0x2E | `bpp` (uint16 LE) | 1 (only 1-bit images are decoded) |

Image data starts at `chunk_start + 0x42`.

## Per-line dispatch (canonical)

The image data is a stream of byte-aligned per-line markers. The
canonical PaperPort 3.6 decoder (`MAXKER2.DLL` seg2:0xBC0) reads one
marker byte from input, extracts the top 2 bits, and dispatches as
follows. Only **four valid markers** exist; all others return error `-2`.

| Marker | Top 2 bits | Action | Stride | Notes |
|--------|------------|--------|--------|-------|
| `0x01` | `00` | Raw-copy one line (`width/8` bytes input → output) | 1 + width/8 | Mode flags in caller |
| `0x03` | `00` | Skip one line (advance input by width/8; output unchanged; `y++`) | 1 + width/8 | "Drop this row" |
| `0x80..0xBF` | `10` | Type 2 — CCITT-T.6 compressed row | 1 + variable | Encoder always writes `0x80` (low6 = 0) |
| `0xC0..0xFF` | `11` | Type 3 — BLANK run, `count = low6 + 1` lines | 1 | Reference table reset to all-white sentinel |
| Anything else | `00` or `01` | **ERROR — marker is sync-drift** | — | Includes all `0x40..0x7F` and all type-0 except `0x01`/`0x03` |

The format is strict: type-1 markers (`0x40..0x7F`) are **never valid**
in ViGBe. A 60-file corpus scan found 4360 out-of-range type-1 dispatches
vs 46 coincidentally-in-range — all are sync-drift artefacts. See
[decoder.md](decoder.md) for the heuristics that suppress them.

### Marker `0x01` — raw-copy one line

After the marker byte, read exactly `(width + 7) / 8` bytes of 1-bit
input data. The calling context passes mode flags:

- bit `0x200` → "skip mode": advance input, don't write output.
- bit `0x100` → "OR-blend mode": `output |= input` per word.
- else → "replace mode": `output = input` (default).

The line counter increments by 1.

### Marker `0x03` — skip one line

After the marker byte, read `(width + 7) / 8` bytes of input but write
nothing. Line counter increments by 1; reference table not updated.

`0x03` appears to be a deliberate "drop this row" command, not a
sync-drift artefact — the canonical decoder handles it with explicit
structured behavior. Whether ViGBe encoders ever emit it legitimately is
unclear; no confirmed `0x03` has been observed in the wild.

### Type 1 — SINGLE (`0x40..0x7F`) — invalid

The canonical decoder treats **all** markers in this range as error `-2`.
Type-1 in the "single-pixel positions × low6" sense applies to later
PaperPort generations (ViGCj+), not ViGBe. The decoder suppresses all
type-1 markers by default; see [decoder.md](decoder.md).

### Type 2 — CCITT-T.6 (`0x80..0xBF`)

Marker followed by a CCITT-T.6 (ITU-T Recommendation T.6, "Group 4 fax")
compressed row, decoded against the previous row's transition list. The
canonical decoder uses the standard Huffman codes:

| Code | Action |
|------|--------|
| `1` | V_0 (a1 = b1) |
| `010` | V_L1 (a1 = b1 − 1) |
| `011` | V_R1 (a1 = b1 + 1) |
| `001` | H (horizontal: white run + black run) |
| `0001` | P (pass: a0′ = b2) |
| `000010` | V_L2 (a1 = b1 − 2) |
| `000011` | V_R2 (a1 = b1 + 2) |
| `0000010` | V_L3 (a1 = b1 − 3) |
| `0000011` | V_R3 (a1 = b1 + 3) |
| `000000…` | EOL / error (return error code) |

**The encoder always writes `0x80` (low6 = 0).** The ~12% of successful
type-2 decodes that start from non-zero-low6 markers are sync-drift bytes
that happen to begin a valid CCITT-T.6 sequence; suppressing them as a
variant loses more real content than it recovers, so the decoder leaves
low6 ignored in the dispatcher.

After each successful decode, the line's transition list becomes the
reference for the next type-2 line. After a FAIL, the previous reference
is preserved.

**Reference table layout** (from canonical disassembly): `ds:si` pointer
with 4-byte stride, terminated by `0x7FFF` end-of-list sentinel.

### Type 3 — BLANK (`0xC0..0xFF`)

`low6 + 1` lines are emitted as all-white. `y += low6 + 1`, `pos += 1`.
The reference table is reset to the all-white sentinel at each BLANK.

**Sync-drift bytes in `0xC0..0xFF` get parsed as type-3 markers and can
skip large stretches of real content.** Each false BLANK swallows an
average of ~32 rows. The decoder defaults to suppressing BLANK markers
that immediately follow low-confidence dispatches; see [decoder.md](decoder.md).

## Padding and embedded preview

After all image rows are decoded (`y == height`), the chunk contains:

1. Padding bytes (typically `0xFF`).
2. The embedded preview thumbnail.

The preview lives at `chunk_start + chunk_length - preview_size`.
It is a 102×146 grayscale image, RLE-compressed independently of the main
image stream, using a **different** top-2-bits dispatch:

| Type | Meaning |
|------|---------|
| `0x00..0x3F` | Emit `low6 × 4` zero pixels |
| `0x40..0x7F` | Emit `low6 × 4` 0xFF pixels |
| `0x80..0xBF` | Read `low6` literal bytes; 4 grayscale pixels per byte: bits 7-6, 5-4, 3-2, 1-0, each × 85 |
| `0xC0..0xFF` | Unknown; decoder skips with no output (~12% pixels lost) |

Output is inverted (bit 0 = white) and vertically flipped. The ~88% of
pixels produced from types 0–2 are sufficient for a clearly recognizable
document thumbnail even when the main CCITT stream fails badly.

## What is NOT in the format

Hypotheses tested and rejected during reverse engineering:

- **G3 EOL/EOFB markers** — `000000000001` does not appear at row boundaries.
- **Magic mid-stream sync bytes** — corpus survey for repeated byte patterns at
  row boundaries: nothing found.
- **Bit-reversed CCITT** — tested; all decodes produce garbage.
- **Type-0 = multi-line CCITT block** — tested; regresses every metric.
- **Type-2 with `low6 != 0` is multi-line** — corpus statistics do not support
  it; low6 distribution is uniform/random for non-zero markers.
- **Reference reset after type-2 FAIL** — coin-flip per page; not a clean win.

## Verification

A CCITT-T.6 round-trip encoder re-encodes the decoder's transition lists
and compares to the original `.max` bytes: 96% byte-identical on a clean
test page, 79% across a 20-file sample. Mismatches are CCITT-T.6 encoder
choice variance (e.g. V_R3 vs H mode for `|a1−b1| = 3` — multiple valid
encodings exist in the T.6 spec), not format issues. This round-trip proof
confirms the per-line dispatch model is correct.

## See also

- [decoder.md](decoder.md) — canonical decoder behaviour and heuristic fixes
- [cli.md](cli.md) — command-line flag reference
- ITU-T Recommendation T.6 (1988) — the Group 4 fax standard
- TIFF 6.0 Specification (Aldus, 1992) — cross-reference for CCITT tables
