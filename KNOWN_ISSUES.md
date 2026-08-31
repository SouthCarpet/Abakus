# Known issues

- **pdfium binary: trust on first use.** `scripts/fetch-pdfium.ps1` pins a release tag
  (`chromium/7469`) from `bblanchon/pdfium-binaries` and checks the archive against
  `scripts/pdfium.sha256`. The hash file was written on the first run, from that same
  download. This proves the binary stays the same across re-runs. It does not prove the
  binary was safe on day one. Treat the pinned hash as trust-on-first-use, not as an
  independent security check.
- **Encrypted-PDF import path: proved only by a real run.** No synthetic PDF in this
  repo is password-protected (the generator only writes plain PDFs), so the locked-card
  UI and the stored-password retry loop have unit coverage on the report shape but no
  end-to-end proof against a real encrypted file. Michal's local check (D13, task 16
  step 4) is the only run that opens a real locked statement; until it reports
  `checksum: ok`, treat the encrypted path as unverified against a real PDF.
- **Net-audit connection sampler is a single snapshot, not continuous monitoring.**
  `sample_connections` reads the process's open TCP sockets once, at app start
  (`src-tauri/src/lib.rs`), and again whenever the settings screen re-runs it. A
  connection opened and closed between two samples leaves no trace in `net_log`. This
  catches a leak that stays open; it does not catch a brief one.
- **Closing-balance label set is fixed, not learned.** The parser recognizes four
  Slovak wordings for the closing-balance line (`parser::statement::CLOSING_LABELS`,
  the Tatra banka wording first, three fallbacks). A statement layout using a fifth
  wording reports `not_verifiable` rather than a wrong number, which is the safe
  failure mode, but it still means an unrecognized label needs a new fixture line and
  a parser fix rather than being handled automatically.
- **Foreign-currency rate precision.** `card_foreign` transactions store the exchange
  rate as `rate_micros` (six decimal digits). Re-deriving the original-currency amount
  from `amount_cents` and `rate_micros` can be off by a cent from the bank's own
  printed original amount because of this rounding; the parser keeps both the printed
  original amount and the rate rather than recomputing one from the other, so this
  affects only a hypothetical future feature, not any value shown today.
- **No installer in v1.** `src-tauri/tauri.conf.json` has `bundle.active: false`. Abakus
  ships as a repo you build and run (`npm run tauri dev` or a built binary you copy
  yourself), not as a signed installer or an auto-updating package.
- **Update check needs a published release.** `check_update_now` reads GitHub's
  `releases/latest` endpoint (`src-tauri/src/update.rs`). Until the repo has a
  published release with a tag, that endpoint has nothing to return, so a fresh
  install reports "no update" even when the code on `main` is newer than the copy
  the user built.
- **"Použiť aj na podobné" never rewrites a confirmed row.** Applying a category to
  matching transactions (`Store::assign` with `apply_to_matching`, see
  `crates/store/src/assign.rs`) re-files only rows still `suggested` or
  `unassigned`. A row already `confirmed` keeps the category the user chose for it.
  This is intentional, not a gap: a confirmed choice is a decision Abakus never
  silently overwrites (spec-adjudicated 2026-08-31).
