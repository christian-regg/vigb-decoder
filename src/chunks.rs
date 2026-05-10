//! `.max` container chunk discovery.
//!
//! PaperPort 2 stores each scanned page as a DL-tagged chunk. Image chunks
//! are identified by the low 16 bits of the flags word being `0x4000`
//! (image tag) AND the high 16 bits being non-zero (page index).

/// Minimum byte length required for an image chunk to contain every
/// documented header field. The image data stream starts at chunk-relative
/// offset `+0x42`; the last header field (`preview_height`) lives at
/// `+0x40..+0x42`. Chunks shorter than this cannot be safely decoded.
pub(crate) const IMAGE_CHUNK_MIN_LEN: usize = 0x42;

/// Maximum supported pixel count for the main image (`width * height`).
///
/// Set to 200 megapixels — comfortably above any realistic scanning
/// resolution (a 600 DPI A4 page is ~35 megapixels) but far below what
/// a malicious 64-byte chunk header (`width = height = 0xFFFF`,
/// ~4.3 gigapixels) could request. At 1 bit per pixel the resulting
/// bitmap allocation tops out at ~25 MB.
pub const MAX_IMAGE_PIXELS: u64 = 200 * 1024 * 1024;

/// Maximum supported pixel count for the intermediate preview buffer
/// (`padded_x * preview_height`).
///
/// Set to 16 megapixels. Real previews are 102×146 (≈ 15 thousand
/// pixels); even a generous 4096×4096 thumbnail is well within. The
/// preview buffer is 8-bit grayscale before being upscaled and
/// thresholded to the main image's 1-bit raster.
pub const MAX_PREVIEW_PIXELS: u64 = 16 * 1024 * 1024;

/// A discovered image chunk in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChunkRef {
    /// Byte offset of the `b'DL'` magic in the file.
    pub offset: usize,
    /// Total length of the chunk in bytes (including the 10-byte header).
    ///
    /// Invariant when produced by [`find_image_chunks`]:
    /// `length >= IMAGE_CHUNK_MIN_LEN` and `offset + length <= data.len()`.
    pub length: usize,
}

