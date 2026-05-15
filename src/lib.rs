//! Decoder for PaperPort 2 (`.max`) image scans.
//!
//! The PaperPort 2 file format ("ViGBe") is a proprietary container used
//! by ScanSoft's PaperPort 2 (1996) for 1-bit scanned documents. Each
//! image chunk wraps a CCITT-T.6 (Group 4 fax) compressed bitmap with a
//! custom per-line marker dispatcher.
//!
//! # Quick start
//!
//! ```no_run
//! use vigb_decoder::{decode_max_file, write_pdf, Config};
//! use std::path::Path;
//!
//! let pages = decode_max_file("scan.max", &Config::default())?;
//! write_pdf(&pages, Path::new("scan.pdf"))?;
//! # Ok::<(), vigb_decoder::MaxError>(())
//! ```
//!
//! # Format documentation
//!
//! See `docs/format.md` and `docs/decoder.md` in the repo for the format
//! specification and the canonical decoder behaviour.
//!
//! # Output bitmap polarity
//!
//! [`Page::bitmap`] is 1-bit packed, MSB-first per byte. **Bit value 1
//! means BLACK.** This matches the PDF `/Indexed [/DeviceGray 1 <FF 00>]`
//! convention used by [`write_pdf`]. If you're comparing against a PNG
//! ground-truth in PIL `'1'` mode, be aware that PIL `'1'` uses the
//! opposite convention (bit 1 = white) — invert before comparing.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
pub use error::{MaxError, Result};

mod config;
pub use config::{Config, ConfigBuilder, DispatchKind, T0DropMode};

mod bitstream;
mod ccitt;
mod chunks;
pub use chunks::{MAX_IMAGE_PIXELS, MAX_PREVIEW_PIXELS};

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
/// - [`MaxError::BadMagic`] if the input does not begin with the
///   `ViGBe` magic.
/// - [`MaxError::Truncated`] if no valid image chunks are found.
/// - [`MaxError::ImageTooLarge`] if any chunk's declared dimensions
///   exceed [`MAX_IMAGE_PIXELS`].
/// - [`MaxError::TooManyPages`] if the file claims more image chunks
///   than [`Config::max_pages`] allows (SEC-M04).
pub fn decode_max(data: &[u8], cfg: &Config) -> Result<Vec<Page>> {
    if data.len() < 5 || &data[..5] != b"ViGBe" {
        return Err(MaxError::BadMagic { offset: 0u64 });
    }
    let chunks = chunks::find_image_chunks(data);
    if chunks.is_empty() {
        return Err(MaxError::Truncated {
            offset: 0u64,
            need: 0x40,
            have: data.len(),
        });
    }
    // SEC-M04: bound total page count before iterating. Each Page allocates
    // up to MAX_IMAGE_PIXELS / 8 bytes (~25 MiB) and is retained in `out`
    // until this function returns; an N-chunk file can request N × 25 MiB
    // resident memory without this cap.
    if (chunks.len() as u64) > cfg.max_pages as u64 {
        return Err(MaxError::TooManyPages {
            count: chunks.len(),
            max: cfg.max_pages,
        });
    }
    let mut out = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        out.push(dispatch::decode_image_chunk(
            data,
            chunk.offset,
            chunk.length,
            cfg,
        )?);
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
