"""
Convert PaperPort 2 (.max) scans to PDF — Python reference decoder.

Sibling implementation to the Rust crate `vigb-decoder` in this repo.
Same algorithm; same bit-perfect output on the canonical test corpus.
Use this script when a Python interpreter is easier to reach than a
Rust toolchain.

Licensed MIT OR Apache-2.0 (matches the Rust crate). The CCITT-T.6
lookup tables embedded below are facts from the public ITU-T T.6 (1988)
specification, cross-checked against the TIFF 6.0 Specification (Aldus,
1992, public domain). Values were copied from `src/ccitt.rs` in this
repo, which transcribed them clean-room from those two PDFs. No code
or table values from `paperman` (GPL-2-or-later) is used. See
`docs/provenance.md` for the full clean-room separation notes.

Usage:
    python vigb_max2pdf.py <input.max> [<input2.max> ...] [-o out_dir]
"""

from __future__ import annotations

import argparse
import io
import struct
import sys
import zlib
from pathlib import Path


# ---------------------------------------------------------------------------
# CCITT T.6 standard tables.
#
# Provenance: copied from `src/ccitt.rs` in this repo, which was transcribed
# clean-room from CCITT Recommendation T.6 (1988) and the TIFF 6.0
# Specification (Aldus 1992, public domain). Same numbers as paperman or
# any other CCITT-T.6 implementation — they are facts from a public ITU
# standard, not paperman's intellectual property.
# ---------------------------------------------------------------------------
WHITE_TERM = [
    (8, 0x35), (6, 0x07), (4, 0x07), (4, 0x08), (4, 0x0B), (4, 0x0C),
    (4, 0x0E), (4, 0x0F), (5, 0x13), (5, 0x14), (5, 0x07), (5, 0x08),
    (6, 0x08), (6, 0x03), (6, 0x34), (6, 0x35), (6, 0x2A), (6, 0x2B),
    (7, 0x27), (7, 0x0C), (7, 0x08), (7, 0x17), (7, 0x03), (7, 0x04),
    (7, 0x28), (7, 0x2B), (7, 0x13), (7, 0x24), (7, 0x18), (8, 0x02),
    (8, 0x03), (8, 0x1A), (8, 0x1B), (8, 0x12), (8, 0x13), (8, 0x14),
    (8, 0x15), (8, 0x16), (8, 0x17), (8, 0x28), (8, 0x29), (8, 0x2A),
    (8, 0x2B), (8, 0x2C), (8, 0x2D), (8, 0x04), (8, 0x05), (8, 0x0A),
    (8, 0x0B), (8, 0x52), (8, 0x53), (8, 0x54), (8, 0x55), (8, 0x24),
    (8, 0x25), (8, 0x58), (8, 0x59), (8, 0x5A), (8, 0x5B), (8, 0x4A),
    (8, 0x4B), (8, 0x32), (8, 0x33), (8, 0x34),
]
WHITE_MAKEUP = [
    (5, 0x1B), (5, 0x12), (6, 0x17), (7, 0x37), (8, 0x36), (8, 0x37),
    (8, 0x64), (8, 0x65), (8, 0x68), (8, 0x67), (9, 0xCC), (9, 0xCD),
    (9, 0xD2), (9, 0xD3), (9, 0xD4), (9, 0xD5), (9, 0xD6), (9, 0xD7),
    (9, 0xD8), (9, 0xD9), (9, 0xDA), (9, 0xDB), (9, 0x98), (9, 0x99),
    (9, 0x9A), (6, 0x18), (9, 0x9B),
    (11, 0x08), (11, 0x0C), (11, 0x0D), (12, 0x12), (12, 0x13),
    (12, 0x14), (12, 0x15), (12, 0x16), (12, 0x17), (12, 0x1C),
    (12, 0x1D), (12, 0x1E), (12, 0x1F),
]
BLACK_TERM = [
    (10, 0x37), (3, 0x02), (2, 0x03), (2, 0x02), (3, 0x03), (4, 0x03),
    (4, 0x02), (5, 0x03), (6, 0x05), (6, 0x04), (7, 0x04), (7, 0x05),
    (7, 0x07), (8, 0x04), (8, 0x07), (9, 0x18), (10, 0x17), (10, 0x18),
    (10, 0x08), (11, 0x67), (11, 0x68), (11, 0x6C), (11, 0x37),
    (11, 0x28), (11, 0x17), (11, 0x18), (12, 0xCA), (12, 0xCB),
    (12, 0xCC), (12, 0xCD), (12, 0x68), (12, 0x69), (12, 0x6A),
    (12, 0x6B), (12, 0xD2), (12, 0xD3), (12, 0xD4), (12, 0xD5),
    (12, 0xD6), (12, 0xD7), (12, 0x6C), (12, 0x6D), (12, 0xDA),
    (12, 0xDB), (12, 0x54), (12, 0x55), (12, 0x56), (12, 0x57),
    (12, 0x64), (12, 0x65), (12, 0x52), (12, 0x53), (12, 0x24),
    (12, 0x37), (12, 0x38), (12, 0x27), (12, 0x28), (12, 0x58),
    (12, 0x59), (12, 0x2B), (12, 0x2C), (12, 0x5A), (12, 0x66),
    (12, 0x67),
]
BLACK_MAKEUP = [
    (10, 0x0F), (12, 0xC8), (12, 0xC9), (12, 0x5B), (12, 0x33),
    (12, 0x34), (12, 0x35), (13, 0x6C), (13, 0x6D), (13, 0x4A),
    (13, 0x4B), (13, 0x4C), (13, 0x4D), (13, 0x72), (13, 0x73),
    (13, 0x74), (13, 0x75), (13, 0x76), (13, 0x77), (13, 0x52),
    (13, 0x53), (13, 0x54), (13, 0x55), (13, 0x5A), (13, 0x5B),
    (13, 0x64), (13, 0x65),
    (11, 0x08), (11, 0x0C), (11, 0x0D), (12, 0x12), (12, 0x13),
    (12, 0x14), (12, 0x15), (12, 0x16), (12, 0x17), (12, 0x1C),
    (12, 0x1D), (12, 0x1E), (12, 0x1F),
]

# 2D mode codes in DISPATCH order: V_L3, V_L2, V_L1, V0, V_R1, V_R2, V_R3,
# Horizontal, Pass. The lookup payload's index maps directly via DISPATCH.
TWO_D = [
    (7, 0x02), (6, 0x02), (3, 0x02), (1, 0x01), (3, 0x03),
    (6, 0x03), (7, 0x03), (3, 0x01), (4, 0x01),
]
DISPATCH = [
    ('V', -3), ('V', -2), ('V', -1), ('V', 0), ('V', +1),
    ('V', +2), ('V', +3), ('H', 0), ('P', 0),
]


