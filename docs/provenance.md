# Provenance and clean-room separation

This document records where every component of the decoder came from,
to support the MIT/Apache-2.0 license posture against any GPL-
contamination claim from prior-art OSS projects.

## Component sources

| Component | Source | Provenance |
|---|---|---|
| CCITT-T.6 lookup tables (`src/ccitt.rs`) | CCITT T.6 (1988) + TIFF 6.0 Spec (Aldus 1992) | Numerical values transcribed from public ITU/Aldus standards. NOT copied from `paperman` or `max2pdf`. Same numbers, different source. |
| Per-line decoder (`src/decoder.rs`) | Disassembly of `MAXKER2.DLL` (PaperPort 3.6, 1996) + author's own bit-traces against self-owned `.max` files | Reverse-engineered for interoperability. Algorithmic logic developed in the parallel Python research workspace; ported here from author's own Python source, not from paperman or max2pdf. |
| Per-line dispatcher (`src/dispatch.rs`) | Same as decoder | Same as decoder. |
| Chunk discovery (`src/chunks.rs`) | Bit-trace of `.max` file structure | Author's own RE; the DL-magic scan is also documented in the JustSolve wiki at http://fileformats.archiveteam.org/wiki/PaperPort_(MAX). |
| Preview RLE (`src/preview.rs`) | Bit-trace + reading paperman as cross-check | Author's own RE. paperman has the only other documented preview decoder; no code copied. |
| PDF writer (`src/pdf.rs`) | PDF 1.4 specification (Adobe) | Hand-written; PDF format itself is an Adobe specification, not encumbered. |
| Test-only encoder (`tests/common/encoder.rs`) | CCITT T.6 spec + author's RE notes | Author's own implementation of standard CCITT-T.6 encoding; tables duplicated from `src/ccitt.rs` for test-crate isolation. |

## Reverse-engineering legal basis

Reverse engineering for interoperability is permitted under:

- Switzerland: URG Art. 21 (decompilation for interface info).
- EU: Software Directive 2009/24/EC Art. 6 (RE for interoperability).
- US: DMCA §1201(f) safe harbour and *Sega v. Accolade*, 977 F.2d 1510
  (9th Cir. 1992) (disassembly for interop is fair use).

The decoder ships zero bytes from PaperPort. The `MAXKER2.DLL`
extraction used `idecomp` on InstallShield V3 `.Z` archives, which is
a publicly-known format operation — not "circumvention of a
technological measure" under DMCA §1201.

## What we deliberately did NOT do

- Did not copy any source from `paperman` (GPL-2-or-later).
- Did not copy any source from `max2pdf` Python (GPL-2-or-later).
- Did not embed `MAXKER2.DLL`, any other PaperPort binary, or any
  bytes from the Visioneer 5.2 ISO in this repository.
- Did not copy CCITT table values from `paperman_btab.dat`. The values
  in `src/ccitt.rs` came from CCITT T.6 (1988) and TIFF 6.0 (1992).

If you find any code in this repository that appears to be a 1:1 port
of paperman or max2pdf logic, please open an issue — that's a
provenance bug we want to fix.
