# Credits

This decoder builds on prior reverse-engineering and standardization work.

## Reverse-engineering bridge

PaperPort 3.6 (ScanSoft, 1996) — the canonical implementation matched
here is `MAXKER2.DLL` from PaperPort 3.6, extracted from the Visioneer
Deluxe 5.2 installer ISO (publicly hosted on archive.org since
[2020-03](https://archive.org/details/PaperPort_Deluxe_Visioneer_Version_5.2_1997)).

[otvdm/winevdm](https://github.com/otya128/winevdm) (otya128) — runs
the 16-bit / Win9x PaperPort 3.6 binary on modern Windows, used as the
test oracle during decoder development.

## Standards

CCITT Recommendation T.6 (1988) — Facsimile Coding Schemes for Group 4
Apparatus. Source for all CCITT-T.6 lookup tables in `src/ccitt.rs`.
Free PDF at https://www.itu.int/rec/T-REC-T.6.

TIFF 6.0 Specification (Aldus Corporation, 1992, public domain) —
cross-reference for the Group 4 fax tables. https://www.itu.int/itudoc/itu-t/com16/tiff-fx/docs/tiff6.pdf

## Prior-art OSS projects

These projects implement partial PaperPort decoders. They are
**GPL-2-or-later** and no code from either project is copied into this
crate (see `provenance.md`); they were used only as cross-checks
during reverse engineering.

- [paperman](https://github.com/sjg20/paperman) — Java PaperPort browser
  by Simon Glass. Active. Does not support PaperPort 2 era files.
- [max2pdf](https://github.com/orangeturtle739/max2pdf) — Python
  PaperPort-to-PDF by orangeturtle739. Dormant. Does not support
  PaperPort 2 era files.

## Author

Christian Regg, 2026. Reverse engineering done over 12 sessions in a
parallel Python research workspace; this Rust crate is the production
artifact of that work.
