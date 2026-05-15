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
    assert!(matches!(
        decode_max(&data, &cfg),
        Err(MaxError::Truncated { .. })
    ));
}

#[test]
fn bad_magic_returns_bad_magic_not_panic() {
    let data = vec![0u8; 0x100]; // all zeros, no magic
    let cfg = Config::default();
    assert!(matches!(
        decode_max(&data, &Config::default()),
        Err(MaxError::BadMagic { .. })
    ));
    let _ = cfg;
}

#[test]
fn empty_input_returns_bad_magic_not_panic() {
    assert!(matches!(
        decode_max(&[], &Config::default()),
        Err(MaxError::BadMagic { .. })
    ));
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
fn zero_consume_type2_fails_terminate_in_bounded_time() {
    // SEC-M01: the worst case for type-2 dispatch is a payload where the
    // CCITT inner decoder fails immediately (consumed_bits = 0). Top-7 bits
    // = 0b0000000 (i.e., bytes 0x00 or 0x01) have no TAB7 match in this
    // implementation — `peek(7)` returns None, the inner decoder returns a
    // zero-bit FAIL.
    //
    // The forward-progress invariant documented in `decode_image_chunk`
    // guarantees that even in this case `pos` advances by at least 1 byte
    // per loop iteration (the marker consume). The total work is therefore
    // bounded at O(chunk_length).
    //
    // This test constructs a ~16 KB chunk filled with `0x80 0x00` pairs
    // (each pair: type-2 marker + bytes that fail TAB7) and verifies the
    // call returns instead of hanging. We use a generous time budget — if
    // forward progress regresses, this test would hang the whole test
    // binary, not merely fail.
    use std::time::Instant;

    let chunk_off = 0x90usize;
    let chunk_len = 0x4000usize; // 16 KiB chunk
    let total_len = chunk_off + chunk_len + 0x10;
    let mut data = vec![0u8; total_len];
    data[..5].copy_from_slice(b"ViGBe");
    data[chunk_off] = b'D';
    data[chunk_off + 1] = b'L';
    data[chunk_off + 2..chunk_off + 6].copy_from_slice(&(chunk_len as u32).to_le_bytes());
    data[chunk_off + 6..chunk_off + 10].copy_from_slice(&0x0001_4000u32.to_le_bytes());
    // Width = 8, height = chunk_len so the dispatcher tries to fill many
    // rows. Stays well under the megapixel cap (8 * 16384 = 131k pixels).
    data[chunk_off + 0x26..chunk_off + 0x28].copy_from_slice(&8u16.to_le_bytes());
    data[chunk_off + 0x28..chunk_off + 0x2a].copy_from_slice(&(chunk_len as u16).to_le_bytes());
    // Body at chunk_off + 0x42: alternate 0x80 (type-2 marker) and 0x00
    // (bytes whose top-7 prefix is 0b0000000 = no TAB7 match).
    let body_start = chunk_off + 0x42;
    let body_end = chunk_off + chunk_len;
    for (i, off) in (body_start..body_end).enumerate() {
        data[off] = if i % 2 == 0 { 0x80 } else { 0x00 };
    }

    let start = Instant::now();
    let _ = decode_max(&data, &Config::default());
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 5,
        "decode_max took {:?} on zero-consume FAIL pattern; \
         forward-progress invariant may have regressed",
        elapsed
    );
}

#[test]
fn pathological_resync_config_does_not_hang() {
    // SEC-M02: a `Config` with `fail_resync_max = u32::MAX` and
    // `fail_resync_lookahead = u32::MAX` would, pre-cap, run
    // `(2K + 1) * lookahead` CCITT decode calls per FAIL event —
    // ~16 quintillion iterations for a single isolated FAIL.
    //
    // Trigger: a chunk whose body is `[0xC0, 0x80, 0x00, 0x00, ...]`.
    // The 0xC0 is a type-3 BLANK marker that sets last_kind = Ok. The
    // following 0x80 is a type-2 marker; bytes 0x00 0x00 fail TAB7
    // (top-7 prefix `0b0000000` has no entry), producing an isolated
    // FAIL whose `prev_kind = Ok` opens the resync gate.
    //
    // With the SEC-M02 cap (MAX_RESYNC_K = 32, MAX_RESYNC_LOOKAHEAD =
    // 64), the resync probe completes in well under a second.
    use std::time::Instant;

    let mut data = vec![0u8; 0x200];
    data[..5].copy_from_slice(b"ViGBe");
    let chunk_off = 0x90usize;
    let chunk_len = 0x80u32;
    data[chunk_off] = b'D';
    data[chunk_off + 1] = b'L';
    data[chunk_off + 2..chunk_off + 6].copy_from_slice(&chunk_len.to_le_bytes());
    data[chunk_off + 6..chunk_off + 10].copy_from_slice(&0x0001_4000u32.to_le_bytes());
    data[chunk_off + 0x26..chunk_off + 0x28].copy_from_slice(&8u16.to_le_bytes());
    data[chunk_off + 0x28..chunk_off + 0x2a].copy_from_slice(&2u16.to_le_bytes());
    let body = chunk_off + 0x42;
    data[body] = 0xC0; // BLANK 1 line — sets last_kind = Ok
    data[body + 1] = 0x80; // type-2 marker
                           // bytes that fail TAB7 — top-7 prefix 0b0000000
    data[body + 2] = 0x00;
    data[body + 3] = 0x00;

    let cfg = Config::builder()
        .fail_resync_max(u32::MAX)
        .fail_resync_lookahead(u32::MAX)
        .fail_resync_budget(u32::MAX)
        .build();

    let start = Instant::now();
    let _ = decode_max(&data, &cfg);
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 5,
        "decode_max took {:?} with pathological resync config; \
         SEC-M02 cap may have regressed",
        elapsed
    );
}

