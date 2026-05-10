// Items are pub(crate) for use by future decoder modules (Tasks 6+).
// Nothing in the current crate consumes them yet.
#![allow(dead_code)]
//! CCITT-T.6 (Group 4 fax) Huffman tables.
//!
//! # Provenance
//!
//! Every table value in this file was transcribed directly from two
//! non-GPL primary sources, retrieved 2026-05-10:
//!
//! **Source 1 (primary):**
//! CCITT Recommendation T.4 (11/1988), "Standardization of Group 3 Facsimile
//! Apparatus for Document Transmission", Blue Book, Fascicle VII.3.
//! Freely available from ITU at:
//! <https://www.itu.int/rec/dologin_pub.asp?lang=e&id=T-REC-T.4-198811-S!!PDF-E&type=items>
//! Tables used: Table 1/T.4 (Terminating codes, pp. 4-5), Table 2/T.4
//! (Make-up codes, p. 5), Table 3/T.4 (Two-dimensional code table, p. 9).
//!
//! **Source 2 (cross-check):**
//! TIFF Revision 6.0, Final — June 3, 1992, Aldus Corporation.
//! Available at:
//! <https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf>
//! Tables used: Table 1/T.4 (pp. 45-46), Table 2/T.4 (p. 46),
//! Additional make-up codes (p. 47). Confirmed identical to Source 1.
//!
//! These are NOT copied from paperman, max2pdf, or any other GPL source.
//! The table values are mathematical facts published in an ITU standard.
//!
//! # Cross-check note
//!
//! The values below match `python-reference/vigb_max2pdf.py`'s tables (also derived from the same
//! ITU standard). That agreement is expected — both implementations
//! transcribe from the same public specification. The provenance of THIS
//! file's values is the ITU/TIFF sources cited above, not the GPL
//! `paperman` or `orangeturtle739/max2pdf` projects.

/// Two-dimensional code dispatch entries (V(-3)..V(+3), H, P).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DispatchEntry {
    /// Vertical mode V(n) where n ∈ {-3, -2, -1, 0, +1, +2, +3}.
    V(i8),
    /// Horizontal mode (followed by two run-length codes).
    H,
    /// Pass mode.
    P,
}

/// Two-dimensional codes from ITU-T T.4 Table 3/T.4 in DISPATCH order:
/// V_L3, V_L2, V_L1, V0, V_R1, V_R2, V_R3, H, P.
///
/// Each entry: `(length_bits, code_bits)`.
///
/// Source: Table 3/T.4, CCITT Recommendation T.4 (11/1988), p. 9.
/// ```text
/// Pass    P       0001          (4 bits)
/// Horiz   H       001           (3 bits)
/// V(0)    V(0)    1             (1 bit)
/// V_R(1)  011                   (3 bits)
/// V_R(2)  000011                (6 bits)
/// V_R(3)  0000011               (7 bits)
/// V_L(1)  010                   (3 bits)
/// V_L(2)  000010                (6 bits)
/// V_L(3)  0000010               (7 bits)
/// ```
pub(crate) const TWO_D: [(u32, u32); 9] = [
    (7, 0x02), // V_L(3) = 0000010  (index 0)
    (6, 0x02), // V_L(2) = 000010   (index 1)
    (3, 0x02), // V_L(1) = 010      (index 2)
    (1, 0x01), // V(0)   = 1        (index 3)
    (3, 0x03), // V_R(1) = 011      (index 4)
    (6, 0x03), // V_R(2) = 000011   (index 5)
    (7, 0x03), // V_R(3) = 0000011  (index 6)
    (3, 0x01), // H      = 001      (index 7)
    (4, 0x01), // P      = 0001     (index 8)
];

/// Dispatch entry labels in the same order as `TWO_D`.
pub(crate) const DISPATCH: [DispatchEntry; 9] = [
    DispatchEntry::V(-3),
    DispatchEntry::V(-2),
    DispatchEntry::V(-1),
    DispatchEntry::V(0),
    DispatchEntry::V(1),
    DispatchEntry::V(2),
    DispatchEntry::V(3),
    DispatchEntry::H,
    DispatchEntry::P,
];

