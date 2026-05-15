//! Per-flag tests verifying each heuristic flag's documented behaviour.

use std::fs;
use vigb_decoder::{decode_max, Config, T0DropMode};

fn fixture() -> Vec<u8> {
    fs::read("tests/fixtures/synthetic.max").expect("read synthetic.max")
}

#[test]
fn lazy_bit_loading_matches_eager_on_synthetic() {
    let data = fixture();
    let p_eager = &decode_max(&data, &Config::default()).unwrap()[0];
    let cfg_lazy = Config::builder().lazy_bit_loading(true).build();
    let p_lazy = &decode_max(&data, &cfg_lazy).unwrap()[0];
    assert_eq!(
        p_eager.bitmap, p_lazy.bitmap,
        "lazy != eager on canonical input"
    );
}

#[test]
fn no_bug4_on_synthetic_does_not_panic() {
    let data = fixture();
    let cfg = Config::builder().bug4(false).build();
    let p = &decode_max(&data, &cfg).unwrap()[0];
    assert_eq!(p.bitmap.len() as u32, p.row_bytes * p.height);
    // Note: bug4=false MAY differ from bug4=true on the synthetic if any
    // V_R{1,2,3} runs hit the divergence path. Just assert shape, not equality.
}

#[test]
fn embed_preview_false_yields_no_preview() {
    // Note: synthetic.max has preview_size==0 so preview is None either way.
    // This test just verifies the flag doesn't break decoding.
    let data = fixture();
    let cfg = Config::builder().embed_preview(false).build();
    let p = &decode_max(&data, &cfg).unwrap()[0];
    assert!(p.preview.is_none());
}

#[test]
fn t0_drop_mode_parses() {
    use std::str::FromStr;
    assert_eq!(T0DropMode::from_str("marker").unwrap(), T0DropMode::Marker);
    assert_eq!(T0DropMode::from_str("full").unwrap(), T0DropMode::Full);
    assert_eq!(T0DropMode::from_str("none").unwrap(), T0DropMode::None);
    assert!(T0DropMode::from_str("bogus").is_err());
}

#[test]
fn smart_resync_does_not_panic() {
    // The synthetic has no FAIL events so smart resync is never invoked.
    // This test just verifies wiring is sane.
    let data = fixture();
    let cfg = Config::builder()
        .fail_resync_max(4)
        .fail_resync_lookahead(5)
        .fail_resync_min_confidence(2)
        .fail_resync_budget(10)
        .reset_ref_after_drift(true)
        .build();
    let p = &decode_max(&data, &cfg).unwrap()[0];
    assert_eq!(p.stats.resync_probes, 0); // synthetic has no FAILs to trigger probes
}
