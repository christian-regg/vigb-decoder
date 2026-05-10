# PaperPort `.max` File Format

## 1. Overview

`.max` is the document container format used by the PaperPort family of
desktop-scanning applications. A `.max` file holds one or more raster page
images, optional preview thumbnails, and a small amount of metadata
(page geometry, scan resolution, bit depth). Pages are stored as
1-bit-per-pixel rasters compressed with ITU-T Recommendation T.6
("CCITT Group 4"), with a per-line dispatch byte that allows a row to be
encoded as raw bits, a compressed bitstream, or a run of blank lines.

This document specifies the **PaperPort 2 generation** of the format
(magic `ViGBe`). Other PaperPort generations use the same five-byte
identifier scheme but differ in chunk layout and image encoding; see
[§3 File identification](#3-file-identification).

## 2. Conventions

- All multi-byte integers are **little-endian** unless stated otherwise.
- Field types: `u8`, `u16`, `u32` (unsigned, byte/short/long).
- Hex literals use lowercase with the `0x` prefix; multi-byte byte
  sequences are written space-separated, e.g. `56 69 47 42 65`.
- Byte offsets prefixed `+` are relative to the enclosing structure;
  unprefixed hex offsets are absolute file offsets.
- "Bit 0" of a byte is the least-significant bit. Bitstreams in image
  data are read **most-significant-bit first**.

## 3. File identification

A `.max` file begins with a 5-byte ASCII magic identifier of the form
`ViG?e` (the third character encodes the format generation):

| Magic       | ASCII bytes       | Generation                  |
|-------------|-------------------|-----------------------------|
| `ViGAe`     | `56 69 47 41 65`  | PaperPort 1                 |
| `ViGBe`     | `56 69 47 42 65`  | **PaperPort 2** (this spec) |
| `ViGCj`     | `56 69 47 43 6a`  | PaperPort 3 – 4             |
| `ViGEm`     | `56 69 47 45 6d`  | PaperPort 5 – 7             |
| `ViGFk`     | `56 69 47 46 6b`  | PaperPort 8 – 12            |

The file extension is `.max`. The container is **not** OLE2/CFBF; it is
a custom chunked stream (see §4).

## 4. High-level structure

```
+--------------------------------+   offset 0x00
|         File header            |   144 bytes (0x00..0x90)
+--------------------------------+   offset 0x90
|     Page directory chunk       |   "DL" chunk, type 0x8000
+--------------------------------+
|     Image chunk (page 1)       |   "DL" chunk, type 0x4000
+--------------------------------+
|     Image chunk (page 2)       |   ... one per page ...
+--------------------------------+
|              ...               |
+--------------------------------+
|         Trailer chunk          |   "DL" chunk, type 0x8005
+--------------------------------+
```

After the file header, the body of the file is a sequence of `DL`-tagged
chunks. The first chunk always begins at file offset `0x90` and is the
page directory. One image chunk follows for each page. A trailer chunk
terminates the stream.

### 4.1 File header (offset 0x00 – 0x90)

| Offset | Size | Type   | Field         | Description                                     |
|-------:|-----:|--------|---------------|-------------------------------------------------|
| 0x00   | 5    | char[5]| magic         | `ViGBe`                                         |
| 0x05   | 31   | —      | (zero)        | Reserved; observed value: zero                  |
| 0x24   | 4    | u32    | page_count    | Number of pages in the file                     |
| 0x28   | 10   | —      | (zero)        | Reserved                                        |
| 0x32   | 4    | u32    | chunk_count   | Total number of `DL` chunks (incl. directory + trailer) |
| 0x36   | 34   | —      | (zero)        | Reserved                                        |
| 0x58   | 4    | u32    | first_chunk   | Offset of first `DL` chunk; always `0x00000090` |
| 0x5c   | 32   | —      | (zero)        | Reserved                                        |
| 0x7c   | 4    | u32    | trailer_offset| Offset of the trailer chunk                     |
| 0x80   | 16   | —      | (zero)        | Reserved                                        |

The header occupies the first `0x90` bytes. The `first_chunk` field is
always `0x90`, i.e., the page directory chunk immediately follows the
header.

### 4.2 `DL` chunk header

Every chunk in the file begins with a 10-byte header:

| Offset | Size | Type   | Field         | Description                                     |
|-------:|-----:|--------|---------------|-------------------------------------------------|
| +0x00  | 2    | char[2]| magic         | `DL` (`44 4c`)                                  |
| +0x02  | 4    | u32    | length        | Total chunk size in bytes (header + payload)    |
| +0x06  | 4    | u32    | flags         | See below                                       |

The `flags` word encodes the chunk type in the low 16 bits and a
type-specific index in the high 16 bits:

```
 31                                       16 15                                    0
+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
|         page_index (u16)                  |             chunk_type (u16)              |
+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+--+
```

| `chunk_type` (low 16 bits) | Meaning                                                         |
|----------------------------|-----------------------------------------------------------------|
| `0x8000`                   | Page directory (§5)                                             |
| `0x4000`                   | Image chunk (§6); high 16 bits = 1-based page index             |
| `0x8005`                   | Trailer (§7)                                                    |

For non-image chunks, the high 16 bits of `flags` are zero.

The next chunk begins at `chunk_offset + length`.

## 5. Page directory chunk (`chunk_type = 0x8000`)

The page directory is a fixed 160-byte (`0xa0`) chunk listing the
location and geometry of every page image chunk in the file.

| Offset | Size | Type | Field             | Description                                 |
|-------:|-----:|------|-------------------|---------------------------------------------|
| +0x00  | 10   | —    | DL header         | `chunk_type = 0x8000`, `length = 0x000000a0`|
| +0x0a  | 22   | —    | (zero)            | Reserved                                    |
| +0x20  | 2    | u16  | page_count        | Equals the file-header `page_count`         |
| +0x22  | 10   | —    | (unspecified)     | Per-directory header fields, purpose unclear|
| +0x2c  | 2    | u16  | width_px          | Page width in pixels (first page)           |
| +0x2e  | 2    | u16  | height_px         | Page height in pixels (first page)          |
| +0x30  | 2    | u16  | dpi               | Resolution in dots per inch (first page)    |
| +0x32  | 2    | u16  | (unspecified)     | Purpose unclear                             |
| +0x34  | 4    | u32  | page_chunk_offset | Absolute file offset of this page's image chunk |

For files containing more than one page, the per-page descriptor
beginning at `+0x2c` repeats every 14 bytes for each subsequent page.

A conformant decoder MAY ignore this chunk and instead locate image
chunks by scanning forward for `DL` markers with `chunk_type = 0x4000`,
because the per-chunk `length` field allows safe traversal of the chunk
stream.

## 6. Image chunk (`chunk_type = 0x4000`)

An image chunk stores one page. Its layout for the `ViGBe` generation is:

| Offset | Size | Type | Field             | Description                                       |
|-------:|-----:|------|-------------------|---------------------------------------------------|
| +0x00  | 10   | —    | DL header         | `chunk_type = 0x4000`, `page_index` in high 16 bits |
| +0x0a  | 22   | —    | (zero)            | Reserved                                          |
| +0x20  | 4    | u32  | payload_size      | Size of `image data + preview` payload area       |
| +0x24  | 2    | —    | (zero)            | Reserved                                          |
| +0x26  | 2    | u16  | width_px          | Page width in pixels                              |
| +0x28  | 2    | u16  | height_px         | Page height in pixels                             |
| +0x2a  | 2    | u16  | dpi_x             | Horizontal resolution (DPI), typically `300`      |
| +0x2c  | 2    | u16  | dpi_y             | Vertical resolution (DPI), typically `300`        |
| +0x2e  | 2    | u16  | bits_per_pixel    | `1` for `ViGBe` (1-bit bilevel only)              |
| +0x30  | 2    | u16  | channels          | Reserved; observed value: `0`                     |
| +0x32  | 10   | —    | (unspecified)     | Purpose unclear                                   |
| +0x3c  | 2    | u16  | preview_size      | Size of preview RLE in bytes (0 if no preview)    |
| +0x3e  | 2    | u16  | preview_width     | Preview width in pixels (typically `102`)         |
| +0x40  | 2    | u16  | preview_height    | Preview height in pixels (typically `146`)        |
| +0x42  | …    | —    | image_data        | Per-line dispatch stream (§6.1)                   |
| (end)  | …    | —    | preview_rle       | RLE-compressed preview (§6.4); located at end of chunk |

The image data starts at chunk-relative offset `+0x42`. The preview RLE,
when present, is appended to the end of the chunk and is located at
absolute file offset `chunk_offset + length - preview_size`.

A conformant decoder MUST reject an image chunk whose `bits_per_pixel`
is not `1`.

### 6.1 Image data: per-line dispatch

The image data is a stream of byte-aligned per-line records. Each
record begins with a single **marker byte** that selects one of four
line-encoding modes from the top two bits, with the low six bits
carrying a mode-specific parameter.

```
   7 6 5 4 3 2 1 0
  +-+-+---+-+-+-+-+
  |TYPE |   PARAM   |
  +-+-+---+-+-+-+-+
   |     |
   |     +-- low 6 bits, "param"
   +-- top 2 bits, "type"
```

The marker byte is dispatched as follows:

| Marker        | Type | Param  | Mode                   | Following bytes                |
|---------------|:----:|:------:|------------------------|--------------------------------|
| `0x01`        | 0    | 1      | Raw line (§6.1.1)      | `ceil(width/8)` raw bytes      |
| `0x03`        | 0    | 3      | Skip line (§6.1.2)     | `ceil(width/8)` raw bytes      |
| `0x80 – 0xbf` | 2    | 0 – 63 | Compressed line (§6.1.3)| Variable-length T.6 bitstream |
| `0xc0 – 0xff` | 3    | 0 – 63 | Blank-line run (§6.1.4)| (none)                         |
| All others    | —    | —      | **Reserved**            | Behavior unspecified; see §6.1.5 |

Decoders maintain three pieces of per-page state during dispatch:

- `y` — the current output line index (`0` initially; incremented as
  each line is emitted).
- `output` — the destination raster of size `height_px × ceil(width_px/8)` bytes.
- `reference` — the **changing-elements table** of the previously
  emitted line, used as the reference for compressed lines. Initialized
  to the empty all-white reference (see §6.2). Reset to all-white
  whenever a blank-line run is processed.

Line bit-packing in `output` and in raw lines: the leftmost pixel of a
line is bit 7 of byte 0; pixel value `1` denotes black (foreground),
`0` denotes white (background).

Decoding terminates when `y == height_px`.

#### 6.1.1 Raw line (marker `0x01`)

The marker byte is followed by exactly `ceil(width_px / 8)` bytes
containing one row of the page in raw 1-bit-per-pixel form (MSB-first,
left-to-right). The decoder writes these bytes verbatim to row `y`,
sets `reference` to the changing-elements table of this row, and
increments `y` by 1.

#### 6.1.2 Skip line (marker `0x03`)

The marker byte is followed by exactly `ceil(width_px / 8)` bytes,
which are read from the stream and **discarded**. No row is written;
`y` is not incremented; `reference` is not modified.

This mode is structurally a "consume but do not emit"; conformant
encoders normally do not produce it, but decoders MUST handle it
without error.

#### 6.1.3 Compressed line (marker `0x80 – 0xbf`)

The marker byte is followed immediately by a CCITT-T.6 (ITU-T
Recommendation T.6, "Group 4 facsimile") two-dimensional bitstream
encoding one row of `width_px` pixels. The bitstream begins at the
first bit of the byte following the marker, and the **next line's
marker byte begins at the next byte boundary** after the last consumed
bit (i.e., each compressed line is byte-padded on the trailing edge).

Decoding follows ITU-T T.6 verbatim. The 2-D mode codes used are:

| Code (bits, MSB-first) | Mode  | Action                                  |
|------------------------|-------|-----------------------------------------|
| `1`                    | V₀    | a₁ = b₁                                 |
| `011`                  | VR(1) | a₁ = b₁ + 1                             |
| `010`                  | VL(1) | a₁ = b₁ − 1                             |
| `000011`               | VR(2) | a₁ = b₁ + 2                             |
| `000010`               | VL(2) | a₁ = b₁ − 2                             |
| `0000011`              | VR(3) | a₁ = b₁ + 3                             |
| `0000010`              | VL(3) | a₁ = b₁ − 3                             |
| `001`                  | H     | Horizontal: white run + black run, T.4 codes |
| `0001`                 | P     | Pass: a₀′ = b₂                          |

Horizontal-mode runs use the standard ITU-T T.4 white/black
terminating and make-up codes.

The low-6-bit parameter of the marker byte is reserved; encoders
write `0`, producing marker byte `0x80`. Decoders SHOULD ignore the
parameter value.

After successful decode, `reference` is updated to the changing
elements of the newly decoded row, and `y` is incremented by 1.

The per-row reference is initialized at the start of the page to the
all-white reference (see §6.2) and is reset to that same value
whenever a blank-line run (§6.1.4) is processed.

If the bitstream cannot be decoded (an undefined code prefix or a row
that does not terminate at column `width_px`), the decoder SHOULD
emit an all-white row at `y`, leave `reference` unchanged, and
re-synchronize on the next byte boundary.

#### 6.1.4 Blank-line run (marker `0xc0 – 0xff`)

A marker in the range `0xc0 – 0xff` emits a run of all-white lines.
The number of lines emitted is `param + 1`, i.e., one line for marker
`0xc0` through 64 lines for marker `0xff`:

```
n_lines = (marker & 0x3f) + 1
```

For each emitted line, `y` is incremented by 1; the line's bytes in
`output` are zero. `reference` is reset to the all-white reference
(§6.2). No additional bytes follow the marker.

If `y + n_lines > height_px`, only `height_px − y` lines are emitted
and decoding terminates normally.

#### 6.1.5 Reserved markers

Any marker byte not listed in §6.1's dispatch table (specifically,
marker bytes in the ranges `0x00`, `0x02`, `0x04 – 0x3f`, and
`0x40 – 0x7f`) is **reserved**. Conformant encoders MUST NOT emit
such bytes. A decoder encountering one MAY treat it as a stream
error and abort decoding the chunk.