/// White terminating codes, ITU-T T.4 Table 1/T.4, run lengths 0..=63.
/// Each entry: `(length_bits, code_bits, run_length)`.
///
/// Source: Table 1/T.4, CCITT Recommendation T.4 (11/1988), pp. 4-5.
/// Cross-checked against TIFF 6.0 Specification Table 1/T.4, pp. 45-46.
pub(crate) const WHITE_TERM_ENTRIES: &[(u32, u32, u32)] = &[
    //  run  binary       length  hex
    (8,  0x35, 0),  //   0  00110101       8
    (6,  0x07, 1),  //   1  000111         6
    (4,  0x07, 2),  //   2  0111           4
    (4,  0x08, 3),  //   3  1000           4
    (4,  0x0B, 4),  //   4  1011           4
    (4,  0x0C, 5),  //   5  1100           4
    (4,  0x0E, 6),  //   6  1110           4
    (4,  0x0F, 7),  //   7  1111           4
    (5,  0x13, 8),  //   8  10011          5
    (5,  0x14, 9),  //   9  10100          5
    (5,  0x07, 10), //  10  00111          5
    (5,  0x08, 11), //  11  01000          5
    (6,  0x08, 12), //  12  001000         6
    (6,  0x03, 13), //  13  000011         6
    (6,  0x34, 14), //  14  110100         6
    (6,  0x35, 15), //  15  110101         6
    (6,  0x2A, 16), //  16  101010         6
    (6,  0x2B, 17), //  17  101011         6
    (7,  0x27, 18), //  18  0100111        7
    (7,  0x0C, 19), //  19  0001100        7
    (7,  0x08, 20), //  20  0001000        7
    (7,  0x17, 21), //  21  0010111        7
    (7,  0x03, 22), //  22  0000011        7
    (7,  0x04, 23), //  23  0000100        7
    (7,  0x28, 24), //  24  0101000        7
    (7,  0x2B, 25), //  25  0101011        7
    (7,  0x13, 26), //  26  0010011        7
    (7,  0x24, 27), //  27  0100100        7
    (7,  0x18, 28), //  28  0011000        7
    (8,  0x02, 29), //  29  00000010       8
    (8,  0x03, 30), //  30  00000011       8
    (8,  0x1A, 31), //  31  00011010       8
    (8,  0x1B, 32), //  32  00011011       8
    (8,  0x12, 33), //  33  00010010       8
    (8,  0x13, 34), //  34  00010011       8
    (8,  0x14, 35), //  35  00010100       8
    (8,  0x15, 36), //  36  00010101       8
    (8,  0x16, 37), //  37  00010110       8
    (8,  0x17, 38), //  38  00010111       8
    (8,  0x28, 39), //  39  00101000       8
    (8,  0x29, 40), //  40  00101001       8
    (8,  0x2A, 41), //  41  00101010       8
    (8,  0x2B, 42), //  42  00101011       8
    (8,  0x2C, 43), //  43  00101100       8
    (8,  0x2D, 44), //  44  00101101       8
    (8,  0x04, 45), //  45  00000100       8
    (8,  0x05, 46), //  46  00000101       8
    (8,  0x0A, 47), //  47  00001010       8
    (8,  0x0B, 48), //  48  00001011       8
    (8,  0x52, 49), //  49  01010010       8
    (8,  0x53, 50), //  50  01010011       8
    (8,  0x54, 51), //  51  01010100       8
    (8,  0x55, 52), //  52  01010101       8
    (8,  0x24, 53), //  53  00100100       8
    (8,  0x25, 54), //  54  00100101       8
    (8,  0x58, 55), //  55  01011000       8
    (8,  0x59, 56), //  56  01011001       8
    (8,  0x5A, 57), //  57  01011010       8
    (8,  0x5B, 58), //  58  01011011       8
    (8,  0x4A, 59), //  59  01001010       8
    (8,  0x4B, 60), //  60  01001011       8
    (8,  0x32, 61), //  61  00110010       8
    (8,  0x33, 62), //  62  00110011       8
    (8,  0x34, 63), //  63  00110100       8
];

