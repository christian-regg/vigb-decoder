//! Preview thumbnail decoder. Mirrors `max2pdf.py:840-947`.

use crate::chunks::{read_u16_at, ChunkRef, MAX_PREVIEW_PIXELS};
use crate::decoder::Preview;

/// Decode a preview RLE byte stream. Returns `(grayscale_pixels, type3_count)`.
/// Each pixel is 8 bits, `0` = white, `0xFF` = black.
pub(crate) fn decode_preview_rle(
    buf: &[u8],
    total_pixels: usize,
    max_bytes: usize,
) -> (Vec<u8>, u32) {
    let mut out = Vec::with_capacity(total_pixels);
    let mut pos = 0usize;
    let mut type3 = 0u32;
    let end = max_bytes.min(buf.len());
    while pos < end && out.len() < total_pixels {
        let ch = buf[pos];
        pos += 1;
        let type_ = ch >> 6;
        let count = (ch & 0x3F) as usize;
        match type_ {
            0 => out.extend(std::iter::repeat_n(0u8, count * 4)),
            1 => out.extend(std::iter::repeat_n(0xFFu8, count * 4)),
            2 => {
                for _ in 0..count {
                    if pos >= end {
                        break;
                    }
                    let cb = buf[pos];
                    pos += 1;
                    for j in [6, 4, 2, 0] {
                        out.push(((cb >> j) & 3) * 85);
                    }
                }
            }
            _ => type3 += 1,
        }
    }
    out.truncate(total_pixels);
    (out, type3)
}

