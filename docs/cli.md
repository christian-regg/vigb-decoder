# CLI flag reference

This table maps every `vigb-max2pdf` Rust CLI flag to the equivalent
flag in the Python reference decoder
([`python-reference/vigb_max2pdf.py`](../python-reference/vigb_max2pdf.py)).
Long names match exactly so muscle memory transfers.

| Rust flag | Default | What it does |
|---|---|---|
| `<inputs>` (positional) | (required) | One or more `.max` files |
| `-o`, `--output-dir` | (alongside input) | Write PDFs into this directory |
| `--stats` | off | Print per-file decode statistics |
| `--preview` | off | Append the embedded 102×146 preview thumbnail as an extra PDF page per source page (off by default — useful for recovering layout when the main CCITT decode fails on hand-drawn content or stamps) |
| `--no-bug4` | off | Disable canonical reference-table walk fix |
| `--no-strict-t0` | off | Disable strict type-0 marker gate |
| `--lazy-bit-loading` | off | Use byte-by-byte bit refill (diagnostic) |
| `--t0-reset` | off | Reset reference table at chunk start (vestigial) |
| `--t0-drop-after-drift` | none | Type-0 drop-after-drift mode (none/marker/full) |
| `--t0-drop-kinds` | (none) | Restrict t0-drop to comma-separated dispatch kinds (e.g. "fail,v0") |
| `--fail-scan-forward` | 0 | Bytes to scan-forward after a FAIL looking for next valid marker |
| `--suppress-t2-fail-y-in-cascade` | off | In FAIL cascades, do not advance y on each FAIL |
| `--fail-resync-max` | 0 | Smart-resync probe range ±K after isolated FAIL (0 disables) |
| `--fail-resync-lookahead` | 5 | Smart-resync probe lookahead in lines |
| `--fail-resync-min-confidence` | 0 | Minimum confidence margin (n_ok - n_drift) |
| `--fail-resync-budget` | 0 | Maximum total resync probes per page (Rust: 0 = safe default cap of 1024; Python: 0 = unlimited) |
| `--reset-ref-after-drift` | off | Reset reference table after a drift event |
| `--keep-drift-blanks` | off | Keep type-3 BLANK markers after drift (diagnostic — disables 6th-session fix) |
| `--keep-t1-dispatches` | off | Keep type-1 dispatches (diagnostic — disables 6th-session fix) |
| `--max-pages` | 1024 | **Rust-only.** Maximum image-chunk count accepted per file (SEC-M04 cap). Files claiming more chunks are rejected before decode. Raise for legitimate large scanned collections; lower for service deployments. |

Two flags are Rust-only: `--stats` (the Python reference computes
stats internally but doesn't expose a flag) and `--max-pages`
(SEC-M04 hardening with no Python analogue, matching the SEC-M02
pattern). Every other flag exists in both CLIs with the same long
name.

## Examples

Convert one file with default settings:

    vigb-max2pdf scan.max

Convert several files into a directory, with stats:

    vigb-max2pdf -o out/ --stats *.max

Diagnose a problematic file (turn off canonical fixes one at a time):

    vigb-max2pdf bad.max --no-bug4
    vigb-max2pdf bad.max --no-strict-t0

Try smart resync on a file with FAIL events:

    vigb-max2pdf bad.max --fail-resync-max 4 --reset-ref-after-drift --fail-resync-min-confidence 2

## Security notes

The decoder itself bounds work and memory on adversarial `.max` content
(SEC-M01..M04 in `src/dispatch.rs` and `src/lib.rs`). The CLI's path
handling is **not** sandboxed:

- The `-o` value is honoured verbatim. `-o ../somewhere` or absolute
  paths like `-o /etc/foo/` resolve as written; the binary will
  `create_dir_all` and write into the resulting directory.
- The output filename is `<input.file_stem()>.pdf`. `file_stem` strips
  every path component except the basename, so a crafted input path
  cannot inject `..` segments into the *output* filename — only the
  output *directory* (taken from `-o`) controls where the file lands.

For interactive use this is the expected, useful behaviour. **If you
wrap `vigb-max2pdf` in a service that exposes `-o` (or the working
directory) to an untrusted caller, canonicalize and enforce
containment yourself before invoking the binary.** Sketch:

```rust
use std::path::{Path, PathBuf};

fn safe_output_dir(user_request: &Path, allowed_root: &Path) -> std::io::Result<PathBuf> {
    let absolute = allowed_root.join(user_request);
    let canonical = absolute.canonicalize()?;
    if !canonical.starts_with(allowed_root.canonicalize()?) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "output directory escapes allowed root",
        ));
    }
    Ok(canonical)
}
```

The shell equivalent is `realpath --relative-to=<root>` plus a prefix
check.