def _build_lookup(entries, lookup_bits, fill_down=True):
    table = [None] * (1 << lookup_bits)
    for length, code, payload in entries:
        shift = lookup_bits - length
        if shift < 0:
            raise ValueError("code too long")
        base = code << shift
        for i in range(1 << shift):
            table[base | i] = (payload, length)
    if fill_down:
        # Fill empty slots from the previous slot.  Without this any 13-bit
        # prefix that didn't land in a populated range would error -- with
        # it, marginal data still produces a (possibly wrong) decode, which
        # is the right tradeoff for this format.
        for i in range(1, 1 << lookup_bits):
            if table[i] is None:
                table[i] = table[i - 1]
    return table


def _build_run_table(term, makeup):
    e = [(L, c, i) for i, (L, c) in enumerate(term)]
    for i, (L, c) in enumerate(makeup, start=1):
        e.append((L, c, i * 64))
    return _build_lookup(e, 13)


WHITE_TABLE = _build_run_table(WHITE_TERM, WHITE_MAKEUP)
BLACK_TABLE = _build_run_table(BLACK_TERM, BLACK_MAKEUP)
TAB7 = _build_lookup([(L, c, i) for i, (L, c) in enumerate(TWO_D)], 7)


# ---------------------------------------------------------------------------
# Bit reader -- one 32-bit window at a time, refilled in 16-bit halves to
# match the canonical PaperPort 3.6 inner-decoder bit-loading.
# ---------------------------------------------------------------------------

def _refill(bits, bits_left, data, pos, n):
    b0 = data[pos] if pos < n else 0
    b1 = data[pos + 1] if pos + 1 < n else 0
    return (((bits << 16) | (b0 << 8) | b1) & 0xFFFFFFFF, bits_left + 16, pos + 2)


def _refill_lazy(bits, bits_left, data, pos, n, need):
    """11th-session 6th-pass: byte-by-byte refill, matching the canonical
    PaperPort 3.6 inner CCITT decoder's `mov ch, [es:di]; inc di` placement
    at hand-coded refill points. Each call advances `pos` by exactly the
    number of bytes needed to satisfy `need` bits remaining in the buffer.

    Why this matters: the legacy `_refill` always loads 2 bytes per refill,
    so `pos` can be 1 byte ahead of where the canonical's `di` would be at
    a FAIL. The byte-rounding `pos += (consumed + 7) // 8` after the inner
    decoder return then double-counts that pre-loaded byte, producing the
    1-byte over-consume observed at Kaba y=305 (real next marker at 0xc0f,
    decoder lands at 0xc10).
    """
    while bits_left < need and pos < n:
        b = data[pos]
        bits = ((bits << 8) | b) & 0xFFFFFFFF
        bits_left += 8
        pos += 1
    return bits, bits_left, pos


def _decomp_line(data, start_pos, width, table_prev, lazy=False, bug4=True):
    """Decode one CCITT-T.6 line starting at byte boundary `start_pos`.

    Returns (changing_elements_table, bits_consumed).
    On a mid-line decode failure we return an "all-white" table rather than
    smearing partial transitions across the page.

    If `lazy` is True (11th-session 6th-pass), use byte-by-byte refill that
    matches the canonical PaperPort 3.6 inner decoder's bit-loading
    timing. Reduces the 1-byte FAIL over-consume on Kaba-class files where
    the legacy 2-byte eager refill puts pos one byte past canonical's di.

    If `bug4` is True (12th-session continuation), mirror canonical's `si`
    advance per code instead of our existing "tp_idx -= 1 + scan-forward"
    scheme. Canonical advances `si` by +1 idx per V code's lodsw (plus
    optional b2-skip for V_R{1,2,3}), +2 idx per P (add si,2 + lodsw),
    and walks forward past consumed entries for H. Our existing scheme
    is equivalent in most cases but diverges after consecutive V_-N codes
    where a0 advances less than ref entries are consumed (e.g.,
    Mietvertrag y=453 c32: canonical reads ref[36]=646; our existing
    scheme reads ref[34]=644 because scan didn't fire).
    """
    n = len(data)
    pos = start_pos
    if lazy:
        # Lazy init: load no bytes until first read demands them.
        bits = 0
        bits_left = 0
    else:
        bits = (data[pos] << 8 if pos < n else 0) | (data[pos+1] if pos + 1 < n else 0)
        pos += 2
        bits_left = 16

    out = [-1]
    tp_idx = 1
    colour = 0
    x = 0  # canonical seg2:0xD68 zeros ax (a0 starts at 0, not -1)
    safety = 0
    first_iter = True
    while x < width:
        safety += 1
        if safety > width * 4 + 100:
            return [-1, width, width, width], (pos - start_pos) * 8 - bits_left
        if not bug4:
            # Existing scheme: scan-forward at iteration start.
            # Skip the scan-forward on the first iteration: canonical's lodsw
            # reads table_prev[1] directly. If we scan-forward when x=0 happens to
            # equal table_prev[1] (e.g., black starts at column 0), we'd skip past
            # the valid first b1.
            if not first_iter:
                while table_prev[tp_idx] <= x:
                    tp_idx += 2
        first_iter = False

        if lazy:
            if bits_left < 7:
                bits, bits_left, pos = _refill_lazy(bits, bits_left, data, pos, n, 7)
        else:
            if bits_left <= 16:
                bits, bits_left, pos = _refill(bits, bits_left, data, pos, n)

        top7 = (bits >> (bits_left - 7)) & 0x7F
        entry = TAB7[top7]
        if entry is None:
            return [-1, width, width, width], (pos - start_pos) * 8 - bits_left
        idx, length = entry
        bits_left -= length
        kind, voff = DISPATCH[idx]

        if kind == 'H':
            for _ in range(2):
                while True:
                    if lazy:
                        if bits_left < 13:
                            bits, bits_left, pos = _refill_lazy(bits, bits_left, data, pos, n, 13)
                    else:
                        if bits_left < 16:
                            bits, bits_left, pos = _refill(bits, bits_left, data, pos, n)
                    top13 = (bits >> (bits_left - 13)) & 0x1FFF
                    tab = WHITE_TABLE if colour == 0 else BLACK_TABLE
                    entry = tab[top13]
                    if entry is None:
                        return [-1, width, width, width], (pos - start_pos) * 8 - bits_left
                    run, code_len = entry
                    bits_left -= code_len
                    x += run
                    if run <= 63:
                        break
                out.append(x)
                colour ^= 1
            if bug4:
                # H walk-forward (canonical seg2:0x154D): si += 2 idx
                # while ref[si] <= a2.
                while tp_idx < len(table_prev) and table_prev[tp_idx] <= x:
                    tp_idx += 2
        elif kind == 'P':
            x = table_prev[tp_idx + 1]
            if bug4:
                # Canonical P (seg2:0xDCA): add si, 2; lodsw → si advances
                # by 4 bytes = +2 idx.
                tp_idx += 2
        else:  # vertical
            if bug4:
                b1 = table_prev[tp_idx]
                x = b1 + voff
                out.append(x)
                # Canonical lodsw: si += 1 idx.
                tp_idx += 1
                # b2-skip for V_R1, V_R2 (1 step), V_R3 (2 steps).
                if voff > 0 and x < width:
                    max_skips = 2 if voff == 3 else 1
                    for _ in range(max_skips):
                        if (tp_idx < len(table_prev)
                                and x >= table_prev[tp_idx]):
                            tp_idx += 2
                        else:
                            break
                if x < width:
                    colour ^= 1
            else:
                x = table_prev[tp_idx] + voff
                out.append(x)
                if x < width:
                    tp_idx -= 1
                    colour ^= 1

    out.append(width)
    out.append(width)
    return out, (pos - start_pos) * 8 - bits_left


