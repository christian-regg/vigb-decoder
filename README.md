# vigb-decoder

[![ci](https://github.com/christian-regg/vigb-decoder/actions/workflows/ci.yml/badge.svg)](https://github.com/christian-regg/vigb-decoder/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/vigb-decoder.svg)](https://crates.io/crates/vigb-decoder)
[![docs.rs](https://docs.rs/vigb-decoder/badge.svg)](https://docs.rs/vigb-decoder)

Decoder for PaperPort 2 (`.max`) image scans.

## Why this exists

The PaperPort 2 file format ("ViGBe") is dead. The only way to open
these files used to be PaperPort 3.6 — a 1996 Windows app that doesn't
run natively on modern Windows and isn't supported by any current
PaperPort version (Tungsten / Kofax / Nuance — see
[Nuance KB 1473](https://nuance.custhelp.com/app/answers/detail/a_id/1473/)).

`vigb-decoder` is the first known tool that decodes PaperPort 2 era
files. The closest existing OSS projects ([paperman](https://github.com/sjg20/paperman)
and [max2pdf](https://github.com/orangeturtle739/max2pdf)) explicitly do
not support this format.

## Install

    cargo install vigb-decoder

This installs the `vigb-max2pdf` binary in `~/.cargo/bin/`.

Pre-built binaries for Linux x86_64, Windows x86_64, and macOS aarch64
are attached to each [release](https://github.com/christian-regg/vigb-decoder/releases).

## Use

Convert a single file:

    vigb-max2pdf scan.max

Convert a batch into a directory:

    vigb-max2pdf -o out/ *.max

Print per-file decode stats:

    vigb-max2pdf --stats scan.max

Each `.max` page also has an embedded 102×146 grayscale preview
thumbnail. By default the converter ignores it (the main bit-perfect
image is what you want). Pass `--preview` to append the thumbnail as
an extra PDF page per source page — useful as a fallback when the
main decode fails on hand-drawn content or stamps:

    vigb-max2pdf --preview scan.max

See [`docs/cli.md`](docs/cli.md) for the full flag list.

## Pure-Python alternative

If you can't install Rust, a pure-Python sibling implementation lives
at [`python-reference/vigb_max2pdf.py`](python-reference/vigb_max2pdf.py).
Same algorithm, same canonical bit-perfect output, ~4× slower. Same
CLI flags. Same MIT/Apache-2.0 license.

    python python-reference/vigb_max2pdf.py scan.max -o out/

## Library use

```rust
use vigb_decoder::{decode_max_file, write_pdf, Config, MaxError};
use std::path::Path;

fn main() -> Result<(), MaxError> {
    let pages = decode_max_file("scan.max", &Config::default())?;
    write_pdf(&pages, Path::new("scan.pdf"))?;
    Ok(())
}
```

`Page::bitmap` is 1-bit packed, MSB-first per byte. **Bit value 1 means
BLACK** (matches the PDF `/Indexed [/DeviceGray 1 <FF 00>]` convention).

## Status

Bit-perfect against the canonical PaperPort 3.6 reference on every file
we have ground truth for. Median IoU = 1.000 across a 159-page test
corpus (private — the test corpus is the author's personal document
archive).

## Format reverse-engineering

See [`docs/format.md`](docs/format.md) for the file structure and
[`docs/decoder.md`](docs/decoder.md) for the canonical decoder behaviour
(including the four canonical fixes the decoder implements).

## Reverse-engineering legal basis

This decoder was reverse-engineered for interoperability under:

- Switzerland: [URG Art. 21](https://www.fedlex.admin.ch/eli/cc/1993/1798_1798_1798/en) (decompilation for interface info).
- EU: [Software Directive 2009/24/EC Art. 6](https://eur-lex.europa.eu/legal-content/EN/TXT/PDF/?uri=CELEX:32009L0024).
- US: [DMCA §1201(f)](https://www.law.cornell.edu/uscode/text/17/1201) safe harbour and [_Sega v. Accolade_, 977 F.2d 1510 (9th Cir. 1992)](https://www.copyright.gov/fair-use/summaries/segaenters-accolade-9thcir1992.pdf).

The decoder ships zero bytes from PaperPort. CCITT-T.6 lookup tables
are derived from the [CCITT T.6 Recommendation](https://www.itu.int/rec/T-REC-T.6)
(1988, a public standard) cross-checked against the TIFF 6.0
Specification (1992, public domain); format dispatch logic was
developed against bit-traces of the author's own `.max` files
cross-checked against the disassembly of ScanSoft's `MAXKER2.DLL`
(extracted from the publicly distributed Visioneer 5.2 installer ISO,
archive.org, 2020).

See [`docs/provenance.md`](docs/provenance.md) for component-level
clean-room separation notes.

## Credits

- PaperPort 3.6 (ScanSoft, 1996) — bridge that made the RE possible.
- CCITT T.6 (1988) + TIFF 6.0 (Aldus, 1992) — source for CCITT Group 4
  table values.
- [paperman](https://github.com/sjg20/paperman) (Simon Glass) and
  [max2pdf](https://github.com/orangeturtle739/max2pdf) (orangeturtle739)
  — prior-art OSS projects, used as cross-checks during RE. Both
  GPL-2-or-later; no code is copied from either project.
- [otvdm](https://github.com/otya128/winevdm) (otya128) — runs the
  PP3.6 bridge under modern Windows.

## License

Licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT license](LICENSE-MIT)

at your option.

PaperPort is a trademark of its respective owner (Tungsten Automation,
formerly Kofax / Nuance / ScanSoft). This project is independent and
not affiliated with or endorsed by the trademark holder.
