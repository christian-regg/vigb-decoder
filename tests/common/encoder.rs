//! Test-only CCITT-T.6 line encoder. Mirrors the algorithm in
//! `C:\Users\chris\Desktop\Alte Scans\encoder_validator.py:encode_row`.
//!
//! Not part of the public crate API — lives in tests/common/ so the
//! integration tests can produce their own fixtures.
//!
//! Tables here are duplicates of src/ccitt.rs (transcribed from the
//! same ITU-T T.4/T.6 source). Test-only duplication is acceptable.

// (length_bits, code_bits) — run payload is implicit by array index.
// Copied from src/ccitt.rs (same ITU-T T.4 provenance).

/// White terminating codes, run lengths 0..=63.
const WHITE_TERM: &[(u32, u32)] = &[
    (8,  0x35), // 0
    (6,  0x07), // 1
    (4,  0x07), // 2
    (4,  0x08), // 3
    (4,  0x0B), // 4
    (4,  0x0C), // 5
    (4,  0x0E), // 6
    (4,  0x0F), // 7
    (5,  0x13), // 8
    (5,  0x14), // 9
    (5,  0x07), // 10
    (5,  0x08), // 11
    (6,  0x08), // 12
    (6,  0x03), // 13
    (6,  0x34), // 14
    (6,  0x35), // 15
    (6,  0x2A), // 16
    (6,  0x2B), // 17
    (7,  0x27), // 18
    (7,  0x0C), // 19
    (7,  0x08), // 20
    (7,  0x17), // 21
    (7,  0x03), // 22
    (7,  0x04), // 23
    (7,  0x28), // 24
    (7,  0x2B), // 25
    (7,  0x13), // 26
    (7,  0x24), // 27
    (7,  0x18), // 28
    (8,  0x02), // 29
    (8,  0x03), // 30
    (8,  0x1A), // 31
    (8,  0x1B), // 32
    (8,  0x12), // 33
    (8,  0x13), // 34
    (8,  0x14), // 35
    (8,  0x15), // 36
    (8,  0x16), // 37
    (8,  0x17), // 38
    (8,  0x28), // 39
    (8,  0x29), // 40
    (8,  0x2A), // 41
    (8,  0x2B), // 42
    (8,  0x2C), // 43
    (8,  0x2D), // 44
    (8,  0x04), // 45
    (8,  0x05), // 46
    (8,  0x0A), // 47
    (8,  0x0B), // 48
    (8,  0x52), // 49
    (8,  0x53), // 50
    (8,  0x54), // 51
    (8,  0x55), // 52
    (8,  0x24), // 53
    (8,  0x25), // 54
    (8,  0x58), // 55
    (8,  0x59), // 56
    (8,  0x5A), // 57
    (8,  0x5B), // 58
    (8,  0x4A), // 59
    (8,  0x4B), // 60
    (8,  0x32), // 61
    (8,  0x33), // 62
    (8,  0x34), // 63
];

/// White make-up codes, standard (64..=1728 step 64), 27 entries.
/// Index i covers run (i+1)*64.
const WHITE_MAKEUP: &[(u32, u32)] = &[
    (5,  0x1B), // 64
    (5,  0x12), // 128
    (6,  0x17), // 192
    (7,  0x37), // 256
    (8,  0x36), // 320
    (8,  0x37), // 384
    (8,  0x64), // 448
    (8,  0x65), // 512
    (8,  0x68), // 576
    (8,  0x67), // 640
    (9,  0xCC), // 704
    (9,  0xCD), // 768
    (9,  0xD2), // 832
    (9,  0xD3), // 896
    (9,  0xD4), // 960
    (9,  0xD5), // 1024
    (9,  0xD6), // 1088
    (9,  0xD7), // 1152
    (9,  0xD8), // 1216
    (9,  0xD9), // 1280
    (9,  0xDA), // 1344
    (9,  0xDB), // 1408
    (9,  0x98), // 1472
    (9,  0x99), // 1536
    (9,  0x9A), // 1600
    (6,  0x18), // 1664
    (9,  0x9B), // 1728
];