def _table_to_row(table, width, row_bytes):
    out = bytearray(row_bytes)
    i = 1
    n = len(table)
    while i + 1 < n:
        start = max(table[i], 0)
        end = table[i + 1]
        if start >= width:
            break
        if end > width:
            end = width
        if end > start:
            sb = start >> 3
            eb = (end - 1) >> 3
            if sb == eb:
                lo = start & 7
                hi = (end & 7) or 8
                out[sb] |= (((0xFF >> lo) & ((0xFF << (8 - hi)) & 0xFF))) & 0xFF
            else:
                out[sb] |= (0xFF >> (start & 7)) & 0xFF
                for b in range(sb + 1, eb):
                    out[b] = 0xFF
                rem = end & 7
                out[eb] = 0xFF if rem == 0 else (out[eb] | ((0xFF << (8 - rem)) & 0xFF))
        i += 2
    return bytes(out)


def _table_from_raw(row, width):
    """Build a transition table from a raw 1-bit row (MSB-first, 1=black)."""
    out = [-1]
    colour = 0
    for x in range(width):
        bit = (row[x >> 3] >> (7 - (x & 7))) & 1
        if bit != colour:
            out.append(x)
            colour ^= 1
    out.extend([width, width])
    return out


# ---------------------------------------------------------------------------
# Image-chunk decoder.  PaperPort 2 stores each line's payload byte-aligned,
# preceded by a 1-byte marker:
#       top 2 bits  = type
#       low 6 bits  = blank-count (type=3) or single-mode position-count
#                     (type=1); padding (zero) for types 0 and 2.
# Types: 0 = uncompressed raw bitmap, 1 = single-pixel positions,
#        2 = CCITT-T.6 compressed line, 3 = blank-line run.
# ---------------------------------------------------------------------------

def _resync_probe(data, start_pos, table_prev, width, line_bytes, n_steps):
    """Lookahead probe: from start_pos, walk dispatch for up to n_steps lines.
    Return (n_ok, n_drift). n_ok counts successful type-2 OK decodes; n_drift
    counts FAIL/V0/BAD/T1 events. Used by `fail_resync_max` to score candidate
    resync offsets after a type-2 FAIL. Self-contained: does not write output.
    """
    n = len(data)
    sentinel = [-1] + [width] * (width + 16)
    tp = list(table_prev)
    p = start_pos
    n_ok = 0
    n_drift = 0
    steps = 0
    while steps < n_steps and p < n:
        marker = data[p]
        type_ = (marker >> 6) & 3
        low6 = marker & 0x3F
        if type_ == 0:
            if p + 1 + line_bytes > n:
                break
            row = data[p+1:p+1+line_bytes]
            if len(row) < line_bytes:
                row = bytes(row) + bytes(line_bytes - len(row))
            tp = _table_from_raw(row, width) + [width] * 16
            p += 1 + line_bytes
            n_drift += 1
            steps += 1
        elif type_ == 1:
            p += 1  # treat as suppressed (ship default)
            continue
        elif type_ == 2:
            table, consumed = _decomp_line(data, p+1, width, tp)
            adv = (consumed + 7) // 8
            looks_fb = (len(table) == 4 and table == [-1, width, width, width])
            is_real_fail = looks_fb and consumed != 1
            looks_v0 = looks_fb and consumed == 1
            tail = table[1:]
            x_final = tail[-3] if len(tail) >= 3 else (tail[-1] if tail else None)
            valid_x = (x_final is None or x_final <= width + 3)
            is_bad = (not looks_fb) and (not valid_x)
            if is_real_fail or looks_v0 or is_bad:
                n_drift += 1
            else:
                n_ok += 1
                tp = [-1] + list(table[1:]) + [width] * 16
            p += 1 + adv
            steps += 1
        else:  # type 3
            tp = list(sentinel)
            p += 1
            steps += 1  # count one BLANK marker as one step
    return n_ok, n_drift


