//! Decoder configuration (canonical defaults + heuristic flags).

use std::str::FromStr;

/// Behaviour for the type-0 marker `t0_drop_after_drift` heuristic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum T0DropMode {
    /// No type-0 drops (default).
    #[default]
    None,
    /// Drop the marker byte only.
    Marker,
    /// Drop the marker byte plus its declared payload bytes.
    Full,
}

impl FromStr for T0DropMode {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "" | "none" => Ok(T0DropMode::None),
            "marker" => Ok(T0DropMode::Marker),
            "full" => Ok(T0DropMode::Full),
            other => Err(format!("invalid t0-drop mode: {other}")),
        }
    }
}

/// Per-line dispatch outcome kinds (used by `t0_drop_kinds` filter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchKind {
    /// Successful type-2 decode.
    Ok,
    /// Type-2 V(0)-only decode (single vertical code consumed).
    V0,
    /// Type-0 marker (raw or skip).
    T0,
    /// Type-1 marker (single-pixel positions).
    T1,
    /// Type-2 decode failed (FAIL).
    Fail,
    /// Marker byte that didn't match a known type.
    Bad,
}

/// Decoder configuration.
///
/// `Config::default()` produces canonical behaviour (matches the Python
/// `python-reference/vigb_max2pdf.py` defaults at corpus median IoU = 1.000). All other fields
/// are diagnostic or experimental — leave them as default unless you know
/// what you're flipping.
///
/// Marked `#[non_exhaustive]`: out-of-crate callers must construct via
/// [`Config::default`] or [`Config::builder`], not struct-literal syntax.
/// Allows new fields to be added without a semver-breaking change.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Config {
    // --- Canonical fixes (default ON) ---
    /// 12th-session canonical reference-table walk. Default true.
    pub bug4: bool,
    /// 11th-session strict type-0 marker gate (only low6==1 raw, low6==3
    /// skip — drop everything else). Default true.
    pub strict_t0: bool,
    /// 6th-session: drop type-3 BLANK markers that follow a non-OK
    /// dispatch (sync-drift recovery). Default true.
    pub drop_blank_after_drift: bool,
    /// 6th-session: suppress all type-1 dispatches (99% sync-drift in
    /// ViGBe corpus). Default true.
    pub suppress_t1_all: bool,
    /// Embed the 102×146 grayscale preview thumbnail as a second PDF
    /// page per scanned page. Default **false**. Set true to recover
    /// chunk-encoded layout (hand-drawn content, stamps, regions where
    /// the CCITT path fails to decode) at the cost of one extra
    /// upscaled-thumbnail page per source page in the output.
    pub embed_preview: bool,

    // --- Experimental / diagnostic (default OFF) ---
    /// 11th-session lazy bit loading (byte-by-byte refill). Diagnostic.
    pub lazy_bit_loading: bool,
    /// Reset reference table after each chunk. Diagnostic.
    pub t0_reset: bool,
    /// `t0_drop_after_drift` mode (None | Marker | Full).
    pub t0_drop_after_drift: T0DropMode,
    /// Optional: only apply t0 drop after drift for these dispatch kinds.
    pub t0_drop_kinds: Option<Vec<DispatchKind>>,
    /// Bytes to scan-forward after a FAIL looking for next valid marker.
    pub fail_scan_forward: u32,
    /// 7th-session: in cascade FAIL runs, do not advance y on each FAIL.
    pub suppress_t2_fail_y_in_cascade: bool,

    // --- Smart resync (10th-session) ---
    /// Search range ±K for resync probe after isolated FAIL. 0 disables.
    pub fail_resync_max: u32,
    /// Probe lookahead in lines. Default 5.
    pub fail_resync_lookahead: u32,
    /// Minimum (n_ok - n_drift) margin to accept a resync candidate.
    pub fail_resync_min_confidence: u32,
    /// Maximum total resync probes per page. `0` means "use the safe
    /// default cap" (currently 1024); any value above the cap is clamped
    /// to the cap. This bounds worst-case dispatcher work on malformed
    /// input (SEC-M02 mitigation). The Python reference at
    /// `python-reference/vigb_max2pdf.py` treats `0` as truly unlimited
    /// (`float('inf')`) — a documented Rust-side hardening, not a parity
    /// bug. Bit-perfect decode of the canonical corpus is unaffected.
    pub fail_resync_budget: u32,
    /// Reset reference table to all-white after a drift event.
    pub reset_ref_after_drift: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bug4: true,
            strict_t0: true,
            drop_blank_after_drift: true,
            suppress_t1_all: true,
            embed_preview: false,
            lazy_bit_loading: false,
            t0_reset: false,
            t0_drop_after_drift: T0DropMode::None,
            t0_drop_kinds: None,
            fail_scan_forward: 0,
            suppress_t2_fail_y_in_cascade: false,
            fail_resync_max: 0,
            fail_resync_lookahead: 5,
            fail_resync_min_confidence: 0,
            fail_resync_budget: 0,
            reset_ref_after_drift: false,
        }
    }
}

