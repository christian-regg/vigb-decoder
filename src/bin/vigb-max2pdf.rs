//! `vigb-max2pdf` — convert PaperPort 2 (.max) files to PDF.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{ArgAction, Parser, ValueEnum};

use vigb_decoder::{decode_max_file, write_pdf, Config, DispatchKind, T0DropMode};

#[derive(Debug, Parser)]
#[command(name = "vigb-max2pdf", version, about = "Convert PaperPort 2 (.max) files to PDF")]
struct Cli {
    /// One or more .max files
    #[arg(required = true)]
    inputs: Vec<PathBuf>,

    /// Write PDFs into this directory (default: alongside each input)
    #[arg(short = 'o', long = "output-dir")]
    output_dir: Option<PathBuf>,

    /// Print per-file decode statistics
    #[arg(long)]
    stats: bool,

    // --- Canonical fixes (default ON) ---
    /// Skip embedding the preview thumbnail page
    #[arg(long = "no-preview", action = ArgAction::SetTrue)]
    no_preview: bool,

    /// Disable the canonical reference-table walk fix (diagnostic)
    #[arg(long = "no-bug4", action = ArgAction::SetTrue)]
    no_bug4: bool,

    /// Disable the strict type-0 marker gate (diagnostic)
    #[arg(long = "no-strict-t0", action = ArgAction::SetTrue)]
    no_strict_t0: bool,

    /// Keep type-3 BLANK markers that follow drift (disables 6th-session fix)
    #[arg(long, action = ArgAction::SetTrue)]
    keep_drift_blanks: bool,

    /// Keep type-1 dispatches (disables 6th-session fix)
    #[arg(long, action = ArgAction::SetTrue)]
    keep_t1_dispatches: bool,

    // --- Experimental / diagnostic ---
    /// Use byte-by-byte bit refill (diagnostic)
    #[arg(long)]
    lazy_bit_loading: bool,

    /// Reset reference table at chunk start (diagnostic vestige)
    #[arg(long)]
    t0_reset: bool,

    /// Type-0 drop-after-drift mode
    #[arg(long, value_enum, default_value_t = T0DropArg::None)]
    t0_drop_after_drift: T0DropArg,

    /// Restrict t0-drop to comma-separated dispatch kinds (e.g. "fail,v0")
    #[arg(long)]
    t0_drop_kinds: Option<String>,

    /// Bytes to scan-forward after a FAIL looking for next valid marker
    #[arg(long, default_value_t = 0)]
    fail_scan_forward: u32,

    /// In FAIL cascades, do not advance y on each FAIL
    #[arg(long)]
    suppress_t2_fail_y_in_cascade: bool,

    // --- Smart resync (10th-session) ---
    /// Smart-resync probe range ±K after isolated FAIL (0 disables)
    #[arg(long, default_value_t = 0)]
    fail_resync_max: u32,

    /// Smart-resync probe lookahead in lines
    #[arg(long, default_value_t = 5)]
    fail_resync_lookahead: u32,

    /// Smart-resync minimum confidence margin (n_ok - n_drift)
    #[arg(long, default_value_t = 0)]
    fail_resync_min_confidence: u32,

    /// Maximum total resync probes per page (0 = unlimited)
    #[arg(long, default_value_t = 0)]
    fail_resync_budget: u32,