def decode_image_chunk(data, chunk_start, t0_reset=False, t0_blank=False,
                       t0_drop_after_drift='', t0_drop_kinds=None,
                       drop_blank_after_drift=False, drop_blank_kinds=None,
                       reset_ref_after_drift=False, suppress_t1_bad=False,
                       suppress_t1_all=False, suppress_t2_nonzero_low6=False,
                       fail_scan_forward_max=0,
                       suppress_t2_fail_y_in_cascade=False,
                       fail_resync_max=0, fail_resync_lookahead=5,
                       fail_resync_budget=0, fail_resync_min_confidence=0,
                       fail_resync_clean_window=0, fail_resync_clean_min_ok=0,
                       fail_resync_zero_max=None,
                       fail_resync_clean_marker_bonus=0,
                       strict_t0=True,
                       lazy_bit_loading=False,
                       bug4=True):
    """Decode a single image chunk to a 1-bit raster.

    t0_reset: if True, after a type-0 ("uncompressed") dispatch, reset the
        reference table to all-white instead of building it from the
        purportedly-raw 308 bytes. The 4th-session investigation showed the
        308 bytes are NOT a real bitmap (entropy/density signature is
        compressed-stream), so using them as a reference corrupts the next
        compressed line's decode. Helps File 2 (FAIL: 22->0) and File 3
        (FAIL: 60->4) at the cost of File 1's form-field decodes that
        currently rely on the misinterpreted reference.

    t0_blank: if True, type-0 dispatches consume the same number of bytes
        (preserving stride) but emit an all-white row and reset the
        reference (implies t0_reset). 5th-session corpus comparison
        showed dec_black/gt_black ratio of ~28x — the type-0 308-byte
        "raw bitmap" is actually compressed-noise emitted as ~50%-density
        garbage, drowning real content. This variant tests whether
        suppressing that noise raises corpus-wide IoU.

    t0_drop_after_drift: '' (off, default), 'marker' (drop 1-byte
        marker only), or 'full' (drop marker + 308-byte payload).
        When set and `last_kind` is in the t0-drift-kinds set,
        suppress the T0 dispatch entirely. Mirrors
        drop_blank_after_drift's pattern. Driven by H1 finding
        (96.5% drift-prev fraction in oracle CSV mining, 2026-05-08).

    t0_drop_kinds: optional iterable of last_kind values that
        trigger t0_drop_after_drift. None (default) inherits the
        drop_blank_kinds set ({V0,FAIL,BAD,T1,T0}); the 7th-session
        position-asymmetry result (last-of-FAIL-cascade rows are
        6.8% accidentally correct vs 34.2% middle) motivates testing
        a tighter set, e.g. {'FAIL'} or {'FAIL','V0'}.

    fail_scan_forward_max: bytes (0 disables, default). When >0,
        after a real type-2 FAIL the decoder scans forward for the
        next occurrence of the byte sequence b'\\x80\\xf8' within
        the given window and resets `pos` there. Driven by oracle
        H4 finding (`(0x80, 0xf8)` byte pair has 30.9% encoder-
        oracle match rate at FAIL positions, vs 0.3% corpus avg —
        100x baseline lift). The hope is that this byte sequence
        marks a real next-line resync point.

    suppress_t2_fail_y_in_cascade: when True and a type-2 marker
        FAILs decoding AND the kind from the immediately preceding
        dispatch (captured before this dispatch updates last_kind)
        is in the drift_kinds set, consume the bytes but do NOT
        advance y, do NOT emit a row, do NOT update last_kind, do
        NOT touch table_prev. Lens-driven (8th session): type-2
        FAILs in cascade are the largest unfiltered y-over-advance
        source. Skipping the y advance preserves alignment for
        downstream real content.

    strict_t0: when True (default, 11th-session RE-driven), follow
        the canonical PaperPort 3.6 reader's type-0 dispatch rule
        from MAXKER2.DLL seg2:0xCC8/0xD2E. Only low6==1 triggers
        raw-copy; only low6==3 triggers skip-line (consume marker +
        line_bytes input, no y advance, no output write, table_prev
        unchanged). All other type-0 markers are consumed as a
        single stray byte (canonical aborts with error -2; we
        approximate that as drop-and-continue). When False, falls
        back to the pre-RE behaviour of treating every type-0 byte
        as a 308-byte raw-copy, which is what the corpus baseline
        (`corpus_results.csv`) used. Disable with `--no-strict-t0`.

    lazy_bit_loading: when True (11th-session 6th-pass, opt-in),
        use byte-by-byte refill in `_decomp_line` matching the
        canonical PaperPort 3.6 inner decoder's bit-load timing.
        The legacy eager refill loads 2 bytes per call; on FAIL,
        pos may be 1 byte ahead of where canonical's di would be,
        causing the 1-byte over-consume observed at Kaba y=305
        (real next marker 0xc0f, decoder lands at 0xc10). Lazy
        refill loads exactly enough bytes to satisfy the next
        read; on FAIL, pos matches canonical's di. Enable with
        `--lazy-bit-loading`.
    """
    width  = struct.unpack_from('<H', data, chunk_start + 0x26)[0]
    height = struct.unpack_from('<H', data, chunk_start + 0x28)[0]
    dpi_x  = struct.unpack_from('<H', data, chunk_start + 0x2a)[0]
    dpi_y  = struct.unpack_from('<H', data, chunk_start + 0x2c)[0]
    bpp    = struct.unpack_from('<H', data, chunk_start + 0x2e)[0]
    if bpp != 1:
        raise NotImplementedError(f"only 1-bit images supported; got {bpp} bpp")
    img_start = chunk_start + 0x42

    line_bytes = (width + 7) // 8
    row_bytes  = (line_bytes + 3) & ~3
    sentinel   = [-1] + [width] * (width + 16)
    table_prev = list(sentinel)

    out = bytearray()
    blank = bytes(row_bytes)
    pos = img_start
    n = len(data)
    y = 0
    last_kind = None  # 'OK', 'V0', 'FAIL', 'BAD', 'T0', 'T1', 'T1ok', 'BLANK'
    # 10th-session 2nd pass: per-FAIL local-context gate. Track last K
    # dispatches; only allow resync if at least M of the last K were OK.
    # Captures "isolated FAIL in a clean region" more precisely than the
    # 1-step prev_kind check. fail_resync_clean_window=0 disables.
    recent_kinds = []  # FIFO of last K kinds (window for clean-region gate)
    # Optimal drift_kinds set per 6th-session corpus scoring (134-pair GT corpus,
    # `compare_to_groundtruth.py --inventory`). Median IoU 1.95% -> 2.97%, mean
    # 3.11% -> 3.75%, dec/gt black ratio 0.28x -> 0.53x. Adding 'OK' (= drop
    # always-except-after-BLANK) keeps median but lowers mean; dropping in BLANK
    # runs too tanks everything (real long blank stretches need consecutive
    # BLANK markers).
    if drop_blank_kinds is None:
        drift_kinds = {'V0', 'FAIL', 'BAD', 'T1', 'T0'}
    else:
        drift_kinds = set(drop_blank_kinds)
    if t0_drop_kinds is None:
        t0_drift_kinds = drift_kinds
    else:
        t0_drift_kinds = set(t0_drop_kinds)
    # 10th-session: budget on number of resync attempts per page.
    # 0 = unlimited (try on every isolated FAIL), N>0 = stop after N attempts.
    # Driven by hypothesis: first FAIL is the most likely single-event
    # opportunity; later FAILs are usually cascade artifacts where resync
    # finds spurious markers.
    resync_attempts_remaining = fail_resync_budget if fail_resync_budget else float('inf')
    while y < height:
        if pos >= n:
            break
        marker = data[pos]
        type_ = (marker >> 6) & 3
        low6 = marker & 0x3F

        if type_ == 0:                # uncompressed raw / skip / invalid
            # Canonical PaperPort 3.6 reader (MAXKER2.DLL seg2:0xCC8 +
            # 0xD2E) only accepts low6==1 (raw-copy) and low6==3
            # (skip-line); every other type-0 byte returns error -2
            # and aborts decode. With strict_t0 (default), match that
            # gate: drop bad type-0 markers as 1-byte stray noise.
            if strict_t0 and low6 not in (1, 3):
                pos += 1
                continue  # leave last_kind / table_prev / y untouched
            if low6 == 3:
                # canonical "skip" dispatch (seg2:0xD32-0xD38):
                #   add di, [bp+0x1c]   ; advance INPUT by line_bytes
                #   inc word [bp+0x4]   ; un-decrement loop counter
                #   jmp 0xbe2
                # Net effect: consume marker + line_bytes from input,
                # produce no output for this iteration, do not count
                # this as one of the strip's lines. We translate the
                # "doesn't count as a line" semantics by NOT advancing
                # y; table_prev stays as-is.
                pos += 1 + line_bytes
                # last_kind unchanged so chained drift handling still works
                continue
            # low6 == 1 (canonical raw-copy) -- or strict_t0 disabled,
            # in which case we keep the legacy behaviour where any
            # type-0 byte triggers a 308-byte raw-copy.
            if t0_drop_after_drift and last_kind in t0_drift_kinds:
                # H1 finding: 96.5% of T0 dispatches are sync-drift
                # artefacts. When prev was drift-like, drop this T0.
                if t0_drop_after_drift == 'marker':
                    pos += 1
                else:  # 'full'
                    pos += 1 + line_bytes
                continue  # leave last_kind unchanged
            pos += 1
            row = data[pos:pos + line_bytes]
            if len(row) < line_bytes:
                row = row + bytes(line_bytes - len(row))
            row += bytes(row_bytes - line_bytes)
            if t0_blank:
                out += blank
                table_prev = list(sentinel)
            elif t0_reset:
                out += row
                table_prev = list(sentinel)
            else:
                out += row
                # Add 16 trailing width sentinels to match the post-OK
                # pattern below — _decomp_line can walk tp_idx past
                # the end on rows with many transitions. 9th-session
                # orphan validation found this crashes 3/111 orphans
                # (e.g. 2000_02_17 Offerte Schibli.MAX) with
                # "list index out of range" in _decomp_line.
                table_prev = _table_from_raw(row, width) + [width] * 16
            pos += line_bytes
            y += 1
            last_kind = 'T0'
        elif type_ == 1:              # single-pixel positions; low6 = count
            # Peek validity before committing pos
            valid = True
            p = pos + 1
            positions = [-1]
            for _ in range(low6):
                if p + 1 >= n:
                    break
                ch = (data[p] << 8) | data[p + 1]
                if ch > width + 16:
                    valid = False
                positions.append(ch)
                p += 2
            if suppress_t1_all or (suppress_t1_bad and not valid):
                # Treat as a stray byte: consume marker only, no y advance,
                # don't touch reference. Across the corpus, type-1 dispatches
                # are 99% sync-drift (4360 T1bad vs 46 T1ok in 60 files); the
                # T1ok cases are likely coincidentally-valid drift too.
                pos += 1
                continue
            pos = p
            positions.extend([width, width])
            row = _table_to_row(positions, width, row_bytes)
            out += row
            table_prev = [-1] + positions[1:] + [width] * 16
            y += 1
            last_kind = 'T1' if not valid else 'T1ok'
        elif type_ == 2:              # compressed CCITT-T.6 line
            if suppress_t2_nonzero_low6 and low6 != 0:
                # Real type-2 markers in ViGB are always 0x80 (low6=0).
                # Markers like 0x81..0xBF are 99% sync-drift bytes that
                # happen to have top-2-bits=10. Consume the marker only.
                pos += 1
                continue
            prev_kind = last_kind
            pos += 1
            table, consumed = _decomp_line(data, pos, width, table_prev, lazy=lazy_bit_loading, bug4=bug4)
            pos += (consumed + 7) // 8
            is_fallback = (len(table) == 4 and table == [-1, width, width, width])
            looks_v0 = is_fallback and consumed == 1
            tail = table[1:]
            x_final = tail[-3] if len(tail) >= 3 else (tail[-1] if tail else None)
            valid_x = (x_final is None or x_final <= width + 3)
            is_real_fail = is_fallback and consumed != 1
            is_bad = (not is_fallback) and (not valid_x)
            if is_real_fail and suppress_t2_fail_y_in_cascade and prev_kind in drift_kinds:
                # V6 (8th session, lens-driven): type-2 FAIL in cascade
                # is overwhelmingly drift. Consume the bytes (already done
                # via pos += adv above) but skip y advance, no row emit,
                # leave last_kind and table_prev untouched.
                continue
            out += _table_to_row(table, width, row_bytes)
            if is_real_fail:
                last_kind = 'FAIL'
            elif looks_v0:
                last_kind = 'V0'
            elif is_bad:
                last_kind = 'BAD'
            else:
                last_kind = 'OK'
            if reset_ref_after_drift and last_kind in {'BAD', 'FAIL', 'V0'}:
                table_prev = list(sentinel)
            elif not is_real_fail:
                table_prev = [-1] + table[1:] + [width] * 16
            y += 1
            # Update recent_kinds window before any potential resync gate.
            # Use the just-set last_kind from this dispatch.
            if fail_resync_clean_window:
                recent_kinds.append(last_kind)
                if len(recent_kinds) > fail_resync_clean_window:
                    recent_kinds.pop(0)
            # Local-context gate: count OK in the recent window EXCLUDING
            # the FAIL we just recorded. We want to know what was happening
            # BEFORE this FAIL.
            local_ok_before = sum(1 for k in recent_kinds[:-1] if k == 'OK') if fail_resync_clean_window else 0
            if (fail_resync_max and is_real_fail
                    and prev_kind not in {'FAIL', 'V0', 'BAD', 'T1'}
                    and resync_attempts_remaining > 0
                    and (not fail_resync_clean_window
                         or local_ok_before >= fail_resync_clean_min_ok)):
                # 10th-session smart resync: after type-2 FAIL, the decoder may
                # have over- or under-consumed by a few bytes. Probe offsets
                # [-K, +K] from the naive next position; for each, run a short
                # lookahead decode and score by (n_ok - n_drift). Pick the
                # offset with the highest score, tie-break by smallest |offset|.
                # Conditional: only apply on ISOLATED FAILs (prev_kind=OK or
                # BLANK or T0). Dense-file cascades aren't fixable this way.
                naive = pos
                # Use the *current* table_prev as the lookahead reference.
                ref_for_probe = list(table_prev)
                best_off = 0
                best_score = -10**9
                score_at_zero = 0
                for off in range(-fail_resync_max, fail_resync_max + 1):
                    cand = naive + off
                    if cand <= chunk_start + 0x42 or cand >= n - 1:
                        continue
                    n_ok_p, n_drift_p = _resync_probe(
                        data, cand, list(ref_for_probe), width, line_bytes,
                        fail_resync_lookahead)
                    score = n_ok_p - n_drift_p
                    # Bonus for landing on a "clean" type-2 marker (0x80)
                    if (fail_resync_clean_marker_bonus
                            and 0 <= cand < n
                            and data[cand] == 0x80):
                        score += fail_resync_clean_marker_bonus
                    if off == 0:
                        score_at_zero = score
                    if score > best_score or (score == best_score and abs(off) < abs(best_off)):
                        best_score = score
                        best_off = off
                # New gate: only commit resync if the no-resync (offset=0)
                # decode would clearly fail (score_at_zero <= zero_max).
                # If offset=0 already produces successful decodes, the
                # decoder is on track without resync — leave it alone.
                if (fail_resync_zero_max is not None
                        and score_at_zero > fail_resync_zero_max):
                    best_off = 0
                # Confidence gate: only commit resync if best score
                # exceeds threshold. Prevents low-confidence resyncs that
                # find spurious markers in cascade-style files.
                if (best_off != 0 and best_score >= fail_resync_min_confidence):
                    pos = naive + best_off
                    table_prev = list(sentinel)
                resync_attempts_remaining -= 1
            elif fail_scan_forward_max and is_real_fail:
                # H4: 0x80 0xf8 marker+first-payload pair has 30.9% oracle
                # match rate at FAIL positions (vs 0.3% corpus avg). After
                # a FAIL, scan forward for the next 0x80 0xf8 sequence within
                # a bounded window and resync there.
                scan_end = min(pos + fail_scan_forward_max, n - 1)
                sp = pos
                while sp < scan_end:
                    if data[sp] == 0x80 and data[sp + 1] == 0xf8:
                        if sp != pos:
                            pos = sp
                        break
                    sp += 1
        else:                         # type 3 -- blank-line run; low6 = count
            if drop_blank_after_drift and last_kind in drift_kinds:
                # Suspect this BLANK is sync-drift: consume the byte but don't
                # advance y or reset reference. The next byte gets a fresh dispatch.
                pos += 1
                # leave last_kind unchanged so chained drift bytes also drop
                continue
            count = low6 + 1  # canonical: BLANK marker advances y by low6+1
            table_prev = list(sentinel)
            for _ in range(count):
                out += blank
                y += 1
                if y >= height:
                    break
            pos += 1
            last_kind = 'BLANK'

    # pad if we ran off the end early
    while y < height:
        out += blank
        y += 1

    return {
        'width': width,
        'height': height,
        'dpi_x': dpi_x or 300,
        'dpi_y': dpi_y or 300,
        'row_bytes': row_bytes,
        'raw': bytes(out),
    }


