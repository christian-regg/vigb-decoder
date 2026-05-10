//! Per-line marker dispatcher for an image chunk. Direct port of
//! `max2pdf.py:decode_image_chunk` lines 437-808.
//!
//! Task 10 adds every heuristic flag branch. The canonical defaults
//! (bug4=true, strict_t0=true, drop_blank_after_drift=true,
//! suppress_t1_all=true) still produce n_fail=0 on the synthetic fixture.

use crate::chunks::MAX_IMAGE_PIXELS;
use crate::config::{Config, DispatchKind, T0DropMode};
use crate::decoder::{decomp_line, resync_probe, table_from_raw, table_to_row, DecodeStats, Page};
use crate::error::MaxError;

/// Build the initial all-white sentinel reference table.
///
/// Mirrors Python: `[-1] + [width] * (width + 16)`.
fn make_sentinel(width: u32) -> Vec<i32> {
    let mut v = Vec::with_capacity(1 + width as usize + 16);
    v.push(-1i32);
    v.extend(std::iter::repeat_n(width as i32, width as usize + 16));
    v
}

/// Return true if `kind` is in the effective drift-kinds set.
///
/// Python default: `drift_kinds = {'V0', 'FAIL', 'BAD', 'T1', 'T0'}`.
fn is_drift(kind: Option<DispatchKind>) -> bool {
    matches!(
        kind,
        Some(
            DispatchKind::V0
                | DispatchKind::Fail
                | DispatchKind::Bad
                | DispatchKind::T1
                | DispatchKind::T0
        )
    )
}

/// Return true if `kind` is in the effective t0-drift-kinds set.
///
/// When `t0_drop_kinds` is None it inherits the same default as `drift_kinds`.
fn is_t0_drift(kind: Option<DispatchKind>, cfg: &Config) -> bool {
    match &cfg.t0_drop_kinds {
        None => is_drift(kind),
        Some(set) => kind.is_some_and(|k| set.contains(&k)),
    }
}