/// Decode the preview thumbnail at the end of an image chunk and (when
/// `scale_to_a4`) upscale to the main image's pixel dimensions. Returns
/// `None` if the chunk has no preview metadata or has malformed preview
/// fields (e.g. `preview_size` larger than `chunk_length`).
///
/// `chunk_start` and `chunk_length` are expected to come from
/// [`crate::chunks::find_image_chunks`], which guarantees the chunk
/// satisfies `chunk_length >= IMAGE_CHUNK_MIN_LEN` and fits within
/// `data`. For any other caller, `read_u16_at` would return `None`
/// safely rather than panicking.
pub(crate) fn decode_preview_chunk(
    data: &[u8],
    chunk_start: usize,
    chunk_length: usize,
    scale_to_a4: bool,
) -> Option<Preview> {
    let chunk = ChunkRef { offset: chunk_start, length: chunk_length };

    let preview_size = read_u16_at(data, chunk, 0x3c)? as usize;
    let preview_x = read_u16_at(data, chunk, 0x3e)? as usize;
    let preview_y = read_u16_at(data, chunk, 0x40)? as usize;
    if preview_size == 0 || preview_x == 0 || preview_y == 0 {
        return None;
    }
    // Preview RLE lives at chunk_start + chunk_length - preview_size.
    // A malformed `preview_size > chunk_length` would underflow `usize`;
    // bail safely instead.
    if preview_size > chunk_length {
        return None;
    }
    let main_w = read_u16_at(data, chunk, 0x26)? as usize;
    let main_h = read_u16_at(data, chunk, 0x28)? as usize;

    let padded_x = (preview_x + 3) & !3;
    // SEC-H02: cap intermediate-buffer pixel count. A 6-byte preview header
    // claiming 0xFFFF × 0xFFFF would otherwise request ~4 GB of grayscale
    // pixels. Bail safely instead.
    let target_pixels = (padded_x as u64).checked_mul(preview_y as u64)?;
    if target_pixels > MAX_PREVIEW_PIXELS {
        return None;
    }
    let target_pixels = target_pixels as usize;
    let offset = chunk_start + chunk_length - preview_size;
    let (mut pixels, _type3) = decode_preview_rle(
        &data[offset..chunk_start + chunk_length],
        target_pixels,
        preview_size,
    );
    if pixels.len() < target_pixels {
        pixels.resize(target_pixels, 128);
    }

    // Vertical flip
    let mut rows: Vec<Vec<u8>> = (0..preview_y)
        .map(|i| pixels[i * padded_x..(i + 1) * padded_x].to_vec())
        .collect();
    rows.reverse();
    let flipped: Vec<u8> = rows.into_iter().flatten().collect();

    let (target_w, target_h) = if scale_to_a4 {
        (main_w, main_h)
    } else {
        (preview_x, preview_y)
    };

    let line_bytes = target_w.div_ceil(8);
    let row_bytes = (line_bytes + 3) & !3;
    let mut bitmap = vec![0u8; row_bytes * target_h];

    // Nearest-neighbor upscale + threshold at 128 -> 1-bit (1=black).
    for ty in 0..target_h {
        let sy = ty * preview_y / target_h.max(1);
        let src_row = &flipped[sy * padded_x..(sy + 1) * padded_x];
        let dst_row = &mut bitmap[ty * row_bytes..(ty + 1) * row_bytes];
        for tx in 0..target_w {
            let sx = tx * preview_x / target_w.max(1);
            if src_row[sx] >= 128 {
                dst_row[tx >> 3] |= 0x80 >> (tx & 7);
            }
        }
    }

    Some(Preview {
        width: target_w as u32,
        height: target_h as u32,
        row_bytes: row_bytes as u32,
        bitmap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rle_type0_emits_zeros() {
        // Marker 0b00_000010 = 0x02 => type=0, count=2 => 2*4 = 8 zero pixels.
        let buf = [0x02u8];
        let (out, type3) = decode_preview_rle(&buf, 8, 1);
        assert_eq!(out, vec![0u8; 8]);
        assert_eq!(type3, 0);
    }

    #[test]
    fn rle_type1_emits_ff() {
        // 0b01_000001 = 0x41 => type=1, count=1 => 4 bytes of 0xFF.
        let buf = [0x41u8];
        let (out, _) = decode_preview_rle(&buf, 4, 1);
        assert_eq!(out, vec![0xFFu8; 4]);
    }

    #[test]
    fn rle_type2_emits_grayscale_quartets() {
        // 0b10_000001 = 0x81 => type=2, count=1; followed by 1 literal byte.
        // Literal 0xC0 = 0b11_00_00_00 => pixels = (3,0,0,0)*85 = (255,0,0,0).
        let buf = [0x81u8, 0xC0];
        let (out, _) = decode_preview_rle(&buf, 4, 2);
        assert_eq!(out, vec![255, 0, 0, 0]);
    }

    #[test]
    fn rle_stops_at_total_pixels() {
        // Same type-1 marker but request only 2 pixels.
        let buf = [0x41u8];
        let (out, _) = decode_preview_rle(&buf, 2, 1);
        assert_eq!(out, vec![0xFFu8; 2]);
    }

    #[test]
    fn chunk_decoder_returns_none_when_preview_size_exceeds_chunk() {
        // Build a buffer large enough to read all header fields but with
        // a preview_size field claiming more bytes than the chunk holds.
        // Pre-fix this would underflow `chunk_start + chunk_length - preview_size`
        // and then panic on the slice index.
        let mut data = vec![0u8; 0x100];
        let chunk_start = 0;
        let chunk_length = 0x80usize;
        // preview_size at +0x3c
        data[chunk_start + 0x3c..chunk_start + 0x3e]
            .copy_from_slice(&0xFFFFu16.to_le_bytes());
        // preview_x at +0x3e
        data[chunk_start + 0x3e..chunk_start + 0x40]
            .copy_from_slice(&102u16.to_le_bytes());
        // preview_y at +0x40
        data[chunk_start + 0x40..chunk_start + 0x42]
            .copy_from_slice(&146u16.to_le_bytes());
        // width / height at +0x26 / +0x28
        data[chunk_start + 0x26..chunk_start + 0x28]
            .copy_from_slice(&16u16.to_le_bytes());
        data[chunk_start + 0x28..chunk_start + 0x2a]
            .copy_from_slice(&4u16.to_le_bytes());

        assert!(decode_preview_chunk(&data, chunk_start, chunk_length, false).is_none());
    }

    #[test]
    fn chunk_decoder_returns_none_for_undersized_chunk() {
        // chunk_length < IMAGE_CHUNK_MIN_LEN (0x42): header reads should
        // bail safely via read_u16_at returning None.
        let data = vec![0u8; 0x100];
        assert!(decode_preview_chunk(&data, 0, 0x10, false).is_none());
    }
}