# ---------------------------------------------------------------------------
# .max container -- find the image chunks (DL-tagged with the image flag).
# ---------------------------------------------------------------------------

def find_image_chunks(data):
    chunks = []
    pos = 0
    n = len(data)
    while pos < n - 8:
        if data[pos:pos + 2] == b'DL':
            length = struct.unpack_from('<I', data, pos + 2)[0]
            flags  = struct.unpack_from('<I', data, pos + 6)[0]
            if (flags & 0xFFFF) == 0x4000 and (flags >> 16) > 0 and 0 < length <= n - pos:
                chunks.append((pos, length))
                pos += length
                continue
        pos += 1
    return chunks


# ---------------------------------------------------------------------------
# Preview thumbnail decoder.  Each image chunk has a 102x146 grayscale
# thumbnail (sometimes 105x147) appended to the end of the chunk, encoded
# with a byte-level RLE (top 2 bits = type, low 6 = count):
#
#   type 0: emit count*4 zero pixels (white background after inversion)
#   type 1: emit count*4 0xFF pixels (black after inversion)
#   type 2: read `count` literal bytes; 4 grayscale pixels per byte
#           ((byte >> j) & 3) * 85 with j in (6, 4, 2, 0)
#   type 3: unknown -- paperman bails on this; we skip with no output
#
# Preview lives at chunk_start + chunk_length - preview_size.
# Output is inverted (so 0=white background) and vertically flipped.
# ---------------------------------------------------------------------------