### 6.2 The all-white reference

The CCITT-T.6 changing-elements reference for an empty (all-white) row
of width `width_px` is the table:

```
[ width_px, width_px, … ]
```

terminated by the sentinel `0x7fff`. (The first entry is
`width_px` because there is no color change before the right edge of
an all-white row.) This is the reference state used:

- before decoding the first compressed line of a page, and
- immediately after processing any blank-line run (§6.1.4).

### 6.3 Coordinate system and pixel order

Lines are emitted top-to-bottom (`y = 0` is the topmost line). Within
a line, pixels are written left-to-right, packed 8-per-byte, MSB-first
(pixel `x = 0` is bit 7 of byte 0). Bit value `1` denotes the
foreground (black) and `0` the background (white).

### 6.4 Preview thumbnail (RLE)

The preview thumbnail is an 8-bit-per-pixel grayscale image of size
`preview_width × preview_height` (typically `102 × 146`), compressed
with a byte-oriented run-length encoding distinct from §6.1. It is
located at file offset `chunk_offset + length − preview_size`.

The RLE stream uses the same top-2-bits / low-6-bits split as §6.1,
but with completely different semantics:

| Marker        | Type | Action                                                            |
|---------------|:----:|-------------------------------------------------------------------|
| `0x00 – 0x3f` | 0    | Emit `count × 4` pixels with value `0x00`                         |
| `0x40 – 0x7f` | 1    | Emit `count × 4` pixels with value `0xff`                         |
| `0x80 – 0xbf` | 2    | Read `count` literal bytes; each yields four 2-bit grayscale samples |
| `0xc0 – 0xff` | 3    | Reserved (purpose unclear); see §11                              |

