//! One-time fixture generator. Produces tests/fixtures/synthetic.max +
//! tests/fixtures/synthetic.pbm from a 200×100 programmatic bitmap.
//!
//! Validate by running the Python decoder afterwards:
//!   python python-reference/max2pdf.py tests/fixtures/synthetic.max -o tests/fixtures/

#[path = "../../tests/common/encoder.rs"]
mod encoder;

use encoder::encode_row;
use std::io::Write;
use std::path::Path;

// ─── Bitmap builder ──────────────────────────────────────────────────────────

/// Build a 200×100 synthetic 1-bit bitmap (packed, MSB-first).
/// Pattern:
///   rows 0..24  : 8×8 checkerboard (inverted so x=0,y=0 cell is WHITE)
///   rows 25..74 : horizontal bars (2 px on every 10 rows, starting at x=8
///                 so the left edge is always white)
///   rows 75..99 : sparse pixels at x = 5, 22, 39, ... (step 17)
///
/// All rows start with at least one white pixel (x=0 is always white).
/// This is required because the CCITT-T.6 encoder uses a0=-1 (standard),
/// while the canonical ViGBe decoder uses x=0 (initial colour=white).
/// A row starting with a black pixel at x=0 causes a 1-pixel decode shift.
fn build_synthetic_bitmap(width: usize, height: usize) -> Vec<u8> {
    let row_bytes = width.div_ceil(8);
    let mut bits = vec![0u8; row_bytes * height];

    let mut set = |x: usize, y: usize| {
        bits[y * row_bytes + (x >> 3)] |= 0x80u8 >> (x & 7);
    };

    for y in 0..height {
        if y < height / 4 {
            // Checkerboard of 8×8 cells. Use `== 1` so that cell (0,0) at
            // x=0..7, y=0..7 is white (no black pixel at x=0).
            for x in 0..width {
                if ((x / 8) + (y / 8)) & 1 == 1 {
                    set(x, y);
                }
            }
        } else if y < 3 * height / 4 {
            // Horizontal bars: 2 px on every 10 rows.
            // Start at x=8 (not x=0) so x=0 is always white.
            if (y - height / 4) % 10 < 2 {
                for x in 8..width {
                    set(x, y);
                }
            }
        } else {
            // Sparse pixels every 17 columns. Starts at x=5 (white at x=0).
            for x in (5..width).step_by(17) {
                set(x, y);
            }
        }
    }
    bits
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a changing-elements transition table from a packed 1-bit row.
/// Returns sentinel table: `[-1, x0, x1, ..., width, width]`.
/// (Inlined here because src/decoder.rs is pub(crate) — not reachable from tools/.)
fn table_from_raw(row: &[u8], width: i32) -> Vec<i32> {
    let mut out: Vec<i32> = vec![-1];
    let mut colour: u32 = 0;
    for x in 0..width {
        let bit = ((row[(x >> 3) as usize]) >> (7 - (x & 7) as u32)) & 1;
        if bit as u32 != colour {
            out.push(x);
            colour ^= 1;
        }
    }
    out.push(width);
    out.push(width);
    out
}

/// Write a P4 (raw PBM) file.
fn write_pbm_p4(path: &Path, width: usize, height: usize, bits: &[u8]) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    write!(f, "P4\n{} {}\n", width, height)?;
    f.write_all(bits)?;
    Ok(())
}

// ─── .max file builder ───────────────────────────────────────────────────────

/// Encode the bitmap into a ViGBe .max byte stream.
///
/// File layout:
///   [0x00..0x05]  "ViGBe" magic
///   [0x05..0x40]  zero padding (to align chunk to 0x40)
///   [0x40..]      DL chunk (66-byte header + per-line CCITT stream)
///
/// Chunk header (offsets relative to chunk start = 0x40):
///   +0x00..0x02  "DL"
///   +0x02..0x06  u32 LE total chunk length (header + stream)
///   +0x06..0x0A  u32 LE flags = 0x0001_4000
///   +0x0A..0x26  zero-fill
///   +0x26..0x28  u16 LE width
///   +0x28..0x2A  u16 LE height
///   +0x2A..0x2C  u16 LE dpi_x = 300
///   +0x2C..0x2E  u16 LE dpi_y = 300
///   +0x2E..0x30  u16 LE bpp = 1
///   +0x30..0x3C  zero-fill
///   +0x3C..0x3E  u16 LE preview_size = 0
///   +0x3E..0x42  zero-fill
///   +0x42..end   CCITT line stream
fn encode_synthetic_max(bits: &[u8], width: u32, height: u32) -> Vec<u8> {
    let src_row_bytes = width.div_ceil(8) as usize;

    // Build the per-line CCITT stream.
    let mut line_stream: Vec<u8> = Vec::new();

    // All-white sentinel reference for line 0: [-1, width, width, ...]
    let mut prev_table: Vec<i32> = {
        let mut v = vec![-1i32];
        // Need at least width+2 entries beyond the sentinel so that
        // get(prev, b_idx) never panics — use width+16 copies.
        v.extend(std::iter::repeat_n(width as i32, width as usize + 16));
        v
    };

    for y in 0..height as usize {
        let row = &bits[y * src_row_bytes..(y + 1) * src_row_bytes];
        let curr_table = table_from_raw(row, width as i32);
        let body = encode_row(&curr_table, &prev_table, width as i32);

        // Type-2 marker: top 2 bits = 10, low 6 = 0 → 0x80.
        line_stream.push(0x80);
        line_stream.extend_from_slice(&body);

        prev_table = curr_table;
    }

    // Chunk header is 0x42 bytes.
    let mut chunk = vec![0u8; 0x42];
    chunk[0x00..0x02].copy_from_slice(b"DL");

    let chunk_length = (chunk.len() + line_stream.len()) as u32;
    chunk[0x02..0x06].copy_from_slice(&chunk_length.to_le_bytes());

    chunk[0x06..0x0A].copy_from_slice(&0x0001_4000u32.to_le_bytes());

    chunk[0x26..0x28].copy_from_slice(&(width as u16).to_le_bytes());
    chunk[0x28..0x2A].copy_from_slice(&(height as u16).to_le_bytes());
    chunk[0x2A..0x2C].copy_from_slice(&300u16.to_le_bytes()); // dpi_x
    chunk[0x2C..0x2E].copy_from_slice(&300u16.to_le_bytes()); // dpi_y
    chunk[0x2E..0x30].copy_from_slice(&1u16.to_le_bytes());   // bpp = 1

    // preview_size at +0x3C = 0 (already zero from vec initialisation).
    // preview_x, preview_y at +0x3E, +0x40 stay zero.

    // Assemble the file.
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"ViGBe");
    // Pad to chunk start at file offset 0x40.
    out.extend(std::iter::repeat_n(0u8, 0x40 - 5));
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&line_stream);
    out
}

// ─── main ────────────────────────────────────────────────────────────────────

fn main() -> std::io::Result<()> {
    let width = 200usize;
    let height = 100usize;

    let bits = build_synthetic_bitmap(width, height);

    std::fs::create_dir_all("tests/fixtures")?;

    write_pbm_p4(
        Path::new("tests/fixtures/synthetic.pbm"),
        width,
        height,
        &bits,
    )?;

    let max_bytes = encode_synthetic_max(&bits, width as u32, height as u32);
    std::fs::write("tests/fixtures/synthetic.max", &max_bytes)?;

    eprintln!(
        "wrote tests/fixtures/synthetic.max ({} bytes) and synthetic.pbm ({} bytes)",
        max_bytes.len(),
        5 + (0x40 - 5) + bits.len(), // approximate pbm size
    );
    Ok(())
}
