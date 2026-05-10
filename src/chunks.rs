//! `.max` container chunk discovery.
//!
//! PaperPort 2 stores each scanned page as a DL-tagged chunk. Image chunks
//! are identified by the low 16 bits of the flags word being `0x4000`
//! (image tag) AND the high 16 bits being non-zero (page index).

/// A discovered image chunk in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct ChunkRef {
    /// Byte offset of the `b'DL'` magic in the file.
    pub offset: usize,
    /// Total length of the chunk in bytes (including the 10-byte header).
    pub length: usize,
}

/// Scan `data` for image chunks. Mirrors `max2pdf.py:find_image_chunks`.
#[allow(dead_code)]
pub(crate) fn find_image_chunks(data: &[u8]) -> Vec<ChunkRef> {
    let mut out = Vec::new();
    let n = data.len();
    if n < 8 {
        return out;
    }
    let mut pos = 0usize;
    while pos + 10 <= n {
        if &data[pos..pos + 2] == b"DL" {
            let length = u32::from_le_bytes(data[pos + 2..pos + 6].try_into().unwrap()) as usize;
            let flags = u32::from_le_bytes(data[pos + 6..pos + 10].try_into().unwrap());
            let tag = flags & 0xFFFF;
            let page_index = flags >> 16;
            if tag == 0x4000 && page_index > 0 && length > 0 && length <= n - pos {
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
        data[chunk_offset] = b'D';
        data[chunk_offset + 1] = b'L';
        data[chunk_offset + 2..chunk_offset + 6].copy_from_slice(&64u32.to_le_bytes());
        data[chunk_offset + 6..chunk_offset + 10].copy_from_slice(&0x0001_4000u32.to_le_bytes());

        let chunks = find_image_chunks(&data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].offset, chunk_offset);
        assert_eq!(chunks[0].length, 64);
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