def _decode_preview_rle(buf, total_pixels, max_bytes):
    """Return (pixels_bytes, type3_count).  pixels_bytes are 8bpp grayscale."""
    out = bytearray()
    pos = 0
    type3 = 0
    end = min(max_bytes, len(buf))
    while pos < end and len(out) < total_pixels:
        ch = buf[pos]; pos += 1
        type_ = ch >> 6
        count = ch & 0x3F
        if type_ == 0:
            out += bytes(count * 4)
        elif type_ == 1:
            out += bytes([0xFF] * (count * 4))
        elif type_ == 2:
            for _ in range(count):
                if pos >= end: break
                cb = buf[pos]; pos += 1
                for j in (6, 4, 2, 0):
                    out.append(((cb >> j) & 3) * 85)
        else:
            type3 += 1
    return bytes(out[:total_pixels]), type3


def decode_preview_chunk(data, chunk_start, chunk_length, scale_to_a4=True):
    """Decode the preview thumbnail and return a 1-bit raster suitable
    for the PDF writer. If scale_to_a4, upscale to the same page
    dimensions as the main image so it fills a comparable PDF page."""
    preview_size = struct.unpack_from('<H', data, chunk_start + 0x3c)[0]
    preview_x    = struct.unpack_from('<H', data, chunk_start + 0x3e)[0]
    preview_y    = struct.unpack_from('<H', data, chunk_start + 0x40)[0]
    if preview_size == 0 or preview_x == 0 or preview_y == 0:
        return None
    main_dpi   = struct.unpack_from('<H', data, chunk_start + 0x2a)[0] or 300
    main_w     = struct.unpack_from('<H', data, chunk_start + 0x26)[0]
    main_h     = struct.unpack_from('<H', data, chunk_start + 0x28)[0]

    padded_x = (preview_x + 3) & ~3
    target_pixels = padded_x * preview_y
    offset = chunk_start + chunk_length - preview_size
    pixels, _type3 = _decode_preview_rle(
        data[offset : chunk_start + chunk_length], target_pixels, preview_size)
    # Pad if short (type-3 or truncated input)
    if len(pixels) < target_pixels:
        pixels = pixels + bytes([128] * (target_pixels - len(pixels)))

    # The raw RLE grayscale is already in the right polarity for our 1-bit
    # PDF convention: type-0 emits 0x00 (= white background, bit=0 in PDF),
    # type-1 emits 0xFF (= text/black, bit=1).
    # Vertical flip (paperman flips for display).
    rows = [pixels[i*padded_x:(i+1)*padded_x] for i in range(preview_y)]
    rows.reverse()
    flipped = b''.join(rows)

    if scale_to_a4:
        # Upscale to same dimensions as main image, nearest neighbor.
        # Convert to 1-bit by thresholding at 128.
        target_w = main_w
        target_h = main_h
    else:
        target_w = preview_x
        target_h = preview_y

    # Threshold -> 1-bit, then nearest-neighbor upscale into row_bytes-aligned raster.
    # PIL is ~430x faster than the per-pixel Python loop (Mietvertrag: 862 ms → 2 ms).
    line_bytes = (target_w + 7) // 8
    row_bytes  = (line_bytes + 3) & ~3
    try:
        from PIL import Image
        src = Image.frombytes('L', (padded_x, preview_y), flipped)
        if (padded_x, preview_y) != (target_w, target_h):
            src = src.resize((target_w, target_h), Image.NEAREST)
        # Threshold to '1' mode. Source `flipped` has 0xFF = foreground
        # (the 'after inversion' convention from _decode_preview_rle), and
        # the decoder's raw-byte convention sets bit=1 for foreground.
        # PIL '1' mode tobytes() packs bit=1 where pixel is non-zero,
        # which lines up: source 0xFF → point→255 → PIL bit=1 = our bit=1.
        bw = src.point(lambda v: 255 if v >= 128 else 0, mode='1')
        packed = bw.tobytes()
        if line_bytes == row_bytes:
            out = bytearray(packed)
        else:
            out = bytearray()
            for y in range(target_h):
                row = packed[y*line_bytes:(y+1)*line_bytes]
                out += row + bytes(row_bytes - line_bytes)
    except ImportError:
        out = bytearray()
        for y in range(target_h):
            sy = (y * preview_y) // target_h
            src_row = flipped[sy*padded_x : sy*padded_x + preview_x]
            line = bytearray(row_bytes)
            for x in range(target_w):
                sx = (x * preview_x) // target_w
                if sx < len(src_row) and src_row[sx] >= 128:
                    line[x >> 3] |= 0x80 >> (x & 7)
            out += bytes(line)

    return {
        'width': target_w,
        'height': target_h,
        'dpi_x': main_dpi,
        'dpi_y': main_dpi,
        'row_bytes': row_bytes,
        'raw': bytes(out),
    }