/// White make-up codes, ITU-T T.4 Table 2/T.4, runs 64..=1728 (step 64)
/// followed by the shared extended make-up codes 1792..=2560.
/// Each entry: `(length_bits, code_bits, run_length)`.
///
/// Source: Table 2/T.4 + extended make-up codes, CCITT T.4 (11/1988), p. 5.
/// Cross-checked against TIFF 6.0 Specification, pp. 46-47.
pub(crate) const WHITE_MAKEUP_ENTRIES: &[(u32, u32, u32)] = &[
    // Standard make-up codes (run = 64 * n), 27 entries:
    (5,  0x1B,  64),  // 11011
    (5,  0x12, 128),  // 10010
    (6,  0x17, 192),  // 010111
    (7,  0x37, 256),  // 0110111
    (8,  0x36, 320),  // 00110110
    (8,  0x37, 384),  // 00110111
    (8,  0x64, 448),  // 01100100
    (8,  0x65, 512),  // 01100101
    (8,  0x68, 576),  // 01101000
    (8,  0x67, 640),  // 01100111
    (9,  0xCC, 704),  // 011001100
    (9,  0xCD, 768),  // 011001101
    (9,  0xD2, 832),  // 011010010
    (9,  0xD3, 896),  // 011010011
    (9,  0xD4, 960),  // 011010100
    (9,  0xD5, 1024), // 011010101
    (9,  0xD6, 1088), // 011010110
    (9,  0xD7, 1152), // 011010111
    (9,  0xD8, 1216), // 011011000
    (9,  0xD9, 1280), // 011011001
    (9,  0xDA, 1344), // 011011010
    (9,  0xDB, 1408), // 011011011
    (9,  0x98, 1472), // 010011000
    (9,  0x99, 1536), // 010011001
    (9,  0x9A, 1600), // 010011010
    (6,  0x18, 1664), // 011000
    (9,  0x9B, 1728), // 010011011
    // Extended make-up codes (shared white+black), 13 entries:
    // Source: Table 2/T.4 Note + extended table, CCITT T.4 (11/1988), p. 5.
    (11, 0x08, 1792), // 00000001000
    (11, 0x0C, 1856), // 00000001100
    (11, 0x0D, 1920), // 00000001101
    (12, 0x12, 1984), // 000000010010
    (12, 0x13, 2048), // 000000010011
    (12, 0x14, 2112), // 000000010100
    (12, 0x15, 2176), // 000000010101
    (12, 0x16, 2240), // 000000010110
    (12, 0x17, 2304), // 000000010111
    (12, 0x1C, 2368), // 000000011100
    (12, 0x1D, 2432), // 000000011101
    (12, 0x1E, 2496), // 000000011110
    (12, 0x1F, 2560), // 000000011111
];

/// Black terminating codes, ITU-T T.4 Table 1/T.4, run lengths 0..=63.
/// Each entry: `(length_bits, code_bits, run_length)`.
///
/// Source: Table 1/T.4, CCITT Recommendation T.4 (11/1988), pp. 4-5.
/// Cross-checked against TIFF 6.0 Specification Table 1/T.4, pp. 45-46.
pub(crate) const BLACK_TERM_ENTRIES: &[(u32, u32, u32)] = &[
    //  run  binary              length  hex
    (10, 0x037,  0), //   0  0000110111    10
    ( 3, 0x002,  1), //   1  010            3
    ( 2, 0x003,  2), //   2  11             2
    ( 2, 0x002,  3), //   3  10             2
    ( 3, 0x003,  4), //   4  011            3
    ( 4, 0x003,  5), //   5  0011           4
    ( 4, 0x002,  6), //   6  0010           4
    ( 5, 0x003,  7), //   7  00011          5
    ( 6, 0x005,  8), //   8  000101         6
    ( 6, 0x004,  9), //   9  000100         6
    ( 7, 0x004, 10), //  10  0000100        7
    ( 7, 0x005, 11), //  11  0000101        7
    ( 7, 0x007, 12), //  12  0000111        7
    ( 8, 0x004, 13), //  13  00000100       8
    ( 8, 0x007, 14), //  14  00000111       8
    ( 9, 0x018, 15), //  15  000011000      9
    (10, 0x017, 16), //  16  0000010111    10
    (10, 0x018, 17), //  17  0000011000    10
    (10, 0x008, 18), //  18  0000001000    10
    (11, 0x067, 19), //  19  00001100111   11
    (11, 0x068, 20), //  20  00001101000   11
    (11, 0x06C, 21), //  21  00001101100   11
    (11, 0x037, 22), //  22  00000110111   11
    (11, 0x028, 23), //  23  00000101000   11
    (11, 0x017, 24), //  24  00000010111   11
    (11, 0x018, 25), //  25  00000011000   11
    (12, 0x0CA, 26), //  26  000011001010  12
    (12, 0x0CB, 27), //  27  000011001011  12
    (12, 0x0CC, 28), //  28  000011001100  12
    (12, 0x0CD, 29), //  29  000011001101  12
    (12, 0x068, 30), //  30  000001101000  12
    (12, 0x069, 31), //  31  000001101001  12
    (12, 0x06A, 32), //  32  000001101010  12
    (12, 0x06B, 33), //  33  000001101011  12
    (12, 0x0D2, 34), //  34  000011010010  12
    (12, 0x0D3, 35), //  35  000011010011  12
    (12, 0x0D4, 36), //  36  000011010100  12
    (12, 0x0D5, 37), //  37  000011010101  12
    (12, 0x0D6, 38), //  38  000011010110  12
    (12, 0x0D7, 39), //  39  000011010111  12
    (12, 0x06C, 40), //  40  000001101100  12
    (12, 0x06D, 41), //  41  000001101101  12
    (12, 0x0DA, 42), //  42  000011011010  12
    (12, 0x0DB, 43), //  43  000011011011  12
    (12, 0x054, 44), //  44  000001010100  12
    (12, 0x055, 45), //  45  000001010101  12
    (12, 0x056, 46), //  46  000001010110  12
    (12, 0x057, 47), //  47  000001010111  12
    (12, 0x064, 48), //  48  000001100100  12
    (12, 0x065, 49), //  49  000001100101  12
    (12, 0x052, 50), //  50  000001010010  12
    (12, 0x053, 51), //  51  000001010011  12
    (12, 0x024, 52), //  52  000000100100  12
    (12, 0x037, 53), //  53  000000110111  12
    (12, 0x038, 54), //  54  000000111000  12
    (12, 0x027, 55), //  55  000000100111  12
    (12, 0x028, 56), //  56  000000101000  12
    (12, 0x058, 57), //  57  000001011000  12
    (12, 0x059, 58), //  58  000001011001  12
    (12, 0x02B, 59), //  59  000000101011  12
    (12, 0x02C, 60), //  60  000000101100  12
    (12, 0x05A, 61), //  61  000001011010  12
    (12, 0x066, 62), //  62  000001100110  12
    (12, 0x067, 63), //  63  000001100111  12
];