where `count = marker & 0x3f`.

For Type 2 (literal bytes), each byte encodes four 2-bit grayscale
samples packed two bits per pixel, MSB-first. The 2-bit code is
expanded to an 8-bit grayscale value by multiplication by `85`:

```
for j in (6, 4, 2, 0):
    pixel = ((literal_byte >> j) & 0x3) * 85
```

producing values in `{0, 85, 170, 255}`.

Pixels are decoded in row-major order. Each row is padded out to a
multiple of four pixels (the "padded width" is
`(preview_width + 3) & ~3`). After the full pixel array is decoded, it
is **vertically flipped** (the decoded top row becomes the bottom row of
the displayed image).

After the flip, pixel value `0x00` denotes the background and `0xff`
denotes the foreground.

The total number of decoded pixels is
`padded_width × preview_height`; decoding stops when this many pixels
have been emitted or when `preview_size` bytes have been consumed.

## 7. Trailer chunk (`chunk_type = 0x8005`)

The trailer is a fixed 32-byte (`0x20`) chunk that marks the end of the
chunk stream:

| Offset | Size | Type | Field      | Description                                |
|-------:|-----:|------|------------|--------------------------------------------|
| +0x00  | 10   | —    | DL header  | `chunk_type = 0x8005`, `length = 0x00000020`|
| +0x0a  | 22   | —    | (unspecified) | Trailer body; purpose unclear           |