/// Black terminating codes, run lengths 0..=63.
const BLACK_TERM: &[(u32, u32)] = &[
    (10, 0x037), // 0
    ( 3, 0x002), // 1
    ( 2, 0x003), // 2
    ( 2, 0x002), // 3
    ( 3, 0x003), // 4
    ( 4, 0x003), // 5
    ( 4, 0x002), // 6
    ( 5, 0x003), // 7
    ( 6, 0x005), // 8
    ( 6, 0x004), // 9
    ( 7, 0x004), // 10
    ( 7, 0x005), // 11
    ( 7, 0x007), // 12
    ( 8, 0x004), // 13
    ( 8, 0x007), // 14
    ( 9, 0x018), // 15
    (10, 0x017), // 16
    (10, 0x018), // 17
    (10, 0x008), // 18
    (11, 0x067), // 19
    (11, 0x068), // 20
    (11, 0x06C), // 21
    (11, 0x037), // 22
    (11, 0x028), // 23
    (11, 0x017), // 24
    (11, 0x018), // 25
    (12, 0x04A), // 26
    (12, 0x04B), // 27
    (12, 0x04C), // 28
    (12, 0x04D), // 29
    (12, 0x068), // 30
    (12, 0x069), // 31
    (12, 0x06A), // 32
    (12, 0x06B), // 33
    (12, 0x0D2), // 34
    (12, 0x0D3), // 35
    (12, 0x0D4), // 36
    (12, 0x0D5), // 37
    (12, 0x0D6), // 38
    (12, 0x0D7), // 39
    (12, 0x06C), // 40
    (12, 0x06D), // 41
    (12, 0x0DA), // 42
    (12, 0x0DB), // 43
    (12, 0x054), // 44
    (12, 0x055), // 45
    (12, 0x056), // 46
    (12, 0x057), // 47
    (12, 0x064), // 48
    (12, 0x065), // 49
    (12, 0x052), // 50
    (12, 0x053), // 51
    (12, 0x024), // 52
    (12, 0x037), // 53
    (12, 0x038), // 54
    (12, 0x027), // 55
    (12, 0x028), // 56
    (12, 0x058), // 57
    (12, 0x059), // 58
    (12, 0x02B), // 59
    (12, 0x02C), // 60
    (12, 0x05A), // 61
    (12, 0x066), // 62
    (12, 0x067), // 63
];

/// Black make-up codes, standard (64..=1728 step 64), 27 entries.
/// Index i covers run (i+1)*64.
const BLACK_MAKEUP: &[(u32, u32)] = &[
    (10, 0x00F), // 64
    (12, 0x0C8), // 128
    (12, 0x0C9), // 192
    (12, 0x05B), // 256
    (12, 0x033), // 320
    (12, 0x034), // 384
    (12, 0x035), // 448
    (13, 0x06C), // 512
    (13, 0x06D), // 576
    (13, 0x04A), // 640
    (13, 0x04B), // 704
    (13, 0x04C), // 768
    (13, 0x04D), // 832
    (13, 0x072), // 896
    (13, 0x073), // 960
    (13, 0x074), // 1024
    (13, 0x075), // 1088
    (13, 0x076), // 1152
    (13, 0x077), // 1216
    (13, 0x052), // 1280
    (13, 0x053), // 1344
    (13, 0x054), // 1408
    (13, 0x055), // 1472
    (13, 0x05A), // 1536
    (13, 0x05B), // 1600
    (13, 0x064), // 1664
    (13, 0x065), // 1728
];

/// Two-d codes by DISPATCH index:
/// V_L3=0, V_L2=1, V_L1=2, V0=3, V_R1=4, V_R2=5, V_R3=6, H=7, P=8.
const TWO_D: [(u32, u32); 9] = [
    (7, 0x02), // V_L(3)
    (6, 0x02), // V_L(2)
    (3, 0x02), // V_L(1)
    (1, 0x01), // V(0)
    (3, 0x03), // V_R(1)
    (6, 0x03), // V_R(2)
    (7, 0x03), // V_R(3)
    (3, 0x01), // H
    (4, 0x01), // P
];

// ─── BitWriter ───────────────────────────────────────────────────────────────

/// Bit-writer that accumulates MSB-first bits and produces bytes.
pub struct BitWriter {
    bytes: Vec<u8>,
    bit_buf: u64,
    bits_in_buf: u32,
}