/// Black make-up codes, ITU-T T.4 Table 2/T.4, runs 64..=1728 (step 64)
/// followed by the shared extended make-up codes 1792..=2560.
/// Each entry: `(length_bits, code_bits, run_length)`.
///
/// Source: Table 2/T.4 + extended make-up codes, CCITT T.4 (11/1988), p. 5.
/// Cross-checked against TIFF 6.0 Specification, pp. 46-47.
pub(crate) const BLACK_MAKEUP_ENTRIES: &[(u32, u32, u32)] = &[
    // Standard make-up codes (run = 64 * n), 27 entries:
    (10, 0x00F,  64),  // 0000001111
    (12, 0x0C8, 128),  // 000011001000
    (12, 0x0C9, 192),  // 000011001001
    (12, 0x05B, 256),  // 000001011011
    (12, 0x033, 320),  // 000000110011
    (12, 0x034, 384),  // 000000110100
    (12, 0x035, 448),  // 000000110101
    (13, 0x06C, 512),  // 0000001101100
    (13, 0x06D, 576),  // 0000001101101
    (13, 0x04A, 640),  // 0000001001010
    (13, 0x04B, 704),  // 0000001001011
    (13, 0x04C, 768),  // 0000001001100
    (13, 0x04D, 832),  // 0000001001101
    (13, 0x072, 896),  // 0000001110010
    (13, 0x073, 960),  // 0000001110011
    (13, 0x074, 1024), // 0000001110100
    (13, 0x075, 1088), // 0000001110101
    (13, 0x076, 1152), // 0000001110110
    (13, 0x077, 1216), // 0000001110111
    (13, 0x052, 1280), // 0000001010010
    (13, 0x053, 1344), // 0000001010011
    (13, 0x054, 1408), // 0000001010100
    (13, 0x055, 1472), // 0000001010101
    (13, 0x05A, 1536), // 0000001011010
    (13, 0x05B, 1600), // 0000001011011
    (13, 0x064, 1664), // 0000001100100
    (13, 0x065, 1728), // 0000001100101
    // Extended make-up codes (shared white+black), 13 entries:
    // Source: Table 2/T.4 Note + extended table, CCITT T.4 (11/1988), p. 5.
    (11, 0x08, 1792), // 00000001000
    (11, 0x0C, 1856), // 00000001100
    (11, 0x0D, 1920), // 00000001101
    (12, 0x12, 1984), // 000000010010
    (12, 0x13, 2048), // 000000010011
    (12, 0x14, 2112), // 000000010100
    (12, 0x15, 2176), // 000000010101
    (12, 0x16, 2240), // 000000010110
    (12, 0x17, 2304), // 000000010111
    (12, 0x1C, 2368), // 000000011100
    (12, 0x1D, 2432), // 000000011101
    (12, 0x1E, 2496), // 000000011110
    (12, 0x1F, 2560), // 000000011111
];