Decoders MAY use the trailer offset (from the file header at `0x7c`)
as a sanity check that the chunk stream is well-formed. Trailer
contents are not required for image decoding.

## 8. Compression and encoding summary

| Region                       | Encoding                                             |
|------------------------------|------------------------------------------------------|
| File header, chunk headers   | Plain little-endian fields                           |
| Image data, raw lines        | 1-bit-per-pixel raw, MSB-first, byte-aligned         |
| Image data, compressed lines | ITU-T Recommendation T.6 ("CCITT Group 4")           |
| Image data, blank-line runs  | Single-byte run-length (count = `param + 1`)         |
| Preview thumbnail            | Custom byte-oriented RLE with 2-bit grayscale literals |

CCITT-T.6 is normatively defined by the ITU-T; this document specifies
only the `.max`-specific framing of T.6 streams (one row per dispatch
record, byte-padded on the trailing edge, all-white initial reference,
reference reset on blank runs).

## 9. Page geometry and metadata

The complete metadata recorded for an image chunk is:

| Field                | Source                  | Notes                              |
|----------------------|-------------------------|------------------------------------|
| Page width (px)      | Image chunk `+0x26`     | Typical: `2464` (8.21" × 300 DPI)  |
| Page height (px)     | Image chunk `+0x28`     | Typical: `3508` (11.69" × 300 DPI) |
| Horizontal DPI       | Image chunk `+0x2a`     | Typical: `300`                     |
| Vertical DPI         | Image chunk `+0x2c`     | Typical: `300`                     |
| Bit depth            | Image chunk `+0x2e`     | `1` only for `ViGBe`               |
| Page index (1-based) | High 16 bits of `flags` | `1` for the first image chunk      |
| Page count           | File header `+0x24`     | Total pages in file                |

No timestamp, scan-device identifier, or per-document text/OCR layer
is stored in the `ViGBe` generation.

## 10. Generational differences

Differences between the `ViGBe` generation specified here and other
PaperPort generations (`ViGAe`, `ViGCj`, `ViGEm`, `ViGFk`):

- `ViGAe` (PaperPort 1) uses a chunk layout very similar to `ViGBe`
  but predates the image-chunk fields documented here at offsets
  `+0x32` and beyond.
- `ViGCj` (PaperPort 3 – 4) onwards changes the per-line dispatch:
  the type-1 marker range (`0x40 – 0x7f`) is reused as a
  "single-pixel positions" mode (low 6 bits = a count of 16-bit
  pixel-position pairs that follow). This mode is not present in
  `ViGBe`.
- `ViGCj` and later generations also support multi-bit grayscale and
  color images via larger `bits_per_pixel` values and additional
  metadata fields not documented here.

The first 5 bytes of the file are always sufficient to determine the
generation; decoders SHOULD validate the magic before applying this
specification.

## 11. Known unknowns

The following points are not fully specified and represent open
questions for future revisions:

- **File-header reserved fields.** The header at `0x00 – 0x90`
  contains additional non-zero fields in some files; their semantics
  are unknown. The four fields documented in §4.1 (`page_count`,
  `chunk_count`, `first_chunk`, `trailer_offset`) are sufficient to
  parse the file, but a complete header dictionary is unavailable.
- **Page directory unspecified shorts.** Offsets `+0x22` (10 bytes)
  and `+0x32` (2 bytes) within the page directory chunk are
  populated with non-zero values but their purpose is unknown.
- **Image chunk unspecified region.** Offsets `+0x32 – +0x3b` within
  an image chunk are non-zero in some files; purpose unknown.
- **Preview RLE Type 3.** Marker bytes `0xc0 – 0xff` in the preview
  RLE stream appear in real previews (~17 occurrences per typical
  preview) but their semantics are not known. Treating them as a
  no-op produces visually faithful previews but loses ~12% of pixel
  area.
- **Trailer body.** The 22 bytes of trailer payload after the DL
  header are non-zero but their meaning is not documented here.
- **Skip-line use.** The skip-line dispatch (marker `0x03`) has
  well-defined semantics but no `ViGBe` encoder has been observed
  to produce it.

## 12. Worked example

This section walks through a synthetic minimal `.max` file with one
1-bit page of `16 × 4` pixels at 300 DPI. The page contents are:

```
Row 0:  ................  (all white)
Row 1:  ################  (all black)
Row 2:  ########........  (left half black, right half white)
Row 3:  ................  (all white)
```

The file is 432 (`0x1b0`) bytes long.

### File header (`0x000 – 0x090`)

```
Offset  Bytes (hex)
0x000:  56 69 47 42 65 00 00 00  00 00 00 00 00 00 00 00   "ViGBe..........."
0x010:  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00
0x020:  00 00 00 00 01 00 00 00  00 00 00 00 00 00 00 00     ┐ page_count = 1
0x030:  00 00 03 00 00 00 00 00  00 00 00 00 00 00 00 00     ┘ chunk_count = 3
0x040:  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00
0x050:  00 00 00 00 00 00 00 00  90 00 00 00 00 00 00 00       first_chunk = 0x90
0x060:  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00
0x070:  00 00 00 00 00 00 00 00  00 00 00 00 90 01 00 00       trailer_offset = 0x190
0x080:  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00
```

### Page directory chunk (`0x090 – 0x130`)

```
Offset  Bytes (hex)
0x090:  44 4c a0 00 00 00 00 80  00 00 00 00 00 00 00 00   "DL".... DL header: type 0x8000, length 0xa0
0x0a0:  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00
0x0b0:  01 00 00 00 00 00 00 00  00 00 00 00 10 00 04 00      page_count=1, width=0x10, height=0x04
0x0c0:  2c 01 00 00 30 01 00 00  00 00 00 00 00 00 00 00      dpi=300, page_chunk_offset=0x130
0x0d0..0x12f: zeros (padding to 0xa0 bytes total)
```

### Image chunk (`0x130 – 0x190`)

```
Offset  Bytes (hex)                                       Field
0x130:  44 4c 60 00 00 00 00 40  01 00 00 00 00 00 00 00   DL header (type 0x4000, page=1, length=0x60)
0x140:  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00   reserved (22 bytes from +0x0a..+0x1f)
0x150:  1e 00 00 00 00 00 10 00  04 00 2c 01 2c 01 01 00   payload_size=0x1e; (zero); width=0x10; height=4; dpi_x=300; dpi_y=300; bpp=1
0x160:  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00   channels=0; (10 bytes unspecified); preview_size=0
0x170:  00 00 c0 01 ff ff 01 ff  00 c0 00 00 00 00 00 00   preview_w=0; preview_h=0; image_data starts at 0x172
0x180:  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00   chunk padding (image data ends at 0x17a)
```

### Image data dispatch

Walking the image data byte stream starting at `0x172`:

| Pos    | Marker | Mode             | Action                                          | y      |
|--------|--------|------------------|-------------------------------------------------|--------|
| 0x172  | `c0`   | Blank-line × 1   | Emit row 0 = all white; `y → 1`                 | 0 → 1  |
| 0x173  | `01`   | Raw line         | Read 2 bytes (`ff ff`), write to row 1; `y → 2` | 1 → 2  |
| 0x176  | `01`   | Raw line         | Read 2 bytes (`ff 00`), write to row 2; `y → 3` | 2 → 3  |
| 0x179  | `c0`   | Blank-line × 1   | Emit row 3 = all white; `y → 4`                 | 3 → 4  |

`y` reaches `height_px = 4`; image decoding terminates. The remaining
bytes of the image chunk (`0x17a – 0x18f`) are unused padding.

### Trailer chunk (`0x190 – 0x1b0`)

```
Offset  Bytes (hex)
0x190:  44 4c 20 00 00 00 05 80  00 00 00 00 00 00 00 00   "DL".... DL header: type 0x8005, length 0x20
0x1a0:  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00
```

End of file at `0x1b0`.

### Decoded raster

After dispatch, the page raster (4 rows × 2 bytes per row, MSB-first,
`1` = black):

```
Row 0:  00 00     ................
Row 1:  ff ff     ################
Row 2:  ff 00     ########........
Row 3:  00 00     ................
```

This matches the source page.
