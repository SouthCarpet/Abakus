# 0.1.2 backend: statement history, transaction notes, database backup

Backend lane of the 0.1.2 release (worktree `080-backend`, branch
`feature/080-backend`), executing
`A:/projects-vault/animus/data/agent-runs/abakus-012-20260907/contract.md`.
Covers `statement_history` (new), `TxRow.note`/`save_transaction_note` (new),
the text-search extension that reaches notes, the CSV `poznamka` column, and
`backup_database` (new). No UI, `src/api.ts`, version numbers, or other
worktrees were touched.

## What shipped

- `crates/store/src/history.rs` (new): `Store::statement_history(account_id,
  account_kind)`, both filters optional and independent (AND when both are
  given), sorted `account_id, period_start, period_end, statement_id`
  ascending. Read-only, separate from the existing `recent_statements`
  (newest-first, capped, Import screen only).
- `crates/store/src/notes.rs` (new): `Store::save_transaction_note(id, note)`.
  Validates BEFORE writing (a U+0000 byte, over `NOTE_MAX_CHARS` = 2000
  Unicode code points via `note.chars().count()`, or an id that does not
  exist all leave the row untouched), then a plain `UPDATE ... SET note =
  ?2`. An empty string clears the note. The column is never named in
  `classify_ids`'/`reclassify_open`'s `UPDATE`s, so a note survives every
  reclassification path (regression: `a_note_survives_reclassification`).
- `crates/store/src/schema.sql` / `migrate.rs`: `transactions.note TEXT NOT
  NULL DEFAULT ''`, added to `schema.sql` for new databases and via a guarded
  `ALTER TABLE ... ADD COLUMN` (`add_notes_column`, checked through
  `pragma_table_info` so it is idempotent whether or not the column already
  exists) for existing ones. `SCHEMA_VERSION` 2 to 3. `Store::migrate` now
  wraps its steps in one `BEGIN IMMEDIATE`/`COMMIT`/`ROLLBACK` (it did not
  before this release): a failure partway through returns the database to
  its old version, never a half-migrated state.
- `crates/store/src/query.rs`: `TxRow.note: String`. The free-text filter
  (`TxFilter.text`) now matches `note` too, and the whole match moved from
  SQL `LIKE '%'||?||'%'` to a Rust-side `parser::fold`-based literal
  substring check (`filter_by_text`/`row_matches_text`) across
  `merchant_raw`, `place`, `counterparty_name` and `note` together. This was
  the only way to satisfy the contract's "`parser::fold` and literal
  substring semantics (percent/underscore are literals)" for a free-form
  note, since SQLite's built-in `LOWER`/`LIKE` cannot fold Unicode diacritics
  without an ICU build; it also fixes the latent wildcard leak the old `LIKE`
  had for the three existing fields (a literal `%`/`_` in the search text no
  longer acts as a SQL wildcard). `where_clause` (shared with the summary
  aggregates, which never set `text`) no longer builds a text condition at
  all.
- `crates/store/src/summary.rs`: CSV header gains a final `poznamka` column;
  `csv_line` appends `csv_quote(&r.note)`. `csv_quote`'s
  `DANGEROUS_LEADING` gains `'\n'`: `note` is the first multiline field this
  function ever sees (`merchant_raw`/`place`/`counterparty_name` are
  single-line parsed fields, `raw_block` is still never passed through it),
  so a leading line feed is now a real input, not the hypothetical case the
  old doc comment described.
- `crates/store/src/backup.rs` (new, race fixed in the 080-repair pass): `Store::backup_to(dest) -> BackupOutcome { path, bytes }`, via `rusqlite`'s
  online-backup API (`Backup::new` + `run_to_completion`, not a filesystem
  copy of the live file, and not the one-shot `Connection::backup` helper,
  which does not retry `Busy`/`Locked`). `dest` is claimed with
  `File::options().create_new(true)`, which atomically fails instead of
  opening or truncating if the path already exists at that instant; this
  refuses self-overwrite, an alias, an old backup, and a competing writer
  that creates `dest` in the same window, all in one step, with no separate
  existence check first. A failure at any later step removes `dest`, which
  this call is always the exclusive creator of, so cleanup can never touch a
  file another process owns. Touches only `self.conn` and the filesystem;
  never the OS keyring.
- `crates/store/Cargo.toml`: `rusqlite`'s `backup` feature enabled (both
  `[dependencies]` and `[dev-dependencies]`, matching the existing
  `bundled` duplication). This is a feature flag on the already-present
  dependency, not a new crate.