/// 7-bit dispatcher table entry: which DISPATCH index matched, and how many
/// bits the matched code consumed.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DispatchHit {
    /// Index into `DISPATCH` / `TWO_D`.
    pub dispatch_idx: u8,
    /// Number of bits consumed by this code.
    pub code_len: u32,
}

/// 13-bit run-length lookup entry.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RunHit {
    /// Decoded run length in pixels.
    pub run: u32,
    /// Number of bits consumed by this code.
    pub code_len: u32,
}

/// 7-bit dispatcher table built from `TWO_D`: index by top 7 bits of stream.
///
/// Built at first use via [`std::sync::LazyLock`]. Entry is `None` if those
/// 7 bits do not begin any valid 2D code.
pub(crate) static TAB7: std::sync::LazyLock<[Option<DispatchHit>; 128]> =
    std::sync::LazyLock::new(|| {
        let mut table: [Option<DispatchHit>; 128] = [None; 128];
        for (idx, &(length, code)) in TWO_D.iter().enumerate() {
            let pad = 7 - length;
            let base = (code << pad) as usize;
            let span = 1usize << pad;
            for j in 0..span {
                table[base + j] = Some(DispatchHit {
                    dispatch_idx: idx as u8,
                    code_len: length,
                });
            }
        }
        table
    });

/// 13-bit white run-length lookup table. Combines terminating and make-up codes.
///
/// Index by the top 13 bits of the bit-stream. Returns `None` if no valid code
/// starts at those bits.
pub(crate) static WHITE_TABLE: std::sync::LazyLock<Vec<Option<RunHit>>> =
    std::sync::LazyLock::new(|| build_run_table(WHITE_TERM_ENTRIES, WHITE_MAKEUP_ENTRIES));

/// 13-bit black run-length lookup table. Combines terminating and make-up codes.
pub(crate) static BLACK_TABLE: std::sync::LazyLock<Vec<Option<RunHit>>> =
    std::sync::LazyLock::new(|| build_run_table(BLACK_TERM_ENTRIES, BLACK_MAKEUP_ENTRIES));