impl Config {
    /// Start building a custom Config (defaults to canonical).
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder { inner: Self::default() }
    }
}

/// Fluent builder for `Config`.
pub struct ConfigBuilder { inner: Config }

macro_rules! setter {
    ($field:ident, $type:ty) => {
        /// Set the corresponding `Config` field.
        pub fn $field(mut self, value: $type) -> Self {
            self.inner.$field = value;
            self
        }
    };
}

impl ConfigBuilder {
    setter!(bug4, bool);
    setter!(strict_t0, bool);
    setter!(drop_blank_after_drift, bool);
    setter!(suppress_t1_all, bool);
    setter!(embed_preview, bool);
    setter!(lazy_bit_loading, bool);
    setter!(t0_reset, bool);
    setter!(t0_drop_after_drift, T0DropMode);
    setter!(t0_drop_kinds, Option<Vec<DispatchKind>>);
    setter!(fail_scan_forward, u32);
    setter!(suppress_t2_fail_y_in_cascade, bool);
    setter!(fail_resync_max, u32);
    setter!(fail_resync_lookahead, u32);
    setter!(fail_resync_min_confidence, u32);
    setter!(fail_resync_budget, u32);
    setter!(reset_ref_after_drift, bool);

    /// Finalize the configuration.
    pub fn build(self) -> Config { self.inner }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_canonical() {
        let c = Config::default();
        // Canonical fixes: ON
        assert!(c.bug4);
        assert!(c.strict_t0);
        assert!(c.drop_blank_after_drift);
        assert!(c.suppress_t1_all);
        // Recovery features: OFF (preview was on by default in 0.0.x when
        // the main image often failed; main image is bit-perfect now).
        assert!(!c.embed_preview);
        // Diagnostic / experimental flags: OFF
        assert!(!c.lazy_bit_loading);
        assert!(!c.t0_reset);
        assert_eq!(c.t0_drop_after_drift, T0DropMode::None);
        assert!(c.t0_drop_kinds.is_none());
        assert_eq!(c.fail_scan_forward, 0);
        assert!(!c.suppress_t2_fail_y_in_cascade);
        assert_eq!(c.fail_resync_max, 0);
        assert_eq!(c.fail_resync_lookahead, 5);
        assert_eq!(c.fail_resync_min_confidence, 0);
        assert_eq!(c.fail_resync_budget, 0);
        assert!(!c.reset_ref_after_drift);
    }

    #[test]
    fn builder_round_trip() {
        let c = Config::builder()
            .bug4(false)
            .fail_resync_max(4)
            .reset_ref_after_drift(true)
            .build();
        assert!(!c.bug4);
        assert_eq!(c.fail_resync_max, 4);
        assert!(c.reset_ref_after_drift);
        // Untouched fields keep defaults
        assert!(c.strict_t0);
    }

    #[test]
    fn t0_drop_mode_parsing() {
        assert_eq!("none".parse::<T0DropMode>().unwrap(), T0DropMode::None);
        assert_eq!("marker".parse::<T0DropMode>().unwrap(), T0DropMode::Marker);
        assert_eq!("full".parse::<T0DropMode>().unwrap(), T0DropMode::Full);
        assert!("bogus".parse::<T0DropMode>().is_err());
    }
}
