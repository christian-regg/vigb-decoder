//! Decoder for PaperPort 2 (.max) image scans.
//!
//! The PaperPort 2 file format ("ViGBe") is a proprietary container used by
//! ScanSoft's PaperPort 2 (1996) for 1-bit scanned documents. Each image
//! chunk wraps a CCITT-T.6 (Group 4 fax) compressed bitmap with a custom
//! per-line marker dispatcher.
//!
//! See `docs/format.md` and `docs/decoder.md` in this repo for the format
//! specification and the canonical decoder behaviour.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
pub use error::{MaxError, Result};

mod config;
pub use config::{Config, ConfigBuilder, DispatchKind, T0DropMode};

mod bitstream;
mod ccitt;
mod chunks;
mod decoder;
mod dispatch;
mod pdf;
mod preview;

pub use decoder::{DecodeStats, Page, Preview};
pub use pdf::{write_pdf, write_pdf_bytes, PdfOptions};

/// Decode all image chunks in a `.max` byte buffer.
///
/// Returns one [`Page`] per image chunk in document order.
///
/// # Errors
///
/// Returns [`MaxError::BadMagic`] if the input does not begin with the
/// `ViGBe` magic. Returns [`MaxError::Truncated`] if no valid image
/// chunks are found in the file.
pub fn decode_max(data: &[u8], cfg: &Config) -> Result<Vec<Page>> {
    if data.len() < 5 || &data[..5] != b"ViGBe" {
        return Err(MaxError::BadMagic { offset: 0u64 });
    }
    let chunks = chunks::find_image_chunks(data);
    if chunks.is_empty() {
        return Err(MaxError::Truncated { offset: 0u64, need: 0x40, have: data.len() });
    }
    let mut out = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        out.push(dispatch::decode_image_chunk(data, chunk.offset, chunk.length, cfg));
    }
    Ok(out)
}

/// Decode a `.max` file from disk. Convenience wrapper for [`decode_max`].
///
/// # Errors
///
/// Returns an I/O error (wrapped as [`MaxError`]) if the file cannot be read,
/// or any error that [`decode_max`] returns.
pub fn decode_max_file<P: AsRef<std::path::Path>>(path: P, cfg: &Config) -> Result<Vec<Page>> {
    let data = std::fs::read(path)?;
    decode_max(&data, cfg)
}
