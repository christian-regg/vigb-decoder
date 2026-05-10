# CLI flag reference

This table maps every `vigb-max2pdf` Rust CLI flag to the equivalent
flag in the Python reference decoder
([`python-reference/max2pdf.py`](../python-reference/max2pdf.py)).
Long names match exactly so muscle memory transfers.

| Rust flag | Default | What it does |
|---|---|---|
| `<inputs>` (positional) | (required) | One or more `.max` files |
| `-o`, `--output-dir` | (alongside input) | Write PDFs into this directory |
| `--stats` | off | Print per-file decode statistics |
| `--no-preview` | off | Skip embedding the preview thumbnail page |
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
| `--fail-resync-budget` | 0 | Maximum total resync probes per page (0 = unlimited) |
| `--reset-ref-after-drift` | off | Reset reference table after a drift event |
| `--keep-drift-blanks` | off | Keep type-3 BLANK markers after drift (diagnostic — disables 6th-session fix) |
| `--keep-t1-dispatches` | off | Keep type-1 dispatches (diagnostic — disables 6th-session fix) |

`--stats` is the one CLI-only addition — the Python reference computes
stats internally but doesn't have a corresponding CLI flag.

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
