# Plan 091 assignment undo backend

This slice adds the backend contract for one reversible bulk assignment in
Abakus 0.2.0. It includes the store operation, Tauri commands and TypeScript
API. The user interface shipped in `src/screens/Transactions.tsx` as the
transient "Späť" control after a bulk assignment. Native application
acceptance is still pending.

## Store contract

`Store::assign_undoable(ids, category_id, apply_to_matching)` uses the existing
assignment and learning rules. It returns `BulkAssignOutcome` with the usual
counts and an optional opaque `undo_id`. The ID is unique between Store
instances during one process run. It is not persisted and it is not a secret.

The Store keeps one private undo record. A later successful undoable assignment
replaces it. An empty or transfer-only assignment succeeds without an ID and
clears the older record. Reopening the Store also clears it. The existing
single-row `Store::assign` and `Store::confirm` methods are unchanged.

`Store::undo_last_assignment(expected_undo_id)` requires the exact current ID.
A wrong or old ID returns `None` and keeps a newer record available. A matching
request returns `UndoAssignmentOutcome { restored_rows }` after a successful
restore. It returns `None` and consumes the matching record if later database
work made the record stale.

## Targeted restore

The private record contains only data affected by the assignment:

- the before and after values of transaction `status`, `category_id`,
  `rule_id` and `source`;
- the before and after category for each learned rule identity;
- the exact provenance pairs newly inserted into `rule_sources`;
- the Store connection change count and SQLite `main.data_version` markers.

Undo restores those four transaction fields. It restores the old category of
a retargeted rule, deletes a rule created by the assignment, and deletes only
new provenance pairs. It does not restore a database snapshot and does not run
open-row reclassification. Money, notes, fingerprints, raw data and unrelated
rows are outside its write set.

## Concurrency and failures

Assignment capture runs inside the assignment `BEGIN IMMEDIATE` transaction.
Undo starts its own `BEGIN IMMEDIATE` before it checks the connection marker,
the external connection marker and all captured post-images. A same-connection
write or a committed external write makes the matching record stale. Read-only
queries and backup do not.

A wrong ID does not start a transaction. Failure to acquire the undo write lock
keeps the record. A restoration DML or COMMIT failure rolls back the transaction
and consumes the record. The implementation does not refresh markers outside a
write lock and has no manual invalidation registry.

## Tauri and TypeScript API

The new Tauri commands are:

```text
bulk_assign(ids, categoryId, applyToMatching)
undo_last_assignment(expectedUndoId)
```

The TypeScript methods are `api.bulkAssign(ids, categoryId,
applyToMatching)` and `api.undoLastAssignment(expectedUndoId)`. Existing
`api.assign` and `api.confirm` calls are unchanged.

## Additive surface inventory

- Rust type `BulkAssignOutcome.updated: usize`: changed selected and matching
  row count.
- Rust type `BulkAssignOutcome.rules_created: usize`: newly created learned
  rule count.
- Rust type `BulkAssignOutcome.skipped_transfers: usize`: protected selected
  transfer count.
- Rust and TypeScript field `BulkAssignOutcome.undo_id: Option<String>` or
  `string | null`: the expected ID for this operation.
- Rust and TypeScript field `UndoAssignmentOutcome.restored_rows`: restored
  transaction count.
- Store method `assign_undoable` and Tauri command `bulk_assign`: run the
  undoable bulk path.
- Store method and Tauri command `undo_last_assignment`: require the opaque
  expected ID and restore only the matching current record.
- TypeScript method `bulkAssign`: sends `ids`, `categoryId` and
  `applyToMatching`.
- TypeScript method `undoLastAssignment`: sends `expectedUndoId`.
- Replacement rule: a successful undoable assignment replaces the one private
  record.
- No-op rule: an empty or transfer-only successful assignment clears it.
- Identity rule: a mismatched expected ID leaves a newer record untouched.
- Stale rule: marker or post-image mismatch consumes the matching record.
- Failure rule: a restore DML or COMMIT failure rolls back and consumes the
  record; failure to acquire the initial write lock retains it.
- Reopen rule: undo state is process memory and does not survive Store reopen.

## Verification scope

Focused Rust tests use constructed parser values and temporary SQLite files.
They cover wrong and stale IDs, replacement, Store reopen, same-connection and
external writes, rules, provenance, rollback, read-only work, and protected row
data. A real competing write lock proves BEGIN failure retains the record, and
a test-only deferred foreign key proves COMMIT failure rolls back and consumes
it. JSON tests cover the exact command arguments and nullable responses.
These source tests do not claim UI, native application or installer acceptance.
