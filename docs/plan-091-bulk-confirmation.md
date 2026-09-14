# Plan 091 bulk confirmation backend

This slice implements the backend contract for accepted points 1 and 11 of
Abakus 0.2.0. The store, Tauri command and TypeScript API are ready. The UI
controls shipped in `src/screens/Transactions.tsx`: the per-row checkbox
"Potvrdiť aj podobné (N)" and the bulk-bar button "Potvrdiť vybrané".

## API contract

The public store method is:

```rust
Store::confirm(ids: &[i64], apply_to_matching: bool) -> Result<AssignOutcome>
```

The Tauri request is `{ ids, applyToMatching? }`. An omitted
`applyToMatching` value means `false`. This keeps an older request containing
only `ids` valid.

The Tauri response changed from a number to the existing `AssignOutcome`
object. The current one-row UI caller ignores the response, so the type change
does not change its behavior.

- `AssignOutcome.updated`: unique rows that this call changed to `confirmed`.
  The count includes selected suggestions and optional matching open rows.
- `AssignOutcome.rules_created`: unique learned rules created by this call.
  Reusing or retargeting an existing rule does not increase the count.
- `AssignOutcome.skipped_transfers`: unique selected transfer ids that the
  protected transfer guard did not change.

The TypeScript method is `confirm(ids, applyToMatching = false)`. It returns
`Promise<AssignOutcome>`.

The companion undoable legacy assignment path is documented in
[`plan-091-assignment-undo.md`](plan-091-assignment-undo.md). It adds separate
`bulk_assign` and `undo_last_assignment` commands. The single-row assignment
and this confirmation contract stay unchanged.

## Selection rules

- Input ids are deduplicated before any count or write.
- An empty id list succeeds with zero counts and no writes.
- An unknown id returns `UnknownTransaction` and rolls back the full batch.
- A selected `suggested` row with a concrete category is confirmed.
- A selected `confirmed` row is unchanged.
- A selected `unassigned` row is unchanged.
- A selected `transfer` row is unchanged and increments `skipped_transfers`
  once.

## Optional matching rules

Matching uses transaction facts and categories captured after `BEGIN
IMMEDIATE` and before the first write.

- `card`, `card_foreign`, `refund`, `atm` and `other` use the normalized
  merchant plus normalized place as one exact identity.
- A merchant must be non-empty. An empty merchant never learns a merchant rule
  and never matches another row.
- Place equality is exact. Missing place matches only missing place. Missing
  place never matches a concrete place.
- `transfer_in`, `transfer_out` and `standing_order` use the exact counterparty
  IBAN as their identity.
- A counterparty IBAN must be non-empty. An empty IBAN never matches another
  row and never creates a counterparty-account rule.
- A card-like row does not use an incidental counterparty IBAN as its identity.
- A matching target must be `suggested` or `unassigned`.
- A matching target may have no category or the same concrete category as the
  selected source.
- A matching target with another concrete category is unchanged.
- A confirmed or transfer target is unchanged.
- Conflicting selected categories for one exact identity disable matching and
  rule learning for that identity.
- Conflicting selected categories for one merchant disable the broad merchant
  rule. Non-conflicting exact merchant-place rules remain valid.
- The result is independent of selected-id order.

`Store::assign` uses the same transaction, learning, provenance and counting
engine. Its older `apply_to_matching` behavior remains merchant-wide for
compatibility. Its `updated` field now includes the selected rows and all
merchant matches that it confirms.

## Atomicity and persistence

Selection validation, rule learning, transaction updates and `rule_sources`
writes share one SQLite transaction. A failure after an earlier successful
write rolls all of them back. The same `Store` can retry after rollback.

Successful learned rules and their `rule_sources` survive database reopen.
Later open-row reclassification does not change confirmed rows. Existing
confirmed rows and transfer rows are outside automatic matching.

No schema migration is required.

## Additive surface inventory

- Rust argument `Store::confirm.apply_to_matching: bool` controls optional
  matching.
- Tauri argument `applyToMatching?: boolean` maps to Rust
  `apply_to_matching: Option<bool>`.
- Tauri default: omitted `applyToMatching` maps to `false`.
- TypeScript argument `confirm.applyToMatching: boolean` defaults to `false`.
- TypeScript interface `AssignOutcome` names the existing response fields.
- Response change: Tauri `confirm` returns `AssignOutcome` instead of a number.
- Matching rule: card-like transactions use non-empty normalized merchant and
  exact normalized place.
- Matching rule: bank transaction kinds use exact non-empty counterparty IBAN.
- Matching rule: only open rows with no category or the same category qualify.
- Conflict rule: one identity with several selected categories is not swept.
- Conflict rule: one merchant with several selected categories creates no
  broad merchant rule.
- Count rule: `updated` counts unique selected and matching confirmed rows.
- Count rule: `skipped_transfers` counts unique selected transfer ids.
- Error rule: an unknown id fails the full batch with `UnknownTransaction`.

## Verification scope

The focused tests use generated parser `Statement` values, temporary SQLite
files and the public `Store` API. They do not read private fixtures or a live
database. A SQLite trigger injects a failure on the second transaction update
to prove rollback after a successful first write and a successful retry on the
same `Store`.