#[test]
fn many_back_to_back_minimum_chunks_terminate_in_bounded_time() {
    // SEC-M03: a malformed file can pack many minimum-size (0x42-byte)
    // image chunks back-to-back. Pre-fix, the per-chunk dispatcher was
    // bounded by `data.len()` instead of `chunk_start + chunk_length`,
    // so the dispatch loop scanned into every later chunk's header bytes
    // interpreting them as stray markers. Total work was O(N²) in the
    // chunk count, contradicting the documented O(chunk_length) bound.
    //
    // Post-fix, each chunk's loop is bounded to its own length. With
    // every chunk at the 0x42 minimum the body is empty, so the loop
    // exits immediately and per-chunk work is O(1).
    use std::time::Instant;

    let n_chunks = 2000usize;
    let chunk_len = 0x42u32;
    let total_len = 0x90 + n_chunks * chunk_len as usize + 0x10;
    let mut data = vec![0u8; total_len];
    data[..5].copy_from_slice(b"ViGBe");
    for i in 0..n_chunks {
        let off = 0x90 + i * chunk_len as usize;
        data[off] = b'D';
        data[off + 1] = b'L';
        data[off + 2..off + 6].copy_from_slice(&chunk_len.to_le_bytes());
        // flags: low16 = 0x4000 (image tag), high16 = page_index (must be > 0
        // for find_image_chunks to admit the chunk).
        let flags = 0x4000u32 | (((i as u32) + 1) << 16);
        data[off + 6..off + 10].copy_from_slice(&flags.to_le_bytes());
        // Minimal dimensions: width=8, height=1. Keeps per-chunk bitmap
        // allocation tiny (4 bytes) and ensures the post-fix loop exits
        // before any real work.
        data[off + 0x26..off + 0x28].copy_from_slice(&8u16.to_le_bytes());
        data[off + 0x28..off + 0x2a].copy_from_slice(&1u16.to_le_bytes());
    }

    // 2000 chunks exceeds the SEC-M04 page-count cap (default 1024), so we
    // raise it explicitly here — this test exercises the SEC-M03 dispatcher
    // bound, not the SEC-M04 page cap.
    let cfg = Config::builder().max_pages(u32::MAX).build();
    let start = Instant::now();
    let _ = decode_max(&data, &cfg);
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 5,
        "decode_max took {:?} on {} back-to-back 0x42-byte chunks; \
         SEC-M03 dispatcher bound may have regressed",
        elapsed,
        n_chunks
    );
}

#[test]
fn page_count_above_cap_returns_too_many_pages() {
    // SEC-M04: a file claiming more image chunks than Config::max_pages is
    // rejected before decoding any chunk, defending against per-chunk
    // memory amplification (each page allocates up to ~25 MiB and is
    // retained until decode_max returns).
    let n_chunks = 1025usize; // just over the default cap of 1024
    let chunk_len = 0x42u32;
    let total_len = 0x90 + n_chunks * chunk_len as usize + 0x10;
    let mut data = vec![0u8; total_len];
    data[..5].copy_from_slice(b"ViGBe");
    for i in 0..n_chunks {
        let off = 0x90 + i * chunk_len as usize;
        data[off] = b'D';
        data[off + 1] = b'L';
        data[off + 2..off + 6].copy_from_slice(&chunk_len.to_le_bytes());
        let flags = 0x4000u32 | (((i as u32) + 1) << 16);
        data[off + 6..off + 10].copy_from_slice(&flags.to_le_bytes());
        data[off + 0x26..off + 0x28].copy_from_slice(&8u16.to_le_bytes());
        data[off + 0x28..off + 0x2a].copy_from_slice(&1u16.to_le_bytes());
    }

    let result = decode_max(&data, &Config::default());
    assert!(
        matches!(
            result,
            Err(MaxError::TooManyPages {
                count: 1025,
                max: 1024
            })
        ),
        "expected TooManyPages, got {:?}",
        result
    );

    // Same file under an explicitly raised cap decodes (or fails for other
    // benign reasons) — proves the cap is configurable, not a hard wall.
    let cfg = Config::builder().max_pages(u32::MAX).build();
    let result2 = decode_max(&data, &cfg);
    assert!(
        !matches!(result2, Err(MaxError::TooManyPages { .. })),
        "raised cap should not trigger TooManyPages"
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