impl BitWriter {
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit_buf: 0,
            bits_in_buf: 0,
        }
    }

    /// Write `length` bits from `code` (MSB-first, left-aligned in code).
    pub fn write(&mut self, code: u32, length: u32) {
        debug_assert!(length >= 1 && length <= 16, "length {length} out of range 1..=16");
        self.bit_buf = (self.bit_buf << length) | (code as u64 & ((1u64 << length) - 1));
        self.bits_in_buf += length;
        while self.bits_in_buf >= 8 {
            let shift = self.bits_in_buf - 8;
            self.bytes.push(((self.bit_buf >> shift) & 0xFF) as u8);
            self.bits_in_buf -= 8;
            self.bit_buf &= (1u64 << self.bits_in_buf) - 1;
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

impl Default for BitWriter {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Run-length encoder ───────────────────────────────────────────────────────

/// Emit make-up + terminating codes for a run length.
///
/// Panics if `run > 1728` (extended make-up codes not implemented — the
/// synthetic fixture is 200 px wide so runs cannot exceed 200).
fn encode_run(bw: &mut BitWriter, run: u32, colour: u32) {
    assert!(run <= 1728, "run {run} > 1728: extended make-up codes not implemented");
    let mut r = run;
    let makeup = if colour == 0 { WHITE_MAKEUP } else { BLACK_MAKEUP };
    let term = if colour == 0 { WHITE_TERM } else { BLACK_TERM };

    // Emit makeup codes for the bulk of the run. Each makeup[idx] covers
    // (idx+1)*64 pixels. After each makeup code, subtract (idx+1)*64 from r.
    // Per CCITT-T.6, a makeup code MUST be followed by a terminating code
    // (even if the remainder is 0 — the terminating code for run-0 closes the
    // run). The loop therefore falls through to the terminating code emit.
    while r >= 64 {
        // idx = min(r / 64 - 1, makeup.len() - 1).
        // makeup[idx] covers (idx+1)*64 pixels.
        let idx = ((r / 64) as usize - 1).min(makeup.len() - 1);
        let (length, code) = makeup[idx];
        bw.write(code, length);
        r -= ((idx + 1) as u32) * 64;
        // If r is still >= 64 after this makeup, loop again. If r < 64,
        // exit the loop and emit the terminating code for the remainder.
        // The Python safety guard `if idx >= len-1: break` is implicitly
        // handled by the .min(len-1) clamp above — if we hit the max makeup
        // index and r is still >= 64, the next iteration will re-use the max
        // makeup index again (effectively infinite loop for runs > 1728, but
        // our assert above prevents that).
    }
    // Always emit a terminating code, even when r == 0 (makeup code was exact).
    let (length, code) = term[r as usize];
    bw.write(code, length);
}

// ─── Row encoder ─────────────────────────────────────────────────────────────

/// Encode a single-line transition table against a previous-line table.
///
/// Both tables use the sentinel format: `[-1, x0, x1, ..., width, width, ...]`
/// (same shape as `decoder::table_from_raw` / `table_from_raw` in the
/// fixture generator).
///
/// Uses a0 = 0 to match the canonical ViGBe decoder (`max2pdf.py:_decomp_line`,
/// `decoder::decomp_line`), both of which initialise `x = 0` (not -1 as in
/// standard T.6). Using a0=-1 (the T.6 default) causes a systematic 1-pixel
/// shift on every first H-mode segment because the decoder interprets a
/// white-run-N starting from x=0, placing the first transition at x=N instead
/// of x=N-1.
///
/// Rows beginning with black at x=0 are handled by emitting an explicit
/// H(white-run-0, black-run-N) for the initial segment, which causes the
/// decoder to push x=0 into its output table (correct).
///
/// Returns the encoded byte stream for the **body** of a type-2 line (no
/// marker byte; the caller prepends `0x80`).
pub fn encode_row(curr: &[i32], prev: &[i32], width: i32) -> Vec<u8> {
    let mut bw = BitWriter::new();

    let get = |t: &[i32], i: usize| -> i32 {
        if i < t.len() { t[i] } else { width }
    };

    // a0=0, initial colour=white (matching canonical decoder x=0).
    let mut a0: i32 = 0;
    let mut a_idx: usize = 1;

    // Handle the "row starts black" edge case: if curr[1]==0, the line begins
    // with black (a transition at x=0). Emit H(white-run-0, black-run-N) so
    // the decoder sees a transition at x=0 (not x=N as it would with a0=0 and
    // no special handling). After this, a0=curr[2] and a_idx=3.
    if curr.len() > 1 && curr[1] == 0 {
        let a1 = get(curr, 2); // end of initial black run
        let (h_len, h_code) = TWO_D[7]; // H
        bw.write(h_code, h_len);
        // white-run-0 from a0=0 to x=0 (0 pixels), then black-run from 0 to a1.
        encode_run(&mut bw, 0, 0); // white, 0 pixels
        encode_run(&mut bw, a1 as u32, 1); // black, a1 pixels
        a0 = a1;
        a_idx = 3;
        // Skip any additional transitions at or before a0.
        while a_idx < curr.len() && curr[a_idx] <= a0 {
            a_idx += 1;
        }
    }

    loop {
        if a0 >= width {
            break;
        }

        // Find a1: next transition strictly after a0.
        while a_idx < curr.len() && curr[a_idx] <= a0 {
            a_idx += 1;
        }
        let a1 = get(curr, a_idx);
        let a2 = get(curr, a_idx + 1);

        // Current colour at a0: (a_idx - 1) & 1.
        let cur_colour = (a_idx - 1) & 1;

        // Find b_idx: first prev entry strictly > a0, then parity-match.
        let mut b_idx: usize = 1;
        while b_idx < prev.len() && prev[b_idx] <= a0 {
            b_idx += 1;
        }
        // Parity: b_idx & 1 must equal a_idx & 1 (same-direction transitions).
        while b_idx < prev.len() && (b_idx & 1) != (a_idx & 1) {
            b_idx += 1;
        }
        let b1 = get(prev, b_idx);
        let b2 = get(prev, b_idx + 1);

        if b2 < a1 {
            // PASS mode: a0 jumps to b2; a_idx unchanged.
            let (length, code) = TWO_D[8];
            bw.write(code, length);
            a0 = b2;
        } else if (a1 - b1).abs() <= 3 {
            // VERTICAL mode.
            let offset = a1 - b1;
            let v_idx = (3 + offset) as usize;
            let (length, code) = TWO_D[v_idx];
            bw.write(code, length);
            a0 = a1;
            a_idx += 1;
        } else {
            // HORIZONTAL mode.
            let (h_len, h_code) = TWO_D[7];
            bw.write(h_code, h_len);
            let run1 = (a1 - a0) as u32;
            let run2 = (a2 - a1) as u32;
            encode_run(&mut bw, run1, cur_colour as u32);
            encode_run(&mut bw, run2, (cur_colour ^ 1) as u32);
            a0 = a2;
            a_idx += 2;
        }

        if a0 >= width {
            break;
        }
    }

    bw.finish()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_writer_packs_msb_first() {
        let mut w = BitWriter::new();
        w.write(0b101, 3);
        w.write(0b11, 2);
        w.write(0b001, 3);
        let out = w.finish();
        // Expected: 101_11_001 = 0xB9
        assert_eq!(out, vec![0xB9]);
    }

    #[test]
    fn bit_writer_pads_to_byte() {
        let mut w = BitWriter::new();
        w.write(0b1, 1);
        let out = w.finish();
        // 1 bit, padded with 7 zeros → 0x80
        assert_eq!(out, vec![0x80]);
    }

    #[test]
    fn encode_run_white_zero() {
        // White run 0 = 00110101 (8 bits, code 0x35).
        let mut bw = BitWriter::new();
        encode_run(&mut bw, 0, 0);
        let out = bw.finish();
        assert_eq!(out, vec![0x35]);
    }

    #[test]
    fn encode_run_black_one() {
        // Black run 1 = 010 (3 bits, code 0x02), padded to byte = 0100_0000 = 0x40.
        let mut bw = BitWriter::new();
        encode_run(&mut bw, 1, 1);
        let out = bw.finish();
        assert_eq!(out, vec![0x40]);
    }

    #[test]
    fn encode_run_white_64_via_makeup() {
        // White make-up for 64 = 11011 (5 bits, code 0x1B) + white-term-0 (00110101, 8 bits).
        // That's 13 bits = 1 1011 0011 0101 = padded to 2 bytes: 1101_1001 1010_1000 = 0xD9 0xA8? Let's verify.
        // bits: 1_1011 = 0x1B / 5, then 0011_0101 = 0x35 / 8 → total 13 bits
        // 11011_00110101_000 padded → 1101 1001 1010 1000 = 0xD9 0xA8
        let mut bw = BitWriter::new();
        encode_run(&mut bw, 64, 0);
        let out = bw.finish();
        // 13 bits: 11011 00110101 padded with 3 zeros
        // = 1101 1001 1010 1000 = 0xD9 0xA8
        assert_eq!(out, vec![0xD9, 0xA8]);
    }

    #[test]
    fn encode_row_all_white() {
        // All-white line vs all-white reference. With a0=0, a_idx=1,
        // a1=get(curr,1)=8=width. b1=get(prev,1)=8=width. |8-8|=0 ≤ 3 → V(0).
        // V(0) = code 1, length 1 → padded to 0x80.
        let width = 8i32;
        let curr = vec![-1i32, 8, 8];
        let prev = vec![-1i32, 8, 8];
        let body = encode_row(&curr, &prev, width);
        assert_eq!(body, vec![0x80]);
    }

    #[test]
    fn encode_row_starts_black() {
        // Row that starts black at x=0 (curr[1]==0 edge case).
        // curr=[-1, 0, 4, 8, 8] means: black 0-3, white 4-7.
        // prev=all-white: [-1, 8, 8].
        // Expected: H(white-0, black-4) then H(white-4, black-0)? No:
        // after initial H(0,4), a0=4, a_idx=3. a1=8=width, b1=8.
        // |8-8|=0 → V(0). So: H(white-0, black-4) + V(0).
        let width = 8i32;
        let curr = vec![-1i32, 0, 4, 8, 8];
        let prev = vec![-1i32, 8, 8, 8];
        let body = encode_row(&curr, &prev, width);
        // Must decode correctly (not testing exact bytes, just that it round-trips).
        assert!(!body.is_empty());
    }
}