/// Decode one image chunk starting at `chunk_start` in `data`.
///
/// `chunk_start` and `chunk_length` are expected to come from
/// [`crate::chunks::find_image_chunks`], which guarantees the chunk
/// satisfies `length >= IMAGE_CHUNK_MIN_LEN` and fits within `data`.
///
/// # Errors
///
/// Returns [`MaxError::ImageTooLarge`] if the chunk's declared
/// dimensions exceed [`MAX_IMAGE_PIXELS`].
pub(crate) fn decode_image_chunk(
    data: &[u8],
    chunk_start: usize,
    chunk_length: usize,
    cfg: &Config,
) -> Result<Page, MaxError> {
    // ── Chunk header ────────────────────────────────────────────────────────
    // Safe by find_image_chunks invariant: chunk_length >= IMAGE_CHUNK_MIN_LEN
    // and chunk_start + chunk_length <= data.len(), so all reads in
    // [chunk_start + 0x26 .. chunk_start + 0x42] are in bounds.
    let read_u16 = |off: usize| {
        u16::from_le_bytes(
            data[chunk_start + off..chunk_start + off + 2]
                .try_into()
                .unwrap(),
        ) as u32
    };
    let width = read_u16(0x26);
    let height = read_u16(0x28);
    let dpi_x = {
        let v = read_u16(0x2a);
        if v == 0 { 300 } else { v }
    };
    let dpi_y = {
        let v = read_u16(0x2c);
        if v == 0 { 300 } else { v }
    };
    // bpp at +0x2e — only 1-bit images supported.

    // CRIT-02: cap declared dimensions before any allocation. Without this,
    // a 64-byte chunk header with width = height = 0xFFFF requests
    // ~537 MB. checked_mul also defends 32-bit targets where row_bytes *
    // height could wrap.
    let pixels = (width as u64).saturating_mul(height as u64);
    if pixels > MAX_IMAGE_PIXELS {
        return Err(MaxError::ImageTooLarge {
            width,
            height,
            pixels,
            max: MAX_IMAGE_PIXELS,
        });
    }

    let line_bytes = width.div_ceil(8) as usize;
    let row_bytes = (line_bytes + 3) & !3usize;
    let bitmap_bytes = row_bytes
        .checked_mul(height as usize)
        .ok_or(MaxError::ImageTooLarge {
            width,
            height,
            pixels,
            max: MAX_IMAGE_PIXELS,
        })?;
    let mut bitmap = vec![0u8; bitmap_bytes];
    let mut stats = DecodeStats::default();

    // ── Reference table ─────────────────────────────────────────────────────
    // t0_reset is vestigial (canonical already resets to sentinel per-chunk).
    // We document: the sentinel IS the initial state; `t0_reset` flag is a
    // no-op because we never *not* start from sentinel.
    let sentinel = make_sentinel(width);
    let mut ref_table = sentinel.clone();

    // ── Stream state ─────────────────────────────────────────────────────────
    let mut pos = chunk_start + 0x42; // CCITT line stream starts at +0x42
    let n = data.len();
    let mut y: u32 = 0;
    let mut consecutive_fail: u32 = 0;

    // Rich last-kind tracking (replaces bool last_was_drift from Task 9).
    // Mirrors Python's `last_kind` variable (None | 'OK' | 'V0' | 'FAIL' |
    // 'BAD' | 'T0' | 'T1' | 'BLANK').
    let mut last_kind: Option<DispatchKind> = None;

    // SEC-M02: clamp user-supplied resync parameters to safe upper bounds
    // before use. Without these caps, a `Config` constructed with
    // pathological values (e.g. fail_resync_max = 1_000_000,
    // fail_resync_lookahead = 1_000_000) would cause `(2K + 1) * lookahead`
    // CCITT decode calls per FAIL event — quadratic in the user-supplied
    // params, multiplied by `fail_resync_budget` per page.
    //
    // Caps chosen empirically to comfortably exceed any value that produces
    // useful results on the corpus (`fail_resync_max = 4` was the
    // 10th-session champion; `fail_resync_lookahead = 5` is the default).
    // Pre-1.0: callers who hit these caps almost certainly have a bug.
    const MAX_RESYNC_K: u32 = 32;
    const MAX_RESYNC_LOOKAHEAD: u32 = 64;
    const MAX_RESYNC_BUDGET: u32 = 1024;
    let cfg_fail_resync_max = cfg.fail_resync_max.min(MAX_RESYNC_K);
    let cfg_fail_resync_lookahead = cfg.fail_resync_lookahead.min(MAX_RESYNC_LOOKAHEAD);
    // Smart-resync budget: 0 in Config means "use the cap" (was "unlimited"
    // pre-SEC-M02; capped at MAX_RESYNC_BUDGET to bound worst-case work).
    let mut resync_budget_remaining: u32 = if cfg.fail_resync_budget == 0 {
        MAX_RESYNC_BUDGET
    } else {
        cfg.fail_resync_budget.min(MAX_RESYNC_BUDGET)
    };

    // ── Main decode loop ─────────────────────────────────────────────────────
    //
    // Forward-progress invariant (SEC-M01): every dispatch arm below
    // advances `pos` by at least 1 byte before `continue`/loop-end:
    //   - type 0 (strict): `pos += 1` (stray marker)
    //   - type 0 (raw-copy / skip-line): `pos += 1 + line_bytes`
    //   - type 1: `pos += 1` (suppressed) or `pos = p` where `p > pos`
    //   - type 2: `pos += 1` (marker consume) THEN `pos += consumed_bytes`
    //             — even on a zero-bit FAIL the marker consume guarantees
    //             ≥1 byte of progress.
    //   - type 3: `pos += 1`
    // Therefore the loop runs in O(chunk_length) iterations and cannot
    // be exploited for unbounded amplification by a malicious bitstream.
    while y < height && pos < n {
        let marker = data[pos];
        let typ = marker >> 6;
        let low6 = (marker & 0x3F) as u32;

        match typ {
            // ── Type 0: raw uncompressed line / skip-line / stray ─────────
            0 => {
                stats.n_t0 += 1;

                if cfg.strict_t0 && low6 != 1 && low6 != 3 {
                    // Canonical: every type-0 byte with low6 ∉ {1,3} is a
                    // stray. Consume 1 byte, leave last_kind / table / y.
                    // Python: `continue` without touching last_kind.
                    pos += 1;
                    // Note: last_kind intentionally NOT updated (matches Python).
                    continue;
                }

                if low6 == 3 {
                    // Skip-line: consume marker + line_bytes, no y advance,
                    // no output write, table_prev unchanged (Python lines 588-600).
                    pos += 1 + line_bytes;
                    // last_kind intentionally NOT updated (Python `continue`).
                    continue;
                }

                // low6 == 1 (raw-copy) — or strict_t0 disabled (legacy path).

                // t0_drop_after_drift gate (H1 heuristic, 7th session).
                // Python lines 604-611.
                if cfg.t0_drop_after_drift != T0DropMode::None
                    && is_t0_drift(last_kind, cfg)
                {
                    match cfg.t0_drop_after_drift {
                        T0DropMode::Marker => pos += 1,
                        T0DropMode::Full => pos += 1 + line_bytes,
                        T0DropMode::None => unreachable!(),
                    }
                    // leave last_kind unchanged (Python `continue`)
                    continue;
                }

                // Consume marker + payload.
                pos += 1;
                let row_end = pos + line_bytes;
                if row_end > n {
                    // Truncated stream.
                    break;
                }
                let raw = &data[pos..row_end];

                // Write row to bitmap.
                let dst = &mut bitmap[y as usize * row_bytes..y as usize * row_bytes + line_bytes];
                dst.copy_from_slice(raw);

                // Update reference table (t0_reset: reset to sentinel instead
                // of building from raw bytes — diagnostic flag).
                if cfg.t0_reset {
                    ref_table = sentinel.clone();
                } else {
                    ref_table = table_from_raw(raw, width as i32);
                    ref_table.extend(std::iter::repeat_n(width as i32, 16));
                }

                pos += line_bytes;
                y += 1;
                consecutive_fail = 0;
                last_kind = Some(DispatchKind::T0);
            }

            // ── Type 1: single-pixel positions ────────────────────────────
            1 => {
                stats.n_t1 += 1;
                if cfg.suppress_t1_all {
                    // Drop marker only, no y advance. Python `continue`
                    // without touching last_kind.
                    pos += 1;
                    continue;
                }
                // Non-suppressed path: treat as stray (diagnostic).
                pos += 1;
                last_kind = Some(DispatchKind::T1);
            }

            // ── Type 2: CCITT-T.6 compressed line ────────────────────────
            2 => {
                // Capture prev_kind before any mutation (needed for
                // suppress_t2_fail_y_in_cascade and fail_resync_max gates).
                let prev_kind = last_kind;

                pos += 1; // consume marker byte
                let (table, consumed_bits) = decomp_line(
                    data,
                    pos,
                    width as i32,
                    &ref_table,
                    cfg.lazy_bit_loading,
                    cfg.bug4,
                );
                let consumed_bytes = ((consumed_bits + 7) / 8) as usize;

                let is_fail_sentinel = table.len() == 4
                    && table[0] == -1
                    && table[1] == width as i32
                    && table[2] == width as i32
                    && table[3] == width as i32;
                let looks_v0 = is_fail_sentinel && consumed_bits == 1;
                let is_real_fail = is_fail_sentinel && consumed_bits != 1;

                // BAD check: table is non-sentinel but x_final > width+3.
                let tail = &table[1..];
                let x_final = if tail.len() >= 3 {
                    Some(tail[tail.len() - 3])
                } else {
                    tail.last().copied()
                };
                let valid_x = x_final.is_none_or(|xf| xf <= width as i32 + 3);
                let is_bad = !is_fail_sentinel && !valid_x;

                // ── suppress_t2_fail_y_in_cascade ────────────────────────
                // When a cascade FAIL occurs (prev was also drift), skip y
                // advance, no row emit, leave last_kind and table_prev.
                // Python lines 680-685.
                if is_real_fail
                    && cfg.suppress_t2_fail_y_in_cascade
                    && is_drift(prev_kind)
                {
                    pos += consumed_bytes;
                    // last_kind intentionally NOT updated (Python `continue`).
                    continue;
                }

                // ── Emit row ─────────────────────────────────────────────
                let row = table_to_row(&table, width as i32, row_bytes);
                let dst = &mut bitmap[y as usize * row_bytes..(y as usize + 1) * row_bytes];
                dst.copy_from_slice(&row);

                // ── Update kind + stats ───────────────────────────────────
                let this_kind = if is_real_fail {
                    stats.n_fail += 1;
                    consecutive_fail += 1;
                    stats.max_consecutive_fail =
                        stats.max_consecutive_fail.max(consecutive_fail);
                    if stats.first_fail_y.is_none() {
                        stats.first_fail_y = Some(y);
                    }
                    DispatchKind::Fail
                } else if looks_v0 {
                    stats.n_v0 += 1;
                    consecutive_fail += 1;
                    stats.max_consecutive_fail =
                        stats.max_consecutive_fail.max(consecutive_fail);
                    DispatchKind::V0
                } else if is_bad {
                    consecutive_fail += 1;
                    stats.max_consecutive_fail =
                        stats.max_consecutive_fail.max(consecutive_fail);
                    DispatchKind::Bad
                } else {
                    stats.n_ok += 1;
                    consecutive_fail = 0;
                    DispatchKind::Ok
                };
                last_kind = Some(this_kind);

                // ── Update reference table ────────────────────────────────
                if cfg.reset_ref_after_drift
                    && matches!(
                        this_kind,
                        DispatchKind::Fail | DispatchKind::V0 | DispatchKind::Bad
                    )
                {
                    ref_table = sentinel.clone();
                } else if !is_real_fail {
                    // Successful (or V0/BAD) — update ref from table.
                    ref_table = std::iter::once(-1i32)
                        .chain(table[1..].iter().copied())
                        .chain(std::iter::repeat_n(width as i32, 16))
                        .collect();
                }
                // On real FAIL without reset_ref_after_drift: keep old ref.

                pos += consumed_bytes;
                y += 1;

                // ── Smart resync (fail_resync_max) ────────────────────────
                // Python lines 710-759. Only on isolated FAIL (prev not in
                // {FAIL, V0, BAD, T1}).
                if is_real_fail
                    && cfg_fail_resync_max > 0
                    && resync_budget_remaining > 0
                    && !matches!(
                        prev_kind,
                        Some(
                            DispatchKind::Fail
                                | DispatchKind::V0
                                | DispatchKind::Bad
                                | DispatchKind::T1
                        )
                    )
                {
                    // `pos` is now positioned after the FAIL's consumed bytes
                    // (the "naive" next position in Python).
                    let naive = pos;
                    let ref_for_probe = ref_table.clone();
                    let k = cfg_fail_resync_max as i64;
                    let mut best_off: i64 = 0;
                    let mut best_score: i64 = i64::MIN;

                    for off in -k..=k {
                        let cand = naive as i64 + off;
                        if cand <= (chunk_start + 0x42) as i64 || cand >= (n as i64) - 1 {
                            continue;
                        }
                        let cand_pos = cand as usize;
                        let (n_ok_p, n_drift_p) = resync_probe(
                            data,
                            cand_pos,
                            &ref_for_probe,
                            width as i32,
                            line_bytes,
                            cfg_fail_resync_lookahead,
                            cfg.bug4,
                            cfg.lazy_bit_loading,
                        );
                        let score = n_ok_p as i64 - n_drift_p as i64;
                        // Tie-break: prefer smaller absolute offset.
                        if score > best_score
                            || (score == best_score && off.unsigned_abs() < best_off.unsigned_abs())
                        {
                            best_score = score;
                            best_off = off;
                        }
                    }

                    // Record 2*K+1 probes for this FAIL event; decrement budget.
                    stats.resync_probes += 2 * cfg_fail_resync_max + 1;
                    resync_budget_remaining = resync_budget_remaining.saturating_sub(1);

                    // Confidence gate: commit only if margin >= min_confidence.
                    if best_off != 0
                        && best_score
                            >= cfg.fail_resync_min_confidence as i64
                    {
                        pos = (naive as i64 + best_off) as usize;
                        // Always reset ref after resync (matches Python line 757:
                        // `table_prev = list(sentinel)` unconditionally on commit).
                        ref_table = sentinel.clone();
                        stats.resync_hits += 1;
                    }
                } else if is_real_fail
                    && cfg.fail_scan_forward > 0
                {
                    // ── fail_scan_forward (H4 heuristic) ─────────────────
                    // Scan for 0x80 0xf8 byte pair. Python lines 760-769.
                    let scan_end = (pos + cfg.fail_scan_forward as usize).min(n.saturating_sub(1));
                    let mut sp = pos;
                    while sp < scan_end {
                        if data[sp] == 0x80
                            && sp + 1 < n
                            && data[sp + 1] == 0xf8
                        {
                            if sp != pos {
                                pos = sp;
                            }
                            break;
                        }
                        sp += 1;
                    }
                }
            }

            // ── Type 3: blank-line run ────────────────────────────────────
            3 => {
                if cfg.drop_blank_after_drift && is_drift(last_kind) {
                    // Suspect sync-drift: consume byte but don't advance y
                    // or reset reference. Python `continue` without touching
                    // last_kind.
                    stats.blank_drops_after_drift += 1;
                    pos += 1;
                    // last_kind remains unchanged.
                    continue;
                }
                // Canonical: advance y by (low6 + 1) (seg2:0xC68 `inc ax`).
                let advance = low6 + 1;
                ref_table = sentinel.clone();
                // Bitmap rows stay zero (already initialised).
                let new_y = (y + advance).min(height);
                y = new_y;
                pos += 1;
                last_kind = Some(DispatchKind::Ok); // 'BLANK' in Python; use Ok as closest
            }

            _ => unreachable!(),
        }
    }

    // Remaining rows are already zero (bitmap initialised to all-zero = white).

    let preview = if cfg.embed_preview {
        crate::preview::decode_preview_chunk(data, chunk_start, chunk_length, true)
    } else {
        None
    };

    Ok(Page {
        width,
        height,
        dpi_x,
        dpi_y,
        row_bytes: row_bytes as u32,
        bitmap,
        preview,
        stats,
    })
}
