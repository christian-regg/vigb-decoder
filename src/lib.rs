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
