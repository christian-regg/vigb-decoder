# Python reference decoder

`max2pdf.py` — pure-Python decoder for PaperPort 2 (`.max`) image scans.
Sibling implementation to the Rust crate in this repo; same algorithm,
same canonical bit-perfect output.

## When to use this vs the Rust binary

| | Rust (`cargo install vigb-decoder`) | Python (`python max2pdf.py`) |
|---|---|---|
| Install | needs Rust toolchain (or pre-built binary) | needs Python 3 only |
| Speed | ~38 ms/page on 2464×3508 scans | ~150 ms/page (4× slower) |
| Distribution | crates.io + GitHub releases | this single file |
| Library use | `cargo add vigb-decoder` | `import max2pdf; pages = max2pdf.parse_max(...)` |

For most users the Rust binary is the path of least resistance. Use
the Python script if you can't install Rust, want to script the
decoder from existing Python code, or want to read the algorithm in a
high-level language.

## Usage

```
python max2pdf.py scan.max scan2.max -o out/
```

See `python max2pdf.py --help` for the full flag list. The CLI flags
match the Rust binary's `max2pdf` 1:1.

## License + provenance

MIT OR Apache-2.0 (matches the Rust crate). The CCITT-T.6 lookup
tables embedded in this script were copied from `../src/ccitt.rs`,
which transcribed them clean-room from CCITT-T.6 (1988) + TIFF 6.0
(Aldus 1992) PDFs. No code or table values from `paperman`
(GPL-2-or-later) is used. See `../docs/provenance.md` for the full
clean-room separation notes.

## Excluded from `cargo publish`

`python-reference/` is listed in `Cargo.toml`'s `exclude` so this
script is visible in the GitHub repo but not part of the crate
uploaded to crates.io. Crate consumers don't pay the bytes cost.
