//! Local-only corpus regression test.
//!
//! Decodes every `.max` file in a directory and asserts the rendered
//! bitmap matches a known-good reference produced by the Python
//! `python-reference/vigb_max2pdf.py`. Used to catch decoder
//! regressions against the user's private archive without committing
//! personal documents to git.
//!
//! Gated behind the `corpus` Cargo feature; CI never enables it.
//!
//! ## How to run
//!
//! ```powershell
//! $env:VIGB_DECODER_CORPUS = "C:\path\to\folder\with\.max\files"
//! $env:VIGB_DECODER_REFERENCE = "C:\path\to\folder\with\reference\.pdf\files"
//! cargo test --features corpus --test corpus -- --nocapture
//! ```
//!
//! `VIGB_DECODER_CORPUS` is the directory containing input `.max` files.
//!
//! `VIGB_DECODER_REFERENCE` is the directory containing reference PDFs
//! produced by the Python decoder over the same files (same basename,
//! `.pdf` extension). Generate them once with:
//!
//! ```powershell
//! python <repo>/python-reference/vigb_max2pdf.py "$env:VIGB_DECODER_CORPUS\*.max" -o "$env:VIGB_DECODER_REFERENCE"
//! ```
//!
//! The test extracts the 1-bit image XObject from page 0 of each
//! reference PDF (which is the main image; preview pages are skipped)
//! and compares pixel-for-pixel against the Rust decoder's bitmap.
//!
//! ## What this test does NOT do
//!
//! - Does not commit any `.max` or `.pdf` files to the repo.
//! - Does not require the Python decoder to be installed at test time
//!   (only at reference-generation time).
//! - Does not extract preview pages from the reference PDFs.

#![cfg(feature = "corpus")]

use std::fs;
use std::path::{Path, PathBuf};

use vigb_decoder::{decode_max_file, Config};

#[test]
fn corpus_regression() {
    let corpus_dir = match std::env::var("VIGB_DECODER_CORPUS") {
        Ok(s) => PathBuf::from(s),
        Err(_) => {
            eprintln!("VIGB_DECODER_CORPUS not set; skipping corpus test");
            return;
        }
    };
    let reference_dir = match std::env::var("VIGB_DECODER_REFERENCE") {
        Ok(s) => PathBuf::from(s),
        Err(_) => {
            eprintln!("VIGB_DECODER_REFERENCE not set; skipping corpus test");
            return;
        }
    };

    let mut total_files = 0;
    let mut total_pages = 0;
    let mut mismatch_files: Vec<String> = Vec::new();

    for entry in fs::read_dir(&corpus_dir).expect("read corpus dir") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("max") {
            continue;
        }
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let ref_pdf = reference_dir.join(format!("{stem}.pdf"));
        if !ref_pdf.exists() {
            eprintln!("skipping {stem}: no reference PDF at {}", ref_pdf.display());
            continue;
        }

        total_files += 1;
        let pages = decode_max_file(&path, &Config::default())
            .unwrap_or_else(|e| panic!("decode {}: {e}", path.display()));

        let ref_pages = extract_pdf_main_images(&ref_pdf);
        if ref_pages.len() < pages.len() {
            panic!(
                "{stem}: reference has {} main pages, decoder produced {}",
                ref_pages.len(),
                pages.len()
            );
        }

        for (i, page) in pages.iter().enumerate() {
            total_pages += 1;
            let (ref_w, ref_h, ref_bits) = &ref_pages[i];
            assert_eq!(page.width, *ref_w, "{stem} page {i} width");
            assert_eq!(page.height, *ref_h, "{stem} page {i} height");

            let pbm_row_bytes = ref_w.div_ceil(8) as usize;
            let dec_row_bytes = page.row_bytes as usize;
            let mut row_diffs = 0u32;
            for y in 0..*ref_h as usize {
                let pbm_row = &ref_bits[y * pbm_row_bytes..(y + 1) * pbm_row_bytes];
                let dec_row = &page.bitmap[y * dec_row_bytes..y * dec_row_bytes + pbm_row_bytes];
                if pbm_row != dec_row {
                    row_diffs += 1;
                }
            }
            if row_diffs > 0 {
                eprintln!("  {stem} page {i}: {row_diffs} rows differ");
                mismatch_files.push(format!("{stem} page {i}"));
            }
        }
    }

    eprintln!(
        "Corpus regression: {} files, {} pages, {} mismatches",
        total_files,
        total_pages,
        mismatch_files.len()
    );
    assert!(
        mismatch_files.is_empty(),
        "regression: {} pages differ from Python reference",
        mismatch_files.len()
    );
}

