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
    /// The CCITT bitstream ran out of bits mid-line.
    #[error("decoder bit underrun at line {y}, x={x}")]
    BitUnderrun {
        /// Row index in the image.
        y: u32,
        /// Pixel column where the underrun was detected.
        x: u32,
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
        let e = MaxError::Truncated { offset: 0x100, need: 8, have: 3 };
        assert_eq!(e.to_string(), "truncated chunk at 0x100: need 8 bytes, have 3");
    }

    #[test]
    fn bit_underrun_displays() {
        let e = MaxError::BitUnderrun { y: 305, x: 2376 };
        assert_eq!(e.to_string(), "decoder bit underrun at line 305, x=2376");
    }

    #[test]
    fn io_error_wrapping_round_trips() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let e: MaxError = io.into();
        assert!(matches!(e, MaxError::Io(_)));
        assert!(e.to_string().contains("io:"));
    }
}
