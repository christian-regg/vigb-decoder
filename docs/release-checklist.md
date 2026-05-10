# v0.1 release checklist

## Pre-release

- [ ] `cargo test` green on Linux + Windows + macOS (CI).
- [ ] `cargo doc --no-deps` clean (no `missing_docs` warnings).
- [ ] `cargo clippy --all-targets -- -D warnings` clean.
- [ ] `cargo fmt --all -- --check` clean.
- [ ] `CHANGELOG.md` has a v0.1.0 entry.
- [ ] `README.md` renders correctly on GitHub.
- [ ] `git grep -i maxker` returns zero hits (no PaperPort binaries committed).
- [ ] `tests/fixtures/synthetic.max` is a non-personal synthetic file.

## Publish

- [ ] `git tag v0.1.0`
- [ ] `git push origin v0.1.0`
- [ ] `cargo publish` (dry-run first: `cargo publish --dry-run`)
- [ ] GitHub release created from the tag with auto-built binaries
      (Linux x86_64, Windows x86_64, macOS aarch64).

## Marketing channels (in priority order)

1. **JustSolve / fileformats.archiveteam.org wiki** — add a "Software"
   entry to http://fileformats.archiveteam.org/wiki/PaperPort_(MAX).
   Highest-leverage move: the page already lists `paperman` and
   `max2pdf` with the PP2-not-supported caveat. Adding `vigb-decoder`
   as the first PP2-capable tool makes it discoverable to anyone
   googling the format.

2. **GitHub issue on `sjg20/paperman`** — Simon Glass is active.
   Title: "PaperPort 2 era support via vigb-decoder". Body: brief
   pointer to this repo + offer to coordinate on shared format docs.

3. **Forum thread replies** (one-line "FYI, here's a Rust tool that
   handles PP2"):
   - https://www.bleepingcomputer.com/forums/t/688796/how-to-open-max-file/
   - https://learn.microsoft.com/en-us/answers/questions/2493531/help-for-files-with-max-extension-unable-to-open-t
   - https://www.windowsbbs.com/threads/how-to-convert-legacy-files-paperport-max.108861/
   - https://forums.linuxmint.com/viewtopic.php?t=194479
   - https://www.techguy.org/threads/retrieving-paperport-files-using-the-max-extension.1268281/
   - https://newsgroup.xnview.com/viewtopic.php?t=43432

   These threads still rank on Google for "open .max file".

4. **Reddit posts** — single post each to r/DataHoarder, r/datarecovery,
   r/genealogy. Title format: "I built a Rust tool for the dead
   PaperPort 2 (.max) format — first decoder that handles 1986–87
   era files".

5. **Open Preservation Foundation** — submit to their
   disappearing-file-formats blog series via blog@openpreservation.org.

6. **Optional: contact DiskTransfer.co.uk** — they openly market paid
   PP1/2/3 recovery. Offering to license/cite the decoder turns a
   competitor into a referrer.