fn build_run_table(
    term: &[(u32, u32, u32)],
    makeup: &[(u32, u32, u32)],
) -> Vec<Option<RunHit>> {
    let mut table: Vec<Option<RunHit>> = vec![None; 1 << 13];
    for entries in [term, makeup] {
        for &(length, code, run) in entries {
            let pad = 13 - length;
            let base = (code << pad) as usize;
            let span = 1usize << pad;
            for j in 0..span {
                if table[base + j].is_none() {
                    table[base + j] = Some(RunHit {
                        run,
                        code_len: length,
                    });
                }
            }
        }
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_term_zero_is_8b_0x35() {
        // ITU-T T.4 Table 1/T.4: white terminating, run length 0 = 00110101 (8 bits)
        let (length, code, _payload) = WHITE_TERM_ENTRIES[0];
        assert_eq!(length, 8);
        assert_eq!(code, 0x35);
    }

    #[test]
    fn black_term_zero_is_10b_0x37() {
        // ITU-T T.4 Table 1/T.4: black terminating, run length 0 = 0000110111 (10 bits)
        let (length, code, _payload) = BLACK_TERM_ENTRIES[0];
        assert_eq!(length, 10);
        assert_eq!(code, 0x37);
    }

    #[test]
    fn two_d_v0_is_1b_1() {
        // ITU-T T.4 Table 3/T.4: V(0) = 1 (1 bit)
        let (length, code) = TWO_D[3]; // V(0) is index 3 in DISPATCH order
        assert_eq!(length, 1);
        assert_eq!(code, 0x01);
    }

    #[test]
    fn dispatch_order_matches_two_d() {
        assert_eq!(DISPATCH.len(), TWO_D.len());
        assert_eq!(DISPATCH[3], DispatchEntry::V(0));
        assert_eq!(DISPATCH[7], DispatchEntry::H);
        assert_eq!(DISPATCH[8], DispatchEntry::P);
    }

    #[test]
    fn tab7_lookup_resolves_v0() {
        // V(0) = code `1` of length 1. With 7-bit lookup, the entry at index
        // 0b1xxxxxx (= 0x40..=0x7F) should map to (DISPATCH idx 3, length 1).
        for top7 in 0x40..=0x7F {
            let entry = TAB7[top7 as usize].expect("V0 must populate top half");
            assert_eq!(entry.dispatch_idx, 3);
            assert_eq!(entry.code_len, 1);
        }
    }

    #[test]
    fn white_table_decodes_short_run() {
        // White run-length 2 = 0111 (4 bits) per ITU-T T.4 Table 1/T.4.
        // Top-13-bit lookup at 0b0111000000000 (0x0E00) should resolve.
        let entry = WHITE_TABLE[0x0E00].expect("white run 2 must lookup");
        assert_eq!(entry.run, 2);
        assert_eq!(entry.code_len, 4);
    }

    // ---- Additional sanity checks ----

    #[test]
    fn white_term_entries_count() {
        assert_eq!(WHITE_TERM_ENTRIES.len(), 64);
    }

    #[test]
    fn black_term_entries_count() {
        assert_eq!(BLACK_TERM_ENTRIES.len(), 64);
    }

    #[test]
    fn white_makeup_entries_count() {
        // 27 standard + 13 extended = 40
        assert_eq!(WHITE_MAKEUP_ENTRIES.len(), 40);
    }

    #[test]
    fn black_makeup_entries_count() {
        // 27 standard + 13 extended = 40
        assert_eq!(BLACK_MAKEUP_ENTRIES.len(), 40);
    }

    #[test]
    fn white_term_run_lengths_are_sequential() {
        for (i, &(_len, _code, run)) in WHITE_TERM_ENTRIES.iter().enumerate() {
            assert_eq!(run, i as u32, "white terminating run at index {i} should be {i}");
        }
    }

    #[test]
    fn black_term_run_lengths_are_sequential() {
        for (i, &(_len, _code, run)) in BLACK_TERM_ENTRIES.iter().enumerate() {
            assert_eq!(run, i as u32, "black terminating run at index {i} should be {i}");
        }
    }

    #[test]
    fn two_d_v_r1_is_3b_0x03() {
        // ITU-T T.4 Table 3/T.4: V_R(1) = 011 (3 bits)
        let (length, code) = TWO_D[4];
        assert_eq!(length, 3);
        assert_eq!(code, 0x03);
    }

    #[test]
    fn two_d_h_is_3b_0x01() {
        // ITU-T T.4 Table 3/T.4: H = 001 (3 bits)
        let (length, code) = TWO_D[7];
        assert_eq!(length, 3);
        assert_eq!(code, 0x01);
    }

    #[test]
    fn two_d_p_is_4b_0x01() {
        // ITU-T T.4 Table 3/T.4: P = 0001 (4 bits)
        let (length, code) = TWO_D[8];
        assert_eq!(length, 4);
        assert_eq!(code, 0x01);
    }

    #[test]
    fn white_makeup_64_is_5b_0x1b() {
        // ITU-T T.4 Table 2/T.4: white make-up 64 = 11011 (5 bits)
        let &(length, code, run) = &WHITE_MAKEUP_ENTRIES[0];
        assert_eq!(length, 5);
        assert_eq!(code, 0x1B);
        assert_eq!(run, 64);
    }

    #[test]
    fn black_makeup_64_is_10b_0x0f() {
        // ITU-T T.4 Table 2/T.4: black make-up 64 = 0000001111 (10 bits)
        let &(length, code, run) = &BLACK_MAKEUP_ENTRIES[0];
        assert_eq!(length, 10);
        assert_eq!(code, 0x0F);
        assert_eq!(run, 64);
    }

    #[test]
    fn extended_makeup_1792_shared() {
        // Both white and black share 1792 = 00000001000 (11 bits, code 0x08)
        let white_ext = WHITE_MAKEUP_ENTRIES.iter().find(|&&(_, _, r)| r == 1792).copied();
        let black_ext = BLACK_MAKEUP_ENTRIES.iter().find(|&&(_, _, r)| r == 1792).copied();
        assert_eq!(white_ext, black_ext);
        let (len, code, _) = white_ext.expect("1792 must be in white makeup");
        assert_eq!(len, 11);
        assert_eq!(code, 0x08);
    }

    #[test]
    fn black_table_run1_decodes() {
        // Black run 1 = 010 (3 bits). Top 13 bits = 0b0100000000000 = 0x0800.
        let entry = BLACK_TABLE[0x0800].expect("black run 1 must decode");
        assert_eq!(entry.run, 1);
        assert_eq!(entry.code_len, 3);
    }
}
