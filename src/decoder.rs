//! Per-line CCITT-T.6 decoder. Direct port of `python-reference/max2pdf.py:_decomp_line`.

use crate::bitstream::BitCursor;
use crate::ccitt::{DispatchEntry, BLACK_TABLE, DISPATCH, TAB7, WHITE_TABLE};

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
    /// Preview width in pixels.
    pub width: u32,
    /// Preview height in pixels.
    pub height: u32,
    /// Bytes per row.
    pub row_bytes: u32,
    /// 1-bit packed, MSB-first, bit=1 means BLACK.
    pub bitmap: Vec<u8>,
}

/// Per-page decoder statistics. Soft-error counters live here, not in `MaxError`.
#[derive(Debug, Default, Clone)]
pub struct DecodeStats {
    /// Number of type-2 lines that decoded successfully.
    pub n_ok: u32,
    /// Number of V(0)-only-decode lines (rare).
    pub n_v0: u32,
    /// Number of type-0 markers consumed (raw, skip, or stray).
    pub n_t0: u32,
    /// Number of type-1 markers consumed.
    pub n_t1: u32,
    /// Number of type-2 lines that FAILed mid-decode.
    pub n_fail: u32,
    /// Longest consecutive FAIL run seen on this page.
    pub max_consecutive_fail: u32,
    /// y of the first FAIL on the page, if any.
    pub first_fail_y: Option<u32>,
    /// Smart-resync probes attempted (always 0 in Task 9; populated in Task 10).
    pub resync_probes: u32,
    /// Smart-resync probes that produced a usable offset (always 0 in Task 9).
    pub resync_hits: u32,
    /// Type-3 blank-line markers dropped after drift.
    pub blank_drops_after_drift: u32,
}

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
    let mut out: Vec<i32> = vec![-1];
    let mut tp_idx: usize = 1;
    let mut colour: u32 = 0;
    let mut x: i32 = 0; // canonical seg2:0xD68 zeros ax (a0 starts at 0, not -1)
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
            // Skipped on first iteration: canonical's lodsw reads table_prev[1]
            // directly. If we scan-forward when x=0 equals table_prev[1] (e.g.,
            // black starts at column 0), we'd skip past the valid first b1.
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
                // Read two run codes (alternating colour), then walk-forward
                // in the reference table past all consumed entries.
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
                        // Make-up code (run > 63): loop to read the following
                        // terminating code, accumulating run length in x.
                    }
                    out.push(x);
                    colour ^= 1;
                }
                if bug4 {
                    // H walk-forward (canonical seg2:0x154D): advance tp_idx by
                    // 2 idx while ref[tp_idx] <= a2 (the current x after both runs).
                    while (tp_idx < table_prev.len()) && (table_prev[tp_idx] <= x) {
                        tp_idx += 2;
                    }
                }
            }
            DispatchEntry::P => {
                // Pass mode: skip to b2 (table_prev[tp_idx + 1]).
                if tp_idx + 1 >= table_prev.len() {
                    return fail_table(width, &bc, start_pos);
                }
                x = table_prev[tp_idx + 1];
                if bug4 {
                    // Canonical P (seg2:0xDCA): add si, 2; lodsw → si advances
                    // by 4 bytes = +2 idx.
                    tp_idx += 2;
                }
                // Note: colour does NOT change in pass mode.
            }
            DispatchEntry::V(voff) => {
                if bug4 {
                    // Canonical si advance (12th-session Bug 4 fix):
                    // each V code does lodsw (+1 idx), plus optional b2-skip
                    // for V_R{1,2,3} when voff > 0 and x < width.
                    if tp_idx >= table_prev.len() {
                        return fail_table(width, &bc, start_pos);
                    }
                    let b1 = table_prev[tp_idx];
                    x = b1 + voff as i32;
                    out.push(x);
                    // Canonical lodsw: si += 1 idx.
                    tp_idx += 1;
                    // b2-skip for V_R1 (voff=1) or V_R2 (voff=2): max 1 step.
                    // For V_R3 (voff=3): max 2 steps.
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
                    // Legacy (pre-12th-session) path: tp_idx -= 1 after push,
                    // scan-forward happens at the top of the next iteration.
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
    let consumed =
        ((bc.next_load_byte() - start_pos) as i64) * 8 - bc.bits_buffered() as i64;
    (out, consumed)
}

fn fail_table(width: i32, bc: &BitCursor<'_>, start_pos: usize) -> (Vec<i32>, i64) {
    let consumed =
        ((bc.next_load_byte() - start_pos) as i64) * 8 - bc.bits_buffered() as i64;
    (vec![-1, width, width, width], consumed)
}

/// Convert a changing-elements table to a packed 1-bit MSB-first row.
///
/// Mirrors `python-reference/max2pdf.py:_table_to_row` (line 332). `row_bytes` is the
/// padded byte width of the output row.
///
/// The table format is `[-1, x0, x1, x2, ..., width, width]` where
/// each pair `(x[2k], x[2k+1])` for k ≥ 0 is a black run [x[2k], x[2k+1]).
/// Entries start at index 1 (skipping the leading -1 sentinel).
pub(crate) fn table_to_row(table: &[i32], width: i32, row_bytes: usize) -> Vec<u8> {
    let mut out = vec![0u8; row_bytes];
    let mut i = 1usize;
    let n = table.len();
    while i + 1 < n {
        let start = table[i].max(0);
        let mut end = table[i + 1];
        if start >= width {
            break;
        }
        if end > width {
            end = width;
        }
        if end > start {
            let sb = (start >> 3) as usize;
            let eb = ((end - 1) >> 3) as usize;
            if sb == eb {
                let lo = (start & 7) as u32;
                // hi = (end & 7) or 8 — Python's "or" means: if zero use 8
                let hi = if (end & 7) == 0 { 8u32 } else { (end & 7) as u32 };
                // Both operands are u8; the & 0xFF in Python is a no-op here.
                let mask = (0xFFu8 >> lo) & (0xFFu8 << (8 - hi));
                out[sb] |= mask;
            } else {
                out[sb] |= 0xFFu8 >> (start & 7) as u32;
                // Fill interior bytes. Use iter_mut to satisfy clippy's
                // needless_range_loop lint while keeping identical semantics.
                for byte in out.iter_mut().take(eb).skip(sb + 1) {
                    *byte = 0xFF;
                }
                let rem = (end & 7) as u32;
                if rem == 0 {
                    out[eb] = 0xFF;
                } else {
                    out[eb] |= 0xFFu8 << (8 - rem);
                }
            }
        }
        i += 2;
    }
    out
}

/// Build a changing-elements table from a packed 1-bit MSB-first row
/// (1 = black). Mirrors `python-reference/max2pdf.py:_table_from_raw` (line 360).
pub(crate) fn table_from_raw(row: &[u8], width: i32) -> Vec<i32> {
    let mut out: Vec<i32> = vec![-1];
    let mut colour: u32 = 0;
    for x in 0..width {
        let bit = ((row[(x >> 3) as usize] >> (7 - (x & 7) as u32)) & 1) as u32;
        if bit != colour {
            out.push(x);
            colour ^= 1;
        }
    }
    out.push(width);
    out.push(width);
    out
}

/// Lookahead probe used by `fail_resync_max`.
///
/// Walks the dispatcher for up to `n_steps` lines from `start_pos`
/// against `table_prev`. Returns `(n_ok, n_drift)` where `n_ok` counts
/// type-2 OK decodes and `n_drift` counts FAIL / V0 / BAD / T1 / type-0
/// events. Self-contained: writes no output, modifies no external state.
///
/// Mirrors `python-reference/max2pdf.py:_resync_probe` (line 383). Used only by
/// `dispatch::decode_image_chunk` when `cfg.fail_resync_max > 0`.
#[allow(clippy::too_many_arguments)]
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
    let mut pos = start_pos;
    let mut local_ref: Vec<i32> = table_prev.to_vec();
    let mut n_ok = 0u32;
    let mut n_drift = 0u32;
    let n = data.len();
    for _ in 0..n_steps {
        if pos >= n {
            break;
        }
        let marker = data[pos];
        let typ = marker >> 6;
        let low6 = (marker & 0x3F) as u32;
        match typ {
            0 => {
                // Python probe treats all type-0 as drift (no strict_t0
                // refinement needed — it just wants a rough score).
                if low6 == 1 || low6 == 3 {
                    // valid type-0: skip marker + payload
                    pos = pos.saturating_add(1 + line_bytes);
                } else {
                    pos += 1;
                }
                n_drift += 1;
            }
            1 => {
                // Suppress (treat as stray byte, same as ship default).
                pos += 1;
                // do not count step — mirrors Python `continue`
            }
            2 => {
                pos += 1;
                let (table, consumed_bits) =
                    decomp_line(data, pos, width, &local_ref, lazy, bug4);
                let consumed_bytes = ((consumed_bits + 7) / 8) as usize;
                let is_fail = table.len() == 4
                    && table[0] == -1
                    && table[1] == width
                    && table[2] == width
                    && table[3] == width;
                let looks_v0 = is_fail && consumed_bits == 1;
                // Compute valid_x check for BAD (mirrors Python).
                let tail = &table[1..];
                let x_final = if tail.len() >= 3 {
                    Some(tail[tail.len() - 3])
                } else {
                    tail.last().copied()
                };
                let valid_x = x_final.is_none_or(|xf| xf <= width + 3);
                let is_bad = !is_fail && !valid_x;
                if is_fail || looks_v0 || is_bad {
                    n_drift += 1;
                } else {
                    n_ok += 1;
                    local_ref = std::iter::once(-1i32)
                        .chain(table[1..].iter().copied())
                        .chain(std::iter::repeat_n(width, 16))
                        .collect();
                }
                pos += consumed_bytes;
            }
            3 => {
                // Type-3 BLANK: reset ref to sentinel, advance one step.
                local_ref = std::iter::once(-1i32)
                    .chain(std::iter::repeat_n(width, width as usize + 16))
                    .collect();
                pos += 1;
            }
            _ => unreachable!(),
        }
    }
    (n_ok, n_drift)
}

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
        let ref_table: Vec<i32> = vec![
            -1, width, width, width, width, width, width, width, width, width, width, width,
            width, width, width, width, width,
        ];
        // Encode V(0): 1 bit = 0b1, padded to a byte = 0x80.
        let data = [0x80u8];
        let (table, consumed) = decomp_line(&data, 0, width, &ref_table, false, true);
        assert!(consumed >= 1);
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
}
