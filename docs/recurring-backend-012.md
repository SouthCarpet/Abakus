# Recurring backend (0.1.2), lane A

Status: provisional source snapshot. The maker CLI stopped before completing
verification or integration. The successor must complete the accepted contract,
including atomic fresh seeding and the real category repair helper. Descriptions
below reflect this unfinished source and are not release acceptance evidence.

Owner: lane A (recurrence store, detection, migration). Contract:
`animus/data/agent-runs/abakus-012-20260907/recurring-contract.md`. Acceptance:
`animus/data/agent-runs/abakus-012-20260907/recurring-acceptance.md`.

## Schema v4

`crates/store/src/migrate.rs` adds `recurring_decisions` and
`recurring_members` at schema version 4, through the SAME versioned,
transaction-guarded path a legacy database uses to reach v2/v3 — never
through `schema.sql`'s unconditional `CREATE TABLE IF NOT EXISTS`. A fresh
database reaches v4 through this path too.

`crates/store/src/lib.rs`'s `Store::init` now wraps base schema creation
(`schema.sql`) and the whole migration (`Store::migrate_tx`, v1→v4) in one
`BEGIN IMMEDIATE`/`COMMIT`. A failure anywhere in that range — including a
failed `COMMIT` itself — rolls back and leaves a database that already
existed on disk completely unchanged. Category/rule seeding
(`Store::seed_categories`/`seed_rules`) is unchanged: it still runs after
that commit, in its own transaction, only against an already-committed
empty `categories` table.

A database whose `schema_version` is newer than this binary's `SCHEMA_VERSION`
is refused outright (a clear Slovak error), and its version marker is never
downgraded.

**Open integration seam (explicitly pending, not a stub):** the contract
calls for `repair_spotify_seed_tx` to run inside this same v4 transaction
(recurring-contract.md §8-9), owned by lane C
(`crates/store/src/seed_repair.rs`). That module does not exist in this
isolated worktree. `migrate_tx`'s v4 branch has a comment marking exactly
where the call belongs; no stub or fake module reference was added. A
parent/controller integration pass must wire the real call in once C
delivers it, and add a shared v4 migration test that exercises both the
recurring DDL and the Spotify repair atomically.

## Group identity

`crates/store/src/recurring/key.rs` computes a versioned canonical
`group_key`: `[version, account_id, direction, currency_basis, identity_kind,
identity_value, place_norm, card_last4]`, hashed with SHA-256, every field
separated by a trailing `0x1F` byte (never raw string concatenation). A
blank identity (no counterparty IBAN+name, no merchant) folds the
transaction's own fingerprint into the key, so it can never accidentally
share a key with another blank-identity row — it is reachable only through
an explicit manual "selected" decision, never automatic inference.

## Detection

`crates/store/src/recurring/detect.rs` groups already-`as_of`-scoped
evidence by `group_key` and looks for the EARLIEST contiguous run where
every adjacent pair (in date order, no skipping) satisfies the day-gap and
calendar-month-difference table for monthly (3 obs, 28-33 days, 1 month)
before quarterly (2 obs, 85-97 days, 3 months) before yearly (2 obs,
355-375 days, 12 months), plus a ≤10% amount band. Requiring ADJACENT
evidence (not a chosen subsequence) is what stops weekly same-merchant
noise from being thinned into a false monthly candidate.

`crates/store/src/recurring/schedule.rs` computes every occurrence date
directly from the anchor (never by repeatedly adding a month to a
previous, possibly-clamped occurrence), with month-end clamping and
Feb-29 leap-year handling.

`crates/store/src/recurring/matching.rs` walks the occurrence schedule
forward, matching each occurrence to the sole eligible observation within
±10 days; two or more eligible observations for one occurrence make the
whole row `unknown/ambiguous_membership`. Coverage-based state precedence
(`crates/store/src/recurring/coverage.rs` for trusted-range merging) follows
recurring-contract.md §5 exactly: unresolved uncovered elapsed window wins
over everything, then two proven-missed windows (`ended`), then one
(`missing`), then an in-window next occurrence (`upcoming`), else `active`
(with `grace_until` set while a due date is still inside its own ±10-day
window).

Stable-price detection (`matching::detect_price_change`) and the projected
amount basis (`matching::stable_or_latest_index`) both work off the
group's own evidence; since `currency_basis` is part of `group_key`, a
`foreign_unknown` transaction can never land in the same group as a clean
`original:<CODE>` or `eur` group, so a missing-original observation can
never corrupt an existing stable pair.

## Money and KPIs

`crates/store/src/recurring/money.rs`: i128 intermediates, half-round-up,
a hard reject outside `Number.MAX_SAFE_INTEGER` before any monetary value
crosses the IPC boundary. Annual projection always multiplies the ORIGINAL
observed amount by the cadence's per-year count (never `monthly*12`).

`crates/store/src/recurring/amounts.rs` aggregates confirmed/estimate
monthly/annual/net/remaining totals from the assembled rows, and computes
the expense-share denominator (complete calendar months, wholly inside the
finite selected range or from the earliest trusted statement, strictly
before the `as_of` month, with every account of the selected kind trusted
covered for the whole month) by reusing `Store::summary_filtered`'s
existing, already-tested expense arithmetic rather than reimplementing it.

## Persistence

`crates/store/src/recurring/persist.rs`: `save_recurring` validates every
target transaction (exists, not transfer/refund/zero), rejects a mixed
account/direction/currency selection, rejects a `selected`-scope member
already claimed by another `selected` decision, refuses to let an edit
change an existing decision's account or scope, and does the whole write
(upsert group decision, or upsert+replace selected members) inside one
`rusqlite::Transaction`. `reset_recurring` deletes the decision row
(members cascade via `ON DELETE CASCADE`); a missing id is a clear error,
never a silent no-op success.

## Wire contract and commands

`crates/store/src/recurring/types.rs` mirrors recurring-contract.md §7's
TypeScript/Rust snippets field-for-field.
`src-tauri/src/recurring_commands.rs` exports the five thin command
wrappers (`recurring_overview`, `recurring_detail`,
`transaction_recurring_context`, `save_recurring`, `reset_recurring`),
registered in `src-tauri/src/lib.rs`.

## Known limitations (honest, not silently cut)

- `TransactionRecurringContext::ambiguous`/`inferred_cadence` reuse the
  same detection/matching helpers as the overview row builder but are
  exercised only by unit-level reasoning here, not by a dedicated
  `recurring_json`/native test; B's UI integration is the natural place to
  add end-to-end coverage once it mounts `RecurringTransactionAction`.
- The `unmatched_history` `UnknownReason` variant exists in the wire type
  (contract §7) but this implementation never emits it; every scenario
  observed in the acceptance rubric that could plausibly need it resolved
  to `no_evidence`, `missing_coverage` or `ambiguous_membership` instead.
  Flagged here rather than silently dropped from the enum.
- Acceptance scenarios R02, R05 (full geometry), R06, R11, R13, R15, R16
  are covered by the underlying pure-logic unit tests (schedule/coverage/
  matching/money modules) but do not each have a dedicated end-to-end
  `crates/store/tests/recurring_*.rs` scenario by ID. R01, R03, R04, R07,
  R08, R09, R12, M01, M02, M03 do.
- `RecurringOverview.history_from` is the earliest ELIGIBLE evidence
  transaction date (status/kind/amount filters applied, optionally scoped
  to `account_kind`), not specifically the earliest TRUSTED statement
  date; the two coincide whenever the earliest statement's checksum is
  `ok`, which is the common case, but the contract's "earliest available
  detection date" wording is not pinned to a dedicated test.
