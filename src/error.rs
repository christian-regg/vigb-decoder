//! Error types for the vigb-decoder crate.

use std::result::Result as StdResult;

/// Errors returned by the .max decoder.
///
/// Hard errors (file-level) — soft errors (per-line decode failures) are
/// reported via `crate::DecodeStats`, never as `Err`.
#[derive(Debug, thiserror::Error)]
pub enum MaxError {
    /// File magic check failed; the input is not a ViGBe `.max` file.
    #[error("not a ViGB file: bad magic at offset {offset:#x}")]
    BadMagic {
        /// Byte offset where magic was expected.
        offset: u64,
    },
    /// A chunk header claims more bytes than remain in the file.
    #[error("truncated chunk at {offset:#x}: need {need} bytes, have {have}")]
    Truncated {
        /// Chunk start offset.
        offset: u64,
        /// Bytes the chunk header advertised.
        need: usize,
        /// Bytes actually available from this offset to EOF.
        have: usize,
    },
    /// An image chunk's declared dimensions would require an unreasonably
    /// large allocation. Crafted `.max` files can claim `width = height =
    /// 65535`, which would request hundreds of MB from a 64-byte header.
    /// The decoder rejects any image whose `width * height` exceeds
    /// [`crate::MAX_IMAGE_PIXELS`].
    #[error(
        "image dimensions {width}x{height} exceed maximum supported size \
         ({pixels} pixels, max {max} pixels)"
    )]
    ImageTooLarge {
        /// Declared width in pixels.
        width: u32,
        /// Declared height in pixels.
        height: u32,
        /// Computed pixel count (`width * height`), saturating on overflow.
        pixels: u64,
        /// The configured maximum allowed pixel count.
        max: u64,
    },
    /// Underlying IO error.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience alias: `Result<T, MaxError>`.
pub type Result<T> = StdResult<T, MaxError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_magic_displays() {
        let e = MaxError::BadMagic { offset: 0x42 };
        assert_eq!(e.to_string(), "not a ViGB file: bad magic at offset 0x42");
    }

    #[test]
    fn truncated_displays() {
        let e = MaxError::Truncated {
            offset: 0x100,
            need: 8,
            have: 3,
        };
        assert_eq!(
            e.to_string(),
            "truncated chunk at 0x100: need 8 bytes, have 3"
        );
    }

    #[test]
    fn image_too_large_displays() {
        let e = MaxError::ImageTooLarge {
            width: 65535,
            height: 65535,
            pixels: 4_294_836_225,
            max: 200 * 1024 * 1024,
        };
        let s = e.to_string();
        assert!(s.contains("65535x65535"));
        assert!(s.contains("4294836225 pixels"));
    }

    #[test]
    fn io_error_wrapping_round_trips() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let e: MaxError = io.into();
        assert!(matches!(e, MaxError::Io(_)));
        assert!(e.to_string().contains("io:"));
    }
}