    /// Reset reference table after a drift event
    #[arg(long)]
    reset_ref_after_drift: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum T0DropArg {
    #[value(name = "")]
    None,
    #[value(name = "marker")]
    Marker,
    #[value(name = "full")]
    Full,
}

impl From<T0DropArg> for T0DropMode {
    fn from(a: T0DropArg) -> Self {
        match a {
            T0DropArg::None => T0DropMode::None,
            T0DropArg::Marker => T0DropMode::Marker,
            T0DropArg::Full => T0DropMode::Full,
        }
    }
}

fn parse_dispatch_kinds(s: &str) -> Result<Vec<DispatchKind>, String> {
    s.split(',')
        .map(|k| {
            let k = k.trim().to_lowercase();
            match k.as_str() {
                "ok" => Ok(DispatchKind::Ok),
                "v0" => Ok(DispatchKind::V0),
                "t0" => Ok(DispatchKind::T0),
                "t1" => Ok(DispatchKind::T1),
                "fail" => Ok(DispatchKind::Fail),
                "bad" => Ok(DispatchKind::Bad),
                _ => Err(format!("unknown dispatch kind: {}", k)),
            }
        })
        .collect()
}

fn build_config(cli: &Cli) -> Result<Config, String> {
    let mut cfg = Config::builder();

    // Canonical fixes (default ON, --no-* to disable)
    cfg = cfg.embed_preview(!cli.no_preview);
    cfg = cfg.bug4(!cli.no_bug4);
    cfg = cfg.strict_t0(!cli.no_strict_t0);
    cfg = cfg.drop_blank_after_drift(!cli.keep_drift_blanks);
    cfg = cfg.suppress_t1_all(!cli.keep_t1_dispatches);

    // Experimental / diagnostic (default OFF, explicit to enable)
    cfg = cfg.lazy_bit_loading(cli.lazy_bit_loading);
    cfg = cfg.t0_reset(cli.t0_reset);
    cfg = cfg.t0_drop_after_drift(cli.t0_drop_after_drift.into());

    if let Some(ref kinds_str) = cli.t0_drop_kinds {
        let kinds = parse_dispatch_kinds(kinds_str)?;
        cfg = cfg.t0_drop_kinds(Some(kinds));
    }

    cfg = cfg.fail_scan_forward(cli.fail_scan_forward);
    cfg = cfg.suppress_t2_fail_y_in_cascade(cli.suppress_t2_fail_y_in_cascade);

    // Smart resync
    cfg = cfg.fail_resync_max(cli.fail_resync_max);
    cfg = cfg.fail_resync_lookahead(cli.fail_resync_lookahead);
    cfg = cfg.fail_resync_min_confidence(cli.fail_resync_min_confidence);
    cfg = cfg.fail_resync_budget(cli.fail_resync_budget);
    cfg = cfg.reset_ref_after_drift(cli.reset_ref_after_drift);

    Ok(cfg.build())
}

fn process_one(
    input: &std::path::Path,
    out_dir: Option<&std::path::Path>,
    cfg: &Config,
    want_stats: bool,
) -> Result<(), String> {
    let pages = decode_max_file(input, cfg).map_err(|e| {
        format!("failed to decode {}: {}", input.display(), e)
    })?;

    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let parent = out_dir
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| {
            input
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        });
    std::fs::create_dir_all(&parent).map_err(|e| {
        format!("failed to create output dir {}: {}", parent.display(), e)
    })?;
    let out_path = parent.join(format!("{stem}.pdf"));

    write_pdf(&pages, &out_path).map_err(|e| {
        format!("failed to write {}: {}", out_path.display(), e)
    })?;
    println!("{} -> {}", input.display(), out_path.display());

    if want_stats {
        for (i, p) in pages.iter().enumerate() {
            let s = &p.stats;
            println!(
                "  page {i}: {}x{} ok={} v0={} t0={} t1={} fail={} max_consec_fail={} first_fail_y={:?} resync_probes={} resync_hits={} blank_drops_drift={}",
                p.width, p.height, s.n_ok, s.n_v0, s.n_t0, s.n_t1, s.n_fail,
                s.max_consecutive_fail, s.first_fail_y, s.resync_probes, s.resync_hits,
                s.blank_drops_after_drift
            );
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let cfg = match build_config(&cli) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };

    let out_dir = cli.output_dir.as_deref();
    let mut had_error = false;

    for input in &cli.inputs {
        if let Err(e) = process_one(input, out_dir, &cfg, cli.stats) {
            eprintln!("error: {e}");
            had_error = true;
        }
    }

    if had_error {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
