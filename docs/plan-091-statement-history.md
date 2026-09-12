# Plan 091: statement-history transaction aggregates

Point 19 extends the existing `statement_history` result. It does not add a
database migration, endpoint, history limit, or visual change. The UI work is
still pending.

Point 21 adds a separate [statement review API](plan-091-statement-review.md).
It reports stored parser warnings and live retained-row open counts. Legacy
warning evidence stays unknown. It does not add fields to the history DTO,
and a clean checklist does not establish bank-data completeness.

## Contract

Each `StatementHistoryRow` contains the count and signed integer-cent sum of
the retained `transactions` rows whose stored `statement_id` points to that
statement. Every transaction kind contributes. An empty statement returns
`0` for both fields.

The query uses statement-correlated scalar aggregates. It produces exactly
one history row per statement and does not evaluate totals for statements
excluded by the account filters. SQLite reports an integer `SUM` overflow as
an error; the store returns that error instead of wrapping the total.

The existing behavior stays unchanged:

- `account_id` and `account_kind` are optional and use AND when both are set.
- Rows stay ordered by account, period start, period end and statement ID.
- The endpoint stays unbounded and returns all matching statements.

## Retained-row boundary

These values are database aggregates. They are not the original PDF totals
and do not reconstruct transactions that were removed or deduplicated.

`statements.file_hash` is unique, but a re-export with a different file hash
can create another statement row. Transaction fingerprints are unique across
the database. If all transactions from the re-export match retained rows
owned by the first import, the new statement owns no rows and returns
`transaction_count: 0` and `total_cents: 0`.

## Additive surface inventory

- `StatementHistoryRow.transaction_count`: required signed 64-bit integer in Rust and required JSON/TypeScript number. It counts retained transactions owned by the statement.
- `StatementHistoryRow.total_cents`: required signed 64-bit integer in Rust and required JSON/TypeScript number. It sums the stored `amount_cents` values with their signs.

## Verification scope

Store integration tests use the public `Store` API and synthetic statements.
They cover empty values, mixed positive and negative cents, every current
transaction kind, statement and account isolation, intersected filters,
deduplicated re-exports, one-row aggregation, and integer overflow. The Tauri
JSON test proves that both snake_case fields are present. TypeScript fixture
builders were updated mechanically; the UI does not consume the fields yet.