/// Read a `u16` LE from `data` at chunk-relative `off`. Returns `None`
/// if the read would go past the chunk's bounds. Used by the image and
/// preview decoders to access header fields without panicking on a
/// malformed (too-short) chunk; for chunks produced by
/// [`find_image_chunks`], `off + 2 <= IMAGE_CHUNK_MIN_LEN <= length` is
/// an invariant and the function never returns `None`.
pub(crate) fn read_u16_at(data: &[u8], chunk: ChunkRef, off: usize) -> Option<u16> {
    let end = off.checked_add(2)?;
    if end > chunk.length {
        return None;
    }
    let i = chunk.offset.checked_add(off)?;
    let bytes = data.get(i..i + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

/// Scan `data` for image chunks. Mirrors `max2pdf.py:find_image_chunks`.
///
/// Returned chunks satisfy two invariants:
///
/// 1. `length >= IMAGE_CHUNK_MIN_LEN` — the chunk is long enough to
///    contain every documented header field. Shorter chunks are
///    rejected to keep downstream header reads panic-free.
/// 2. `offset + length <= data.len()` — the chunk does not extend past
///    the file's end.
pub(crate) fn find_image_chunks(data: &[u8]) -> Vec<ChunkRef> {
    let mut out = Vec::new();
    let n = data.len();
    if n < 10 {
        return out;
    }
    let mut pos = 0usize;
    while pos + 10 <= n {
        if &data[pos..pos + 2] == b"DL" {
            let length =
                u32::from_le_bytes([data[pos + 2], data[pos + 3], data[pos + 4], data[pos + 5]])
                    as usize;
            let flags =
                u32::from_le_bytes([data[pos + 6], data[pos + 7], data[pos + 8], data[pos + 9]]);
            let tag = flags & 0xFFFF;
            let page_index = flags >> 16;
            if tag == 0x4000
                && page_index > 0
                && length >= IMAGE_CHUNK_MIN_LEN
                && length <= n - pos
            {
                out.push(ChunkRef { offset: pos, length });
                pos += length;
                continue;
            }
        }
        pos += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_no_chunks_in_empty_buffer() {
        let chunks = find_image_chunks(&[]);
        assert!(chunks.is_empty());
    }

    #[test]
    fn finds_no_chunks_when_magic_absent() {
        let data = vec![0u8; 256];
        let chunks = find_image_chunks(&data);
        assert!(chunks.is_empty());
    }

    #[test]
    fn finds_single_synthetic_chunk() {
        let mut data = vec![0u8; 256];
        let chunk_offset = 0x40usize;
        let length = 0x80u32; // >= IMAGE_CHUNK_MIN_LEN (0x42)
        data[chunk_offset] = b'D';
        data[chunk_offset + 1] = b'L';
        data[chunk_offset + 2..chunk_offset + 6].copy_from_slice(&length.to_le_bytes());
        data[chunk_offset + 6..chunk_offset + 10].copy_from_slice(&0x0001_4000u32.to_le_bytes());

        let chunks = find_image_chunks(&data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].offset, chunk_offset);
        assert_eq!(chunks[0].length, length as usize);
    }

    #[test]
    fn rejects_chunk_shorter_than_min_len() {
        // A DL chunk that advertises only 16 bytes — too short to contain
        // the documented header fields. Pre-fix, this would be admitted and
        // downstream header reads would panic on slice OOB.
        let mut data = vec![0u8; 256];
        let off = 0x10usize;
        data[off] = b'D';
        data[off + 1] = b'L';
        data[off + 2..off + 6].copy_from_slice(&0x10u32.to_le_bytes());
        data[off + 6..off + 10].copy_from_slice(&0x0001_4000u32.to_le_bytes());
        assert!(find_image_chunks(&data).is_empty());
    }

    #[test]
    fn rejects_chunk_at_min_len_minus_one() {
        // Boundary: 0x41 bytes is one short of the minimum.
        let mut data = vec![0u8; 256];
        let off = 0x10usize;
        data[off] = b'D';
        data[off + 1] = b'L';
        data[off + 2..off + 6]
            .copy_from_slice(&((IMAGE_CHUNK_MIN_LEN - 1) as u32).to_le_bytes());
        data[off + 6..off + 10].copy_from_slice(&0x0001_4000u32.to_le_bytes());
        assert!(find_image_chunks(&data).is_empty());
    }

    #[test]
    fn accepts_chunk_at_exact_min_len() {
        let mut data = vec![0u8; 256];
        let off = 0x10usize;
        data[off] = b'D';
        data[off + 1] = b'L';
        data[off + 2..off + 6].copy_from_slice(&(IMAGE_CHUNK_MIN_LEN as u32).to_le_bytes());
        data[off + 6..off + 10].copy_from_slice(&0x0001_4000u32.to_le_bytes());
        let chunks = find_image_chunks(&data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].length, IMAGE_CHUNK_MIN_LEN);
    }

    #[test]
    fn read_u16_at_returns_some_within_bounds() {
        let mut data = vec![0u8; 256];
        data[0x10 + 0x26] = 0xa0;
        data[0x10 + 0x27] = 0x09;
        let chunk = ChunkRef { offset: 0x10, length: IMAGE_CHUNK_MIN_LEN };
        assert_eq!(read_u16_at(&data, chunk, 0x26), Some(0x09a0));
    }

    #[test]
    fn read_u16_at_returns_none_past_chunk_length() {
        let data = vec![0u8; 256];
        // Chunk advertises 0x42 bytes; offset 0x42 would be past it.
        let chunk = ChunkRef { offset: 0, length: IMAGE_CHUNK_MIN_LEN };
        assert_eq!(read_u16_at(&data, chunk, IMAGE_CHUNK_MIN_LEN), None);
        // And a partially-out-of-range read.
        assert_eq!(read_u16_at(&data, chunk, IMAGE_CHUNK_MIN_LEN - 1), None);
    }

    #[test]
    fn skips_non_image_dl_chunks() {
        let mut data = vec![0u8; 256];
        data[0x10] = b'D';
        data[0x11] = b'L';
        data[0x12..0x16].copy_from_slice(&64u32.to_le_bytes());
        data[0x16..0x1A].copy_from_slice(&0x0001_2000u32.to_le_bytes());
        let chunks = find_image_chunks(&data);
        assert!(chunks.is_empty());
    }

    #[test]
    fn finds_two_back_to_back_chunks() {
        let mut data = vec![0u8; 512];
        for off in [0x00usize, 0x80usize] {
            data[off] = b'D';
            data[off + 1] = b'L';
            data[off + 2..off + 6].copy_from_slice(&0x80u32.to_le_bytes());
            data[off + 6..off + 10].copy_from_slice(&0x0001_4000u32.to_le_bytes());
        }
        let chunks = find_image_chunks(&data);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].offset, 0x00);
        assert_eq!(chunks[1].offset, 0x80);
    }
}