- `src-tauri/src/commands.rs` / `lib.rs`: three new Tauri commands,
  registered in `invoke_handler!`: `statement_history`,
  `save_transaction_note`, `backup_database`. Each is a thin `lock(&state)?`
  + store call, matching every other command in the file.
- Cyclomatic complexity: `history::row_to_history` measured 13 against the
  project's Lizard budget (`-C 12`); split into `row_identity`/
  `row_to_history` (same pattern `query.rs`'s `row_to_tx` already uses),
  6 and 6.

## Tests

`cargo test` grew from the 078-audit baseline by 35 tests: 10 inline
(`crates/store/src/{notes,history,backup,summary}.rs`) and 25 integration
(`crates/store/tests/{notes,history,backup}.rs`, new, plus 3 added to
`tests/migration.rs` and 5 added to `src-tauri/tests/commands_json.rs`).
Every integration test in the three new files opens a REAL file (`Store::open`
on a `std::env::temp_dir()` path, `Drop`-cleaned), not `open_in_memory`, per
the brief.

- **Migration** (`tests/migration.rs`): a frozen schema-version-2 snapshot
  (`SCHEMA_V2`/`OLD_DATA_V2`, rule_sources and its provenance already
  populated with a REAL learned exact rule, not backfilled) reaches version 3
  with every row/status/provenance intact and a working `note` column
  (`a_v2_database_gains_the_notes_column_and_keeps_every_row_status_and_provenance`).
  Reopening an up-to-date database three times changes nothing
  (`reopening_an_up_to_date_database_is_a_no_op_and_stays_idempotent`). A
  second connection holding the file's write lock makes `Store::open` fail
  outright rather than complete the schema/migrate sequence halfway; once
  released, a later open still reaches the current version with all data
  intact (`a_locked_database_fails_the_open_cleanly_and_a_later_open_still_completes_it`,
  the genuine failure mode available without engineering an unreachable
  internal state; see Honest limits).
- **Notes** (`crates/store/src/notes.rs` inline): empty-clears-and-round-trips,
  a NUL byte rejected and untouched, exactly `NOTE_MAX_CHARS` accepted and
  `+1` rejected and untouched, an unknown id rejected and changes nothing,
  newlines/Unicode/emoji preserved exactly, a note surviving
  `classify_ids`.
- **Search** (`crates/store/tests/notes.rs`): a literal `%` and a literal `_`
  in the search text each match only the note that actually contains that
  character, not a decoy that would satisfy a SQL wildcard reading; a
  diacritic-and-case fold match (`Kávička` found by `kavicka`/`KAVICKA`);
  merchant/place search still works alongside note search; saving a note
  never changes `category_id`/`status`/`merchant_raw`.
- **CSV** (`crates/store/tests/notes.rs`,
  `crates/store/src/summary.rs::csv_quote_tests`): a note starting with a
  blank line then `=cmd(...)`, containing an embedded quote and an embedded
  plain newline, is defused (leading `'`), quote-doubled and kept multiline
  in the exported field; the exported record count matches the filtered
  query, not a naive line count.
- **History** (`crates/store/tests/history.rs`): no filters returns every
  statement sorted account/period/id (proven with statements imported
  OUT of period order); an `account_id` filter narrows to one account; an
  `account_kind` filter covers every account of that kind (not just the
  first, the same A17/F3 class of bug this release is careful not to
  reintroduce); both together still mean one account; a statement with no
  "Posledný výpis"/closing line reports `None`/`None`/`Checksum::NotVerifiable`
  instead of a defaulted zero; an ordinary statement reports its real
  cents and `Checksum::Ok`.
- **Backup** (`crates/store/tests/backup.rs`): a snapshot of a REAL seeded
  database (an imported statement, a learned exact rule from `assign`, a
  note, a changed setting) opens independently and keeps all four; a write
  made to the live store AFTER the backup never reaches the snapshot
  (isolation, both a note edit and a whole new account); backing up onto the
  store's own file is refused; an occupied destination (with content, and
  separately an empty file) is refused with its existing bytes left exactly
  as they were AND the live source unaffected; a destination whose parent
  directory does not exist fails cleanly, leaves no partial file, and leaves
  the source unaffected; 8 threads racing `backup_to` at the same real
  destination path resolve to exactly one winner and every loser reports
  `BackupTargetExists`, never a corrupted or double write (080-repair,
  proving the check-then-open race is closed).
