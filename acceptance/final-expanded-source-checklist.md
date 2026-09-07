# Abakus 0.1.2 independent source and native checklist

This checklist is an acceptance input. It is not a verdict. Every checked row
must cite the exact final commit, built executable hash, command output or new
synthetic artifact. Expected values come from `contract.md`,
`recurring-acceptance.md` and `pdf-acceptance.md` in the controller evidence
directory.

## Provenance gate

- [ ] `final-source-ready.json` names the exact final commit and source tree.
- [ ] `pdf-ready.json` names the same commit, production executable, SHA256,
  PID, CDP endpoint, isolated `data_dir` and oracle path.
- [ ] The executable SHA256 is calculated again before attach.
- [ ] The first native call is `data_dir`; it equals the isolated synthetic
  profile path before any mutation.
- [ ] The page uses the embedded Tauri protocol and does not use a Vite URL.
- [ ] No command reads `fixtures/private`, the real user database or a real
  statement.

## Source boundaries

- [ ] Report capture starts one deferred SQLite read transaction before its
  first account, transaction, category or statement read.
- [ ] Every captured value is owned before the transaction ends. No SQLite
  row, `Store` reference or mutex guard crosses an await or render step.
- [ ] Report requests contain only period and account scope. No table search,
  status, category, statement or page filter reaches capture.
- [ ] Account scope validates positive JavaScript-safe IDs and includes every
  account of a selected kind, including accounts without statements.
- [ ] Date parsing is strict calendar arithmetic for years 0001 through 9999.
  It does not use local elapsed milliseconds or approximate day counts.
- [ ] Money uses integer cents and checked `i128` intermediates through final
  checked `i64` values. Formatting handles `i64::MIN` without negation.
- [ ] Read-only report and recurrence queries do not write decisions, settings,
  classifications or category data.
- [ ] Recurrence keys separate account, cash direction, currency, identity,
  place and card. Blank identities cannot merge automatically.
- [ ] Missing recurrence uses only complete trusted statement coverage. A
  transaction count or another account never proves absence.
- [ ] Category creation has no rule side effect. Explicit assignment or
  confirmation of a recognized merchant is the rule-learning action.
- [ ] V2 and v3 migration, v4 DDL, Spotify repair and version update share the
  required transaction. Future schema versions are rejected without writes.
- [ ] Backup and PDF use independent snapshots and no-clobber publication.
  Errors propagate only after owned temporary output is cleaned.
- [ ] Report generation has no network, runtime Python, browser print, system
  font lookup, raw block output, full IBAN, password or source path.

## PDF structure and content

- [ ] Static fonts are embedded with Type0 Identity-H, CIDFontType2,
  CIDToGIDMap, FontFile2 and a bounded ToUnicode CMap.
- [ ] Canonically equivalent Slovak input renders as NFC without changing the
  stored source. Unsupported scalars become visible `[U+XXXX]` text and add a
  replacement count.
- [ ] Text wraps using the same font metrics used to draw. Long words and
  2000-code-point notes keep their final sentinel.
- [ ] Split rows keep one ordinal and amount. Continuations say
  `Pokračovanie položky N`.
- [ ] Each transaction page repeats the table header and report context.
- [ ] Every page says `Strana X z Y`; no page is blank or exceeds its page
  bounds.
- [ ] Top ten category bars use absolute ordering and a signed `Ostatné` sum.
  Numeric keys reconcile to the exact KPI totals.
- [ ] More than 24 months use named years. More than 24 years continue in
  panels of at most 24 without dropping middle years.
- [ ] The renderer rejects 100000+1 rows, 32 MiB+1 source text, 2000+1 pages
  and 100 MiB+1 output. Boundary values have separate proof.
- [ ] The standalone inspector binds PDFium once, decodes all text and glyph
  bounds, and renders every page. Its manifest binds each PNG to the input
  PDF SHA256.

## Literal synthetic scenarios

- [ ] P01-P03: leap/non-leap month edges, six-month and year ranges, unfinished
  periods, all/kind/account scopes and ignored table filters.
- [ ] P04: 8 rows; income 100000; expense 10000; net 90000; transfers 20000
  each; category 7500; uncategorized 2500; suggested 1; unassigned 1.
- [ ] P05-P06: category/sign mismatches, refund-only negative expense, empty
  output and all overflow errors.
