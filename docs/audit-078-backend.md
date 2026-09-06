# 078 audit: backend acceptance notes

Backend lane of plan `078_abakus-audit` (worktree `backend`, branch `audit/078-backend`).
Covers `account_delete_preview`/`delete_account` (new), the wider data-layer
audit, and the review corrections raised after the first pass.

## What shipped

- `crates/store/src/delete_account.rs` (new): `account_delete_preview` /
  `delete_account`, one `BEGIN IMMEDIATE`/`COMMIT`/`ROLLBACK` transaction
  scoped to the whole account. An account-scoped doomed-rule query (not a
  loop over the statement-scoped one) so a rule two of the same account's
  own statements both taught is still removed together. `reclassify_open`
  runs after the delete, so a historical transfer on a RETAINED account is
  never re-touched.
- `src-tauri/src/commands.rs`: `account_delete_preview`, `delete_account`
  Tauri commands. `delete_account`'s body is `delete_account_inner`, which
  clears the keyring credential BEFORE the database delete, not after: a
  keyring failure now leaves the account and its data intact for a retry,
  instead of an already-deleted account with an orphaned credential. See the
  doc comment on `delete_account` for the one residual boundary that two
  systems without a shared commit still leave (a database failure AFTER a
  successful keyring clear), and why a retry self-heals it (`secrets::clear`
  is idempotent).
- `src-tauri/src/secrets.rs`: `clear` returns `Result<(), String>` instead
  of swallowing the keyring error, fixing the same defect class in the
  pre-existing `clear_password` command.
- `crates/store/src/summary.rs`: `csv_quote` defuses a leading `= + - @`,
  leading tab/CR, and their full-width Unicode look-alikes (OWASP CSV
  Injection list, https://owasp.org/www-community/attacks/CSV_Injection,
  read 2026-09-06), with the actual coverage and its limits stated in the
  doc comment rather than a universal guarantee.
- `src-tauri/src/commands.rs`: `export_csv` now reports the record count
  from the same filtered `list_transactions` query the export itself runs,
  not `csv.lines().count() - 1`, which overcounts a record whose field
  legitimately contains an embedded newline (CSV-quoted, not stripped).
- `crates/store/src/delete_account.rs` / `delete_statement.rs`: both
  `delete_account`/`delete_statement` now roll back when `COMMIT` itself
  fails, not only when the delete body fails. Previously
  `execute_batch("COMMIT")?` would propagate a commit error and leave the
  connection holding an open transaction.
- Cyclomatic complexity: `assign_one` (15 to 11 via `learn_rules_for`),
  `row_to_tx` (23 to 3 via `row_identity`/`row_detail`/`parse_row_date`),
  `summary_filtered` (20 to 6 via one method per aggregate). All measured
  with the task-local Lizard tool, `-C 12`, no suppressions.

## Tests

- `crates/store/tests/delete_account.rs` (public-API-only), 3 inline tests
  in `crates/store/src/delete_account.rs` needing raw SQL against a state
  the app itself can't reach (mid-transaction rollback, a commit-failure
  rollback, a refund's evidence disappearing), 1 inline commit-failure test
  in `crates/store/src/delete_statement.rs`, 8 inline unit tests in
  `src-tauri/src/commands.rs` for `account_delete_error`/
  `delete_account_inner`/`export_csv_inner`, unit tests on `csv_quote`
  itself in `crates/store/src/summary.rs` (every OWASP-listed ASCII marker,
  every full-width look-alike, an ordinary value, an embedded quote), and
  one CSV-injection regression test in `crates/store/tests/assign_summary.rs`.
- The commit-failure regressions use `PRAGMA defer_foreign_keys = ON` over
  the same dangling-category setup the existing rollback test uses, so the
  violation surfaces at `COMMIT` instead of inside the transaction body,
  exercising the new branch specifically. Both then perform an unrelated
  write afterward to prove the connection is still usable.
- The multiline export-count regression drives an embedded newline through
  the real public API (a category name is only trimmed at the ends), not
  raw SQL: `export_csv_counts_records_not_lines_when_a_field_has_an_embedded_newline`.

## Commands and results (2026-09-06)

```powershell
$env:CARGO_TARGET_DIR = "A:/projects-vault/apps/abakus/target"
$env:ABAKUS_PDFIUM_DIR = "A:/projects-vault/apps/abakus/src-tauri/resources/pdfium"
cargo test --workspace --jobs 4     # exit 0, all suites green (see evidence/backend-cargo-test-20260906.log)
cargo clippy --workspace --all-targets --jobs 4   # exit 0, no warnings (see evidence/backend-cargo-clippy-20260906.log)
py A:/projects-vault/animus/data/agent-runs/abakus-audit-20260906/tooling/lizard.py -l rust -C 12 -w crates src-tauri/src .   # exit 0, no warnings
```

Full logs: `A:/projects-vault/animus/data/agent-runs/abakus-audit-20260906/evidence/`.

## Honest limits

- Cancellation-never-calls-delete and no-double-submit (contract clause 16)
  is frontend-owned UI debounce; not implementable or testable from this
  lane.
- The original bank PDFs are never touched by any delete path (confirmed by
  reading `delete_account`/`delete_statement`: neither calls `std::fs`); no
  dedicated regression exists for this because there is nothing to assert
  against beyond "no such call exists".
- The keyring-then-database residual boundary described above is real and
  intentionally left as a documented, self-healing residual, not eliminated:
  a keyring and SQLite cannot share one atomic commit. A test proves the
  common case (keyring failure leaves everything intact); the reverse case
  (database failure after a successful keyring clear) is not separately
  regression-tested because reaching it requires engineering the same kind
  of unreachable-via-the-app database state the existing rollback tests
  already use to reach `delete_account`'s own internal failure paths, one
  level removed through the command layer's `&dyn Fn` seam, and the store
  layer's own commit-failure regression already proves the database side of
  that residual (a rolled-back commit) recovers cleanly on its own.
- No coordination was needed with the frontend lane: the account-delete IPC
  shape matches `contracts.md` exactly, unchanged from the first pass.