def parse_max(path, t0_reset=False, t0_drop_after_drift='', t0_drop_kinds=None,
              include_preview=True, drop_blank_after_drift=True,
              suppress_t1_all=True, fail_scan_forward_max=0,
              suppress_t2_fail_y_in_cascade=False,
              fail_resync_max=0, fail_resync_lookahead=5,
              fail_resync_budget=0, fail_resync_min_confidence=0,
              reset_ref_after_drift=False, strict_t0=True,
              lazy_bit_loading=False, bug4=True):
    data = path.read_bytes()
    if data[:5] != b'ViGBe':
        raise ValueError(f"{path}: not a PaperPort 2 file (magic={data[:5]!r})")
    chunks = find_image_chunks(data)
    if not chunks:
        raise ValueError(f"{path}: no image chunks found")
    pages = []
    for chunk_start, chunk_length in chunks:
        pages.append(decode_image_chunk(data, chunk_start, t0_reset=t0_reset,
                                         t0_drop_after_drift=t0_drop_after_drift,
                                         t0_drop_kinds=t0_drop_kinds,
                                         drop_blank_after_drift=drop_blank_after_drift,
                                         suppress_t1_all=suppress_t1_all,
                                         fail_scan_forward_max=fail_scan_forward_max,
                                         suppress_t2_fail_y_in_cascade=suppress_t2_fail_y_in_cascade,
                                         fail_resync_max=fail_resync_max,
                                         fail_resync_lookahead=fail_resync_lookahead,
                                         fail_resync_budget=fail_resync_budget,
                                         fail_resync_min_confidence=fail_resync_min_confidence,
                                         reset_ref_after_drift=reset_ref_after_drift,
                                         strict_t0=strict_t0,
                                         lazy_bit_loading=lazy_bit_loading,
                                         bug4=bug4))
        if include_preview:
            preview_page = decode_preview_chunk(data, chunk_start, chunk_length)
            if preview_page is not None:
                pages.append(preview_page)
    return pages


# ---------------------------------------------------------------------------
# Minimal PDF writer.  Each page becomes a 1-bit FlateDecode image XObject
# wrapped in a page object scaled to the original DPI.
# ---------------------------------------------------------------------------

