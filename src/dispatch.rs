//! Per-line marker dispatcher for an image chunk. Direct port of
//! `max2pdf.py:decode_image_chunk` lines 437-808 (canonical defaults only;
//! Task 10 adds heuristic flag branches).

use crate::config::Config;
use crate::decoder::{decomp_line, table_from_raw, table_to_row, DecodeStats, Page};

/// Build the initial all-white sentinel reference table.
///
/// Mirrors Python: `[-1] + [width] * (width + 16)`.
fn make_sentinel(width: u32) -> Vec<i32> {
    let mut v = Vec::with_capacity(1 + width as usize + 16);
    v.push(-1i32);
    v.extend(std::iter::repeat_n(width as i32, width as usize + 16));
    v
}

/// Decode one image chunk starting at `chunk_start` in `data`. Returns
/// the rendered `Page` (preview field unset — populated separately in
/// Task 11). `chunk_length` is the chunk's total byte length from
/// `ChunkRef::length` and is used by Task 11's preview decoder.
pub(crate) fn decode_image_chunk(
    data: &[u8],
    chunk_start: usize,
    _chunk_length: usize,
    cfg: &Config,
) -> Page {
    // Read chunk header. Offsets per max2pdf.py:527-534.
    let read_u16 = |off: usize| {
        u16::from_le_bytes(
            data[chunk_start + off..chunk_start + off + 2]
                .try_into()
                .unwrap(),
        ) as u32
    };
    let width = read_u16(0x26);
    let height = read_u16(0x28);
    // DPI: clamp to 300 if zero (matches Python `dpi_x or 300`).
    let dpi_x = {
        let v = read_u16(0x2a);
        if v == 0 { 300 } else { v }
    };
    let dpi_y = {
        let v = read_u16(0x2c);
        if v == 0 { 300 } else { v }
    };
    // bpp at +0x2e — only 1-bit images supported. We assume bpp == 1
    // for v0.1 (matching Task spec note; Task 18 can add a runtime check).

    let line_bytes = width.div_ceil(8) as usize;
    let row_bytes = (line_bytes + 3) & !3usize;
    let mut bitmap = vec![0u8; row_bytes * height as usize];
    let mut stats = DecodeStats::default();

    // Sentinel reference table: [-1, width × (width+16)].
    let sentinel = make_sentinel(width);
    let mut ref_table = sentinel.clone();

    let mut pos = chunk_start + 0x42; // CCITT line stream starts at +0x42
    let n = data.len();
    let mut y: u32 = 0;
    let mut consecutive_fail: u32 = 0;
    // Track whether the last dispatch was a drift event (for type-3 blank
    // drop logic). Mirrors Python's `last_kind in drift_kinds` check where
    // drift_kinds = {'V0', 'FAIL', 'BAD', 'T1', 'T0'}.
    let mut last_was_drift = false;

    while y < height && pos < n {
        let marker = data[pos];
        let typ = marker >> 6;
        let low6 = (marker & 0x3F) as u32;

        match typ {
            0 => {
                // ── Type 0: raw uncompressed line / skip-line / stray ──────────
                stats.n_t0 += 1;
                if cfg.strict_t0 {
                    if low6 == 1 {
                        // Raw-copy line: read line_bytes, write into bitmap row.
                        pos += 1;
                        if pos + line_bytes <= n {
                            let row = &data[pos..pos + line_bytes];
                            let dst = &mut bitmap
                                [y as usize * row_bytes..y as usize * row_bytes + line_bytes];
                            dst.copy_from_slice(row);
                            // Update ref_table from the raw row, padded with 16
                            // trailing sentinels (matches Python line 631).
                            ref_table = table_from_raw(row, width as i32);
                            ref_table
                                .extend(std::iter::repeat_n(width as i32, 16));
                            pos += line_bytes;
                            y += 1;
                            consecutive_fail = 0;
                            last_was_drift = false;
                        } else {
                            // Truncated stream — bail out of the loop.
                            break;
                        }
                    } else if low6 == 3 {
                        // Skip-line: consume marker + line_bytes, no y advance,
                        // no output write, table_prev unchanged.
                        pos += 1 + line_bytes;
                        // last_was_drift unchanged (matches Python `continue`
                        // without touching last_kind).
                    } else {
                        // Stray type-0 byte — drop single byte.
                        pos += 1;
                        last_was_drift = true;
                    }
                } else {
                    // Non-strict path (diagnostic; Task 10 fleshes out).
                    // For now: treat as raw-copy (pre-RE legacy behaviour).
                    pos += 1;
                    if pos + line_bytes <= n {
                        let row = &data[pos..pos + line_bytes];
                        let dst = &mut bitmap
                            [y as usize * row_bytes..y as usize * row_bytes + line_bytes];
                        dst.copy_from_slice(row);
                        ref_table = table_from_raw(row, width as i32);
                        ref_table.extend(std::iter::repeat_n(width as i32, 16));
                        pos += line_bytes;
                        y += 1;
                        consecutive_fail = 0;
                        last_was_drift = false;
                    } else {
                        break;
                    }
                }
            }
            1 => {
                // ── Type 1: single-pixel positions ────────────────────────────
                stats.n_t1 += 1;
                if cfg.suppress_t1_all {
                    // Drop the marker only — no y advance, no ref_table change.
                    pos += 1;
                    last_was_drift = true;
                } else {
                    // Diagnostic path (Task 10). For now: drop marker only.
                    pos += 1;
                    last_was_drift = true;
                }
            }
            2 => {
                // ── Type 2: CCITT-T.6 compressed line ────────────────────────
                pos += 1;
                let (table, consumed_bits) = decomp_line(
                    data,
                    pos,
                    width as i32,
                    &ref_table,
                    cfg.lazy_bit_loading,
                    cfg.bug4,
                );
                let consumed_bytes = ((consumed_bits + 7) / 8) as usize;

                // The FAIL sentinel is [-1, width, width, width].
                let is_fail = table.len() == 4
                    && table[0] == -1
                    && table[1] == width as i32
                    && table[2] == width as i32
                    && table[3] == width as i32;

                // Distinguish V0 (fail sentinel returned with only 1 bit consumed)
                // vs real FAIL.
                let looks_v0 = is_fail && consumed_bits == 1;

                if is_fail && !looks_v0 {
                    // Real FAIL: emit all-white row (bitmap already zeroed),
                    // do NOT update ref_table.
                    stats.n_fail += 1;
                    consecutive_fail += 1;
                    stats.max_consecutive_fail =
                        stats.max_consecutive_fail.max(consecutive_fail);
                    if stats.first_fail_y.is_none() {
                        stats.first_fail_y = Some(y);
                    }
                    last_was_drift = true;
                } else if looks_v0 {
                    // V0-only line: decoder fell off after a single V(0) code.
                    // Treat similarly to FAIL for stats and ref tracking.
                    stats.n_v0 += 1;
                    consecutive_fail += 1;
                    stats.max_consecutive_fail =
                        stats.max_consecutive_fail.max(consecutive_fail);
                    last_was_drift = true;
                } else {
                    // Successful decode: write the rendered row + update ref.
                    stats.n_ok += 1;
                    consecutive_fail = 0;
                    let row = table_to_row(&table, width as i32, row_bytes);
                    let dst =
                        &mut bitmap[y as usize * row_bytes..(y as usize + 1) * row_bytes];
                    dst.copy_from_slice(&row);
                    // ref_table = [-1] + table[1:] + [width] * 16
                    // (matches Python line 698).
                    ref_table = std::iter::once(-1i32)
                        .chain(table[1..].iter().copied())
                        .chain(std::iter::repeat_n(width as i32, 16))
                        .collect();
                    last_was_drift = false;
                }
                pos += consumed_bytes;
                y += 1;
            }
            3 => {
                // ── Type 3: blank-line run ────────────────────────────────────
                if cfg.drop_blank_after_drift && last_was_drift {
                    // Suspect sync-drift: consume byte but don't advance y
                    // or reset reference. Matches Python `continue` without
                    // updating last_kind.
                    stats.blank_drops_after_drift += 1;
                    pos += 1;
                    // last_was_drift remains true.
                } else {
                    // Canonical: advance y by (low6 + 1) (seg2:0xC68 `inc ax`).
                    // Reset reference table to sentinel (matches Python line 781).
                    let advance = low6 + 1;
                    ref_table = sentinel.clone();
                    // Bitmap rows stay zero (already initialised).
                    let new_y = (y + advance).min(height);
                    y = new_y;
                    pos += 1;
                    last_was_drift = false;
                }
            }
            _ => unreachable!(),
        }
    }

    // Remaining rows are already zero (bitmap initialised to all-zero = white).

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
