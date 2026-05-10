//! Public-API panic-freedom tests.
//!
//! These tests exercise [`decode_max`] on adversarially-crafted byte
//! sequences that, prior to the chunk-header validation work in CRIT-01
//! / SEC-H01, would have panicked via slice OOB or `usize` underflow.
//! The contract is now: malformed input returns `Err(_)` or an empty
//! page list — never a panic.

use vigb_decoder::{decode_max, Config, MaxError};

/// Construct a byte buffer containing the `ViGBe` magic followed by a DL
/// chunk with the supplied length and flags. The buffer is padded so the
/// chunk fits within the file.
fn synth_file(chunk_length: u32, flags: u32, total_len: usize) -> Vec<u8> {
    let mut data = vec![0u8; total_len];
    data[..5].copy_from_slice(b"ViGBe");
    let chunk_off = 0x90usize;
    data[chunk_off] = b'D';
    data[chunk_off + 1] = b'L';
    data[chunk_off + 2..chunk_off + 6].copy_from_slice(&chunk_length.to_le_bytes());
    data[chunk_off + 6..chunk_off + 10].copy_from_slice(&flags.to_le_bytes());
    data
}

#[test]
fn short_chunk_does_not_panic() {
    // Image chunk advertising only 16 bytes — well below the 0x42 minimum
    // required to read every header field. Pre-fix, the dispatcher would
    // index `data[chunk_start + 0x26..0x42]` and panic on OOB.
    let data = synth_file(0x10, 0x0001_4000, 0x200);
    let cfg = Config::default();
    let result = decode_max(&data, &cfg);
    // The chunk is rejected at discovery time, so no image chunks remain
    // and decode_max surfaces Truncated.
    assert!(matches!(result, Err(MaxError::Truncated { .. })));
}

#[test]
fn chunk_at_exact_min_len_does_not_panic() {
    // Boundary case: a chunk advertising exactly IMAGE_CHUNK_MIN_LEN bytes
    // (0x42). Header fields are all readable; image data section is empty.
    // The decoder should produce a page (possibly empty / all-white) without
    // panicking.
    let data = synth_file(0x42, 0x0001_4000, 0x200);
    let cfg = Config::default();
    let _ = decode_max(&data, &cfg); // any outcome OK as long as no panic
}

#[test]
fn truncated_chunk_advertising_huge_length_does_not_panic() {
    // Chunk header advertises 1 GiB but the file is only 512 bytes. The
    // chunk discovery loop must reject it via `length <= n - pos`.
    let data = synth_file(1024 * 1024 * 1024, 0x0001_4000, 0x200);
    let cfg = Config::default();
    let result = decode_max(&data, &cfg);
    assert!(matches!(result, Err(MaxError::Truncated { .. })));
}

#[test]
fn no_dl_magic_returns_truncated_not_panic() {
    let mut data = vec![0u8; 0x100];
    data[..5].copy_from_slice(b"ViGBe");
    let cfg = Config::default();
    assert!(matches!(decode_max(&data, &cfg), Err(MaxError::Truncated { .. })));
}

#[test]
fn bad_magic_returns_bad_magic_not_panic() {
    let data = vec![0u8; 0x100]; // all zeros, no magic
    let cfg = Config::default();
    assert!(matches!(decode_max(&data, &Config::default()), Err(MaxError::BadMagic { .. })));
    let _ = cfg;
}

#[test]
fn empty_input_returns_bad_magic_not_panic() {
    assert!(matches!(decode_max(&[], &Config::default()), Err(MaxError::BadMagic { .. })));
}

/// Build a minimal `.max` containing one image chunk with the supplied
/// width and height fields, but no actual image data. Used to exercise
/// the dimension cap without paying for a full encode.
fn synth_file_with_dimensions(width: u16, height: u16) -> Vec<u8> {
    let mut data = vec![0u8; 0x200];
    data[..5].copy_from_slice(b"ViGBe");
    let chunk_off = 0x90usize;
    let chunk_len = 0x70u32; // > IMAGE_CHUNK_MIN_LEN, leaves a tiny image-data area
    data[chunk_off] = b'D';
    data[chunk_off + 1] = b'L';
    data[chunk_off + 2..chunk_off + 6].copy_from_slice(&chunk_len.to_le_bytes());
    data[chunk_off + 6..chunk_off + 10].copy_from_slice(&0x0001_4000u32.to_le_bytes());
    data[chunk_off + 0x26..chunk_off + 0x28].copy_from_slice(&width.to_le_bytes());
    data[chunk_off + 0x28..chunk_off + 0x2a].copy_from_slice(&height.to_le_bytes());
    data
}

#[test]
fn pathological_dimensions_return_image_too_large() {
    // 65535 × 65535 pixels would request ~537 MB of bitmap from a
    // 64-byte header. CRIT-02 cap rejects this as ImageTooLarge.
    let data = synth_file_with_dimensions(0xFFFF, 0xFFFF);
    let result = decode_max(&data, &Config::default());
    assert!(
        matches!(result, Err(MaxError::ImageTooLarge { .. })),
        "expected ImageTooLarge, got {:?}",
        result
    );
}

#[test]
fn just_over_cap_returns_image_too_large() {
    // The cap is 200 megapixels. 16384 × 16384 = 268 million > cap.
    let data = synth_file_with_dimensions(16384, 16384);
    assert!(matches!(
        decode_max(&data, &Config::default()),
        Err(MaxError::ImageTooLarge { .. })
    ));
}

#[test]
fn realistic_dimensions_pass_cap() {
    // A4 at 300 DPI (typical PaperPort scan): 2464 × 3508 ≈ 8.6 MP.
    // Well within the cap; should not be rejected for size reasons.
    let data = synth_file_with_dimensions(2464, 3508);
    let result = decode_max(&data, &Config::default());
    // Either Ok or some non-ImageTooLarge error (the synthetic chunk has
    // no image data so the result is undefined-but-non-panicking).
    assert!(
        !matches!(result, Err(MaxError::ImageTooLarge { .. })),
        "realistic A4-300dpi dimensions wrongly rejected as too large"
    );
}

#[test]
fn pathological_preview_dimensions_skip_preview_no_panic() {
    // Construct a chunk with a tiny image but pathological preview
    // dimensions. The preview decoder bails (returns None) instead of
    // allocating ~4 GB; the main image still decodes.
    let mut data = synth_file_with_dimensions(8, 4);
    let chunk_off = 0x90usize;
    // preview_size at +0x3c (must be > 0 to attempt preview decode)
    data[chunk_off + 0x3c..chunk_off + 0x3e].copy_from_slice(&16u16.to_le_bytes());
    // preview_x at +0x3e
    data[chunk_off + 0x3e..chunk_off + 0x40].copy_from_slice(&0xFFFFu16.to_le_bytes());
    // preview_y at +0x40
    data[chunk_off + 0x40..chunk_off + 0x42].copy_from_slice(&0xFFFFu16.to_le_bytes());
    let result = decode_max(&data, &Config::default());
    // Page should decode (no image data + preview gracefully None);
    // crucially, no panic and no OOM.
    if let Ok(pages) = result {
        assert_eq!(pages.len(), 1);
        assert!(pages[0].preview.is_none());
    }
}