def write_pdf(pages, out_path):
    objects = [b'']  # 1-based

    def add(obj_bytes):
        objects.append(obj_bytes)
        return len(objects) - 1

    palette = bytes([0xFF, 0x00])  # 0=white, 1=black
    page_ids = []
    for p in pages:
        # The decoded raw bitmap uses 1=black, but PDF /Indexed [0=white,1=black]
        # also uses 1=black, so no inversion needed here.
        compressed = zlib.compress(p['raw'])
        # The image data is `row_bytes` wide per row (a multiple of 4 bytes).
        stored_width = p['row_bytes'] * 8
        img_dict = (
            b'<< /Type /XObject /Subtype /Image '
            b'/Width %d /Height %d /BitsPerComponent 1 '
            b'/ColorSpace [/Indexed /DeviceGray 1 <%s>] '
            b'/Filter /FlateDecode /Length %d >>\nstream\n'
            % (stored_width, p['height'], palette.hex().encode(), len(compressed))
        )
        img_dict += compressed + b'\nendstream'
        img_id = add(img_dict)

        page_w = p['width'] * 72.0 / p['dpi_x']
        page_h = p['height'] * 72.0 / p['dpi_y']
        # crop the image's painted region to width/height by scaling so the
        # extra padding columns (stored_width - width) hang past the page.
        scale_x = stored_width * 72.0 / p['dpi_x']
        scale_y = page_h
        content = b'q\n%.4f 0 0 %.4f 0 0 cm\n/Im0 Do\nQ\n' % (scale_x, scale_y)
        content_id = add(b'<< /Length %d >>\nstream\n%sendstream' % (len(content), content))

        page_obj = (
            b'<< /Type /Page /Parent 0 0 R /MediaBox [0 0 %.4f %.4f] '
            b'/Contents %d 0 R '
            b'/Resources << /XObject << /Im0 %d 0 R >> /ProcSet [/PDF /ImageB] >> >>'
            % (page_w, page_h, content_id, img_id)
        )
        page_ids.append(add(page_obj))

    pages_id = add(
        b'<< /Type /Pages /Count %d /Kids [%s] >>'
        % (len(page_ids), b' '.join(b'%d 0 R' % pid for pid in page_ids))
    )
    for pid in page_ids:
        objects[pid] = objects[pid].replace(b'/Parent 0 0 R', b'/Parent %d 0 R' % pages_id)
    catalog_id = add(b'<< /Type /Catalog /Pages %d 0 R >>' % pages_id)

    buf = bytearray()
    buf += b'%PDF-1.4\n%\xe2\xe3\xcf\xd3\n'
    offsets = [0] * len(objects)
    for i in range(1, len(objects)):
        offsets[i] = len(buf)
        buf += b'%d 0 obj\n' % i
        buf += objects[i]
        buf += b'\nendobj\n'
    xref_pos = len(buf)
    buf += b'xref\n0 %d\n0000000000 65535 f \n' % len(objects)
    for i in range(1, len(objects)):
        buf += b'%010d 00000 n \n' % offsets[i]
    buf += b'trailer\n<< /Size %d /Root %d 0 R >>\n' % (len(objects), catalog_id)
    buf += b'startxref\n%d\n%%%%EOF\n' % xref_pos

    out_path.write_bytes(bytes(buf))


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main(argv=None):
    ap = argparse.ArgumentParser(description='Convert PaperPort 2 (.max) files to PDF.')
    ap.add_argument('inputs', nargs='+', help='one or more .max files')
    ap.add_argument('-o', '--output-dir', help='write PDFs into this directory '
                    '(default: alongside each input)')
    ap.add_argument('--t0-reset', action='store_true',
                    help='reset reference table to all-white after type-0 dispatches '
                         '(experimental; helps dense-text files at the cost of '
                         'form-field decode quality on sparse files)')
    ap.add_argument('--t0-drop-after-drift',
                    choices=['', 'marker', 'full'], default='',
                    help='Drop drift T0 dispatches (H1 finding). '
                         '"marker"=drop 1-byte marker only, '
                         '"full"=drop marker + 308-byte payload.')
    ap.add_argument('--t0-drop-kinds', default=None,
                    help='Comma list of last_kind values that trigger '
                         '--t0-drop-after-drift (e.g. "FAIL" or "FAIL,V0"). '
                         'Default: same as --drop-blank-kinds '
                         '(V0,FAIL,BAD,T1,T0).')
    ap.add_argument('--fail-scan-forward', type=int, default=0,
                    metavar='N',
                    help='After type-2 FAIL, scan forward up to N bytes '
                         'for next 0x80 0xf8 byte pair and resync there '
                         '(oracle H4: 100x baseline lift). 0 disables.')
    ap.add_argument('--suppress-t2-fail-y-in-cascade', action='store_true',
                    help='When type-2 FAIL occurs and the previous dispatch '
                         'kind is in the drift set, consume bytes but do not '
                         'advance y (lens-driven: y-alignment preservation).')
    ap.add_argument('--fail-resync-max', type=int, default=0, metavar='K',
                    help='10th-session smart resync. After isolated type-2 '
                         'FAILs, probe byte offsets [-K, +K] from the naive '
                         'next position and pick the offset with most '
                         'subsequent OK decodes. 0 disables. K=4 is the '
                         'tested setting. Combine with '
                         '--reset-ref-after-drift and '
                         '--fail-resync-min-confidence=2 for the best '
                         'tested config. Net corpus IoU effect is ~neutral '
                         '(50/50 wins/losses) but specific files can lift '
                         'multiple percentage points (e.g. Kaba 31->35%%, '
                         'Postenauszug 2005-01-18 3.6->9.7%%). Some other '
                         'files regress (e.g. Schreiben Softwork -3.3%%). '
                         'Try this on a single file and visually inspect.')
    ap.add_argument('--fail-resync-lookahead', type=int, default=5, metavar='M',
                    help='Number of lines for the resync lookahead probe '
                         '(default 5; 5 was best-tested).')
    ap.add_argument('--fail-resync-min-confidence', type=int, default=0,
                    metavar='C',
                    help='Resync confidence threshold: only commit a '
                         'resync if the lookahead score (n_ok - n_drift) '
                         'is >= C. 0 disables the gate (always commit '
                         'highest-scoring offset). C=2 reduces the '
                         'magnitude of regressions on cascade-style files '
                         'while preserving most wins.')
    ap.add_argument('--fail-resync-budget', type=int, default=0, metavar='B',
                    help='Maximum number of resync attempts per page. '
                         '0 = unlimited. B>0 stops after B attempts '
                         '(useful if late-page resyncs hurt more than '
                         'help; not generally beneficial in testing).')
    ap.add_argument('--reset-ref-after-drift', action='store_true',
                    help='After any drift dispatch (FAIL/V0/BAD), reset '
                         'the CCITT reference table to all-white. Pairs '
                         'with --fail-resync-max for best effect.')
    ap.add_argument('--no-preview', action='store_true',
                    help='do NOT include the embedded preview thumbnail as an extra '
                         'page in the output PDF')
    ap.add_argument('--keep-drift-blanks', action='store_true',
                    help='disable the 6th-session drop-blank-after-drift heuristic '
                         '(restores pre-fix behaviour: 1490+ rows of false BLANK '
                         'over-dispatch on a typical page)')
    ap.add_argument('--keep-t1-dispatches', action='store_true',
                    help='disable the 6th-session suppress-t1-all heuristic '
                         '(type-1 markers in ViGB are 99%% sync-drift; suppressing '
                         'them recovers another ~5%% of GT ink)')
    ap.add_argument('--no-strict-t0', action='store_true',
                    help='disable the 11th-session strict-T0 dispatch (canonical '
                         'PaperPort 3.6 only accepts type-0 markers with low6==1 '
                         '[raw-copy] or low6==3 [skip]; ours drops other type-0 '
                         'markers as stray bytes). Using --no-strict-t0 restores '
                         'the pre-RE behaviour where every type-0 byte triggers a '
                         '308-byte raw-copy.')
    ap.add_argument('--lazy-bit-loading', action='store_true',
                    help='use byte-by-byte refill matching the canonical PaperPort '
                         '3.6 inner decoder (11th-session 6th-pass). Reduces 1-byte '
                         'FAIL over-consume on Kaba-class files where eager 2-byte '
                         'refill puts pos one byte past canonical di. Opt-in; effect '
                         'is per-file rather than corpus-wide.')
    ap.add_argument('--no-bug4', action='store_true',
                    help='disable canonical reference-table (si) advance (12th-session '
                         "follow-up). Default ON: V codes do tp_idx += 1 (lodsw) plus "
                         'optional b2-skip for V_R{1,2,3}; P advances tp_idx += 2; H '
                         'walks past consumed entries. Closes the cl=8/dec di gap '
                         'and lifts corpus median IoU 58.1%% -> 86.0%%. Use --no-bug4 '
                         'to restore the pre-fix scheme (scan-forward at iteration '
                         'start) for diagnostic comparison.')
    args = ap.parse_args(argv)

    out_dir = Path(args.output_dir) if args.output_dir else None
    if out_dir:
        out_dir.mkdir(parents=True, exist_ok=True)

    for inp in args.inputs:
        path = Path(inp)
        if not path.is_file():
            print(f"skip: {path} (not a file)", file=sys.stderr)
            continue
        t0_kinds = (set(s.strip() for s in args.t0_drop_kinds.split(',') if s.strip())
                    if args.t0_drop_kinds else None)
        try:
            pages = parse_max(path, t0_reset=args.t0_reset,
                              t0_drop_after_drift=args.t0_drop_after_drift,
                              t0_drop_kinds=t0_kinds,
                              include_preview=not args.no_preview,
                              drop_blank_after_drift=not args.keep_drift_blanks,
                              suppress_t1_all=not args.keep_t1_dispatches,
                              fail_scan_forward_max=args.fail_scan_forward,
                              suppress_t2_fail_y_in_cascade=args.suppress_t2_fail_y_in_cascade,
                              fail_resync_max=args.fail_resync_max,
                              fail_resync_lookahead=args.fail_resync_lookahead,
                              fail_resync_budget=args.fail_resync_budget,
                              fail_resync_min_confidence=args.fail_resync_min_confidence,
                              reset_ref_after_drift=args.reset_ref_after_drift,
                              strict_t0=not args.no_strict_t0,
                              lazy_bit_loading=args.lazy_bit_loading,
                              bug4=not args.no_bug4)
        except Exception as exc:
            print(f"{path}: {exc}", file=sys.stderr)
            continue
        out = (out_dir / (path.stem + '.pdf')) if out_dir else path.with_suffix('.pdf')
        write_pdf(pages, out)
        print(f"{path.name} -> {out.name}  ({len(pages)} page(s))")


if __name__ == '__main__':
    main()