- [ ] P07-P08: per-account trusted interval union and one consistent WAL
  snapshot while a second connection commits a change.
- [ ] P09-P10: 31-row page overflow, all years retained, 12 signed categories
  plus reconciled `Ostatné`.
- [ ] P11-P12: Slovak/NFC/NFD, explicit unsupported marker, long note, exact
  fit/one-line-over/split row, 300 rows and continuous ordinals.
- [ ] P13: real 5000-row file plus deterministic arithmetic seams at every
  hard limit.
- [ ] P14-P15: occupied/source/hardlink/symlink/WAL/SHM/directory/device/ADS,
  DOS-reserved `NUL.pdf`/`CON.pdf` leaf names, two writers and late
  write/flush/sync/persist failures.
- [ ] P16: actual dialog cancel and occupied selection, stale preview guard,
  busy control lock and actual capture count after a post-preview commit.
- [ ] P17: literal v4 decisions/members/notes/category edits/settings survive
  restart and PDF contains actual transactions only.
- [ ] R01-R16, M01-M03 and C01-C05 each map to one named executed Rust test
  and its literal assertion or mutation evidence.
- [ ] U01-U06 run through real native UI controls, then native IPC and raw
  SQLite confirm the saved state. IPC-only canaries do not replace these
  front-door flows.

## Migration profiles

- [ ] `create-migration-oracles.py` creates one new root containing v2, v3,
  v3 without the Spotify legacy signature, v2 version-write failure, v3 lock
  and future-v5 profiles. Each profile stays inside `acceptance/`.
- [ ] `verify-migration-oracle.py --phase before` passes before any launch.
  Successful v2/v3 profiles then reach v4 with exact notes, learned
  provenance, settings and assignments retained.
- [ ] The exact legacy Spotify signature moves its seed rule and open row to
  one new Spotify child. Its confirmed Apple row stays unchanged. The minimal
  profile gains no Spotify category or rule.
- [ ] The version-write trigger, an independently held SQLite lock and the
  future-v5 marker each make the production process fail without changing
  the original schema, marker or table hashes.
- [ ] A post-migration snapshot is recorded after the first launch and
  compared byte-for-byte as JSON after reopening the same profile. This is
  the native idempotence proof; an in-process second open is secondary.

## Mechanical gates after the controller opens the serial Rust window

- [ ] `cargo test --jobs 4 --workspace` with the controller target directory
  and PDFium environment.
- [ ] `cargo clippy --jobs 4 --workspace --all-targets -- -D warnings`.
- [ ] `py -m lizard -l rust -C12 -L1000 -a1000 -w crates src-tauri/src`.
- [ ] Full Vitest with `--configLoader runner`, ESLint, TypeScript and Vite.
- [ ] Every command records exit status, test count and skipped tests. A name
  filter that runs zero tests fails the gate.
- [ ] `git diff --check`, version 0.1.2 locations, release documentation and
  byte-equal `CLAUDE.md`/`AGENTS.md` are checked at the final commit.

## Visual handoff

- [ ] Screenshot manifest covers all five main screens in light and dark at
  1024x800 and 1280x800, with the visible theme selector.
- [ ] Focused screenshots cover recurring mixed/unknown/empty states, manual
  membership, category create/edit/move/kind acknowledgement, learned-rule
  help, long-note normal/error, account-delete confirmation/error, backup
  normal/error and PDF preview/error/success.
- [ ] Every screenshot records its state, viewport, DOM width, visible-text
  hash, control values, executable/oracle hashes and relevant UI source hashes.
- [ ] All PDFium PNGs and the native screenshot manifest are immutable and
  hash-bound to the accepted executable and PDFs.
- [ ] The separately routed Google reviewer opens the rendered pixels. This
  functional verifier makes no rendered-pixel quality judgment.
- [ ] Ordinary reports expose every page. The 5000-row report exposes an
  all-page contact sheet plus full-resolution representative and flagged
  pages, with the inspected page set stated exactly.
- [ ] `create-pdf-contact-sheets.ps1` verifies each rendered-page hash and
  maps every stress-report page to one labelled contact-sheet tile exactly
  once. Contact-sheet creation is mechanical and makes no pixel judgment.

The inspector example needs direct dev dependencies in
`acceptance/inspect-pdf-report-dev-deps.patch`. The controller applies that
small patch only after the PDF backend owner finishes its Cargo changes.