- **JSON drift gate** (`src-tauri/tests/commands_json.rs`): `{accountId,
  accountKind}` (both independently optional, including the field being
  absent, not just `null`) for `statement_history`; `{id, note}` for
  `save_transaction_note`; `{path}` for `backup_database`; `StatementHistoryRow`
  and `BackupOutcome` serialize the fields the UI reads; `TxRow`'s existing
  round-trip test extended with `note`.

## Commands and results (080-repair, 2026-09-07)

```powershell
$env:CARGO_TARGET_DIR = "A:/projects-vault/apps/abakus/target"
cargo test --jobs 4 --workspace                              # exit 0, 281 tests, 0 failed
cargo clippy --workspace --all-targets -- -D warnings         # exit 0, no warnings
python -m lizard -C 12 crates src-tauri/src                   # exit 0, no function over CC 12
```

`pdfium::tests::env_override_wins` (crate `parser`, not owned by this lane)
failed once under the full-workspace multi-threaded run and passed cleanly
alone and on a second full run: two `parser::pdfium` tests race a shared
process environment variable. Pre-existing, unrelated to this repair, not
fixed here (out of scope: PDF parsing is explicitly excluded from this
lane).

## Honest limits

- `Backup::run_to_completion`'s retry loop has no maximum attempt count; it
  sleeps and retries on `Busy`/`Locked` until the source connection is free.
  For this app that source is the SAME `Mutex`-guarded connection the
  calling thread already holds, so there is no other in-process writer to
  contend with; the only realistic contention is an external process
  (antivirus, a second Abakus instance pointed at the same file), a case
  this release does not add a timeout for.
- The locked-database migration test proves the OUTER boundary (`Store::open`
  as a whole never leaves a partially-migrated file) via lock contention
  from a second connection, not a forced failure of one specific internal
  migration step. `add_notes_column` (a guarded, idempotent `ALTER TABLE ...
  ADD COLUMN`) and `backfill_rule_sources` (an `INSERT OR IGNORE ... SELECT`
  over rows that always satisfy their own foreign keys) have no natural
  failure mode short of a locked/full disk; engineering a mid-step failure
  for either would need the same kind of unreachable-via-the-app raw-SQL
  state the codebase already uses sparingly for commit-failure regressions
  elsewhere (`delete_account`/`delete_statement`), and was judged not worth
  adding here on top of the genuine lock-contention case.
- `NOTE_MAX_CHARS` counts Unicode scalar values (`str::chars().count()`), not
  grapheme clusters or display width. A multi-codepoint emoji sequence (a
  family emoji built from several codepoints joined with ZWJ, for example)
  counts as several toward the 2000-code-point limit, not one visible
  character. This matches how the contract states the limit ("2000 Unicode
  code points") literally; a grapheme-aware limit would need the
  `unicode-segmentation` crate the store crate does not currently depend on.
- Extending the free-text search to Rust-side `parser::fold` matching also
  changed HOW the three pre-existing fields (`merchant_raw`, `place`,
  `counterparty_name`) match, not just added `note`: they now fold
  diacritics/case and treat `%`/`_` as literal characters too, where the old
  SQL `LIKE` did neither consistently. This was a deliberate reading of the
  contract's "existing text filter... parser::fold and literal substring
  semantics" as describing the filter as a whole, not `note` in isolation,
  and it fixes a real (if minor) wildcard-leak bug in the old behaviour; it
  is called out here because it is a behaviour change beyond the minimum
  literal ask, and every existing search test (`assign_summary.rs`,
  `summary_scope.rs`) still passes unchanged.
- `statement_history` with both filters `None` returns EVERY statement in
  the database in one call, unlike `recent_statements`, which the contract
  gives an explicit `limit`. This matches the contract's `Vec<...>`-returning
  signature exactly (no limit parameter is specified), but is worth flagging
  for the parent/Insights lane if a very large multi-year history ever makes
  an unbounded history screen worth paginating.

## Integration notes for the parent

- `src/api.ts` needs three new declarations, shapes exactly as in
  `contract.md`: `statementHistory(accountId, accountKind)` invoking
  `statement_history` with `{accountId, accountKind}`;
  `saveTransactionNote(id, note)` invoking `save_transaction_note` with
  `{id, note}`; `backupDatabase(path)` invoking `backup_database` with
  `{path}`. All three are round-tripped against the real Rust types in
  `src-tauri/tests/commands_json.rs`.
- `TxRow` gained a required `note: string` field (legacy rows read back as
  `""`). The Interaction lane's existing `TxRow` TS test fixture needs the
  matching field, per the contract's ownership split; this backend lane did
  not touch `src/api.ts` or any `.tsx` file.
- No coordination gaps found against `contract.md` at time of writing; all
  three new IPC shapes match it exactly.
