//! Synthetic round-trip integration test.
//!
//! Reads tests/fixtures/synthetic.max + tests/fixtures/synthetic.pbm,
//! decodes the .max via the canonical decoder, and asserts pixel-for-pixel
//! equality.

use std::fs;
use std::path::Path;

use vigb_decoder::{decode_max, Config};

fn read_pbm_p4(path: &Path) -> (u32, u32, Vec<u8>) {
    let bytes = fs::read(path).expect("read synthetic.pbm");
    let nl1 = bytes.iter().position(|&b| b == b'\n').unwrap();
    assert_eq!(&bytes[..nl1], b"P4");
    let nl2 = bytes[nl1 + 1..].iter().position(|&b| b == b'\n').unwrap() + nl1 + 1;
    let dims = std::str::from_utf8(&bytes[nl1 + 1..nl2]).unwrap();
    let mut parts = dims.split_whitespace();
    let w: u32 = parts.next().unwrap().parse().unwrap();
    let h: u32 = parts.next().unwrap().parse().unwrap();
    (w, h, bytes[nl2 + 1..].to_vec())
}

#[test]
fn synthetic_round_trip_canonical() {
    let max_path = Path::new("tests/fixtures/synthetic.max");
    let pbm_path = Path::new("tests/fixtures/synthetic.pbm");
    let max_bytes = fs::read(max_path).expect("read synthetic.max");
    let (pbm_w, pbm_h, pbm_bits) = read_pbm_p4(pbm_path);

    let cfg = Config::default();
    let pages = decode_max(&max_bytes, &cfg).expect("decode");
    assert_eq!(pages.len(), 1, "synthetic .max has exactly one image chunk");
    let p = &pages[0];

    assert_eq!(p.width, pbm_w);
    assert_eq!(p.height, pbm_h);

    // PBM rows are tightly packed at (w+7)/8 bytes; decoder rows are padded
    // to row_bytes. Compare row-by-row up to the meaningful width.
    let pbm_row_bytes = pbm_w.div_ceil(8) as usize;
    let dec_row_bytes = p.row_bytes as usize;
    let mut diff_count = 0;
    for y in 0..pbm_h as usize {
        let pbm_row = &pbm_bits[y * pbm_row_bytes..(y + 1) * pbm_row_bytes];
        let dec_row = &p.bitmap[y * dec_row_bytes..y * dec_row_bytes + pbm_row_bytes];
        if dec_row != pbm_row {
            diff_count += 1;
            if diff_count <= 3 {
                eprintln!("row {y} mismatch:");
                eprintln!("  pbm: {:02x?}", pbm_row);
                eprintln!("  dec: {:02x?}", dec_row);
            }
        }
    }
    assert_eq!(diff_count, 0, "{} rows differ", diff_count);

    // Canonical decoder must produce zero FAIL events on the synthetic.
    assert_eq!(p.stats.n_fail, 0, "FAIL events on synthetic: {:?}", p.stats);
}