/// Extract the 1-bit packed bitmap (MSB-first, 1=BLACK) from the first
/// image XObject of every odd page (page 0, 2, 4...) of a PDF. Skips
/// preview pages (the Python decoder embeds preview as page i+1).
///
/// Returns one `(width, height, bytes)` per main page in document order.
///
/// Implemented by parsing the PDF text minimally + flate-decoding the
/// image XObject stream. PDF spec is enough for our specific output:
/// each main page has exactly one `/Type /XObject /Subtype /Image
/// /Filter /FlateDecode` stream, with `/Width N /Height M
/// /BitsPerComponent 1 /ColorSpace [/Indexed /DeviceGray 1 <FF 00>]`.
fn extract_pdf_main_images(path: &Path) -> Vec<(u32, u32, Vec<u8>)> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut images: Vec<(u32, u32, Vec<u8>, usize)> = Vec::new();

    // Find every "/Width N /Height M ... stream\n...endstream".
    let mut pos = 0;
    while let Some(rel) = find_subslice(&bytes[pos..], b"/Width") {
        let abs = pos + rel;
        // Parse /Width N /Height M.
        let after = &bytes[abs..];
        let width = parse_int_after(after, b"/Width");
        let height = parse_int_after(after, b"/Height");
        // Bits per component must be 1 — assert and bail if not.
        let bpc = parse_int_after(after, b"/BitsPerComponent");
        if bpc != 1 {
            pos = abs + 1;
            continue;
        }
        let length = parse_int_after(after, b"/Length") as usize;
        // Find the start of the stream payload: "stream\n".
        let stream_off = find_subslice(after, b"stream\n").expect("stream marker");
        let payload_start = abs + stream_off + b"stream\n".len();
        let payload = &bytes[payload_start..payload_start + length];
        let mut decoded: Vec<u8> = Vec::with_capacity((width * height) as usize / 8);
        ZlibDecoder::new(payload)
            .read_to_end(&mut decoded)
            .expect("zlib");
        images.push((width as u32, height as u32, decoded, abs));
        pos = payload_start + length;
    }

    // Sort by source-order position; keep the largest of any pair (main page
    // beats preview by row count). Since our PDF writer emits main + preview
    // alternately and previews are always upscaled to main dimensions, a
    // simple "every other image" filter would also work — but checking by
    // dimension is more robust if the PDF layout shifts.
    images.sort_by_key(|(_, _, _, off)| *off);
    // The Python writer emits main image then preview per page. Both have
    // the same width/height (preview is upscaled to A4). So we can't filter
    // by dimensions alone. Take every other image starting at index 0
    // (main pages are 0, 2, 4, ...).
    let mut main_pages = Vec::new();
    for (i, (w, h, data, _)) in images.into_iter().enumerate() {
        if i % 2 == 0 {
            // The decoded payload's row stride is row_bytes = ((w + 7)/8 + 3) & ~3.
            // The reference test compares against tightly-packed PBM rows of
            // width (w+7)/8 bytes. Trim the padding from each row.
            let line_bytes = w.div_ceil(8) as usize;
            let row_bytes_padded = (line_bytes + 3) & !3;
            let mut tight = Vec::with_capacity(line_bytes * h as usize);
            for y in 0..h as usize {
                tight.extend_from_slice(
                    &data[y * row_bytes_padded..y * row_bytes_padded + line_bytes],
                );
            }
            main_pages.push((w, h, tight));
        }
    }
    main_pages
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn parse_int_after(slice: &[u8], key: &[u8]) -> u32 {
    let off = find_subslice(slice, key).expect("key in PDF");
    let mut p = off + key.len();
    while p < slice.len() && slice[p] == b' ' {
        p += 1;
    }
    let start = p;
    while p < slice.len() && slice[p].is_ascii_digit() {
        p += 1;
    }
    std::str::from_utf8(&slice[start..p])
        .unwrap()
        .parse()
        .unwrap()
}
