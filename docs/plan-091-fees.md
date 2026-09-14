# Plan 091: backend bank fees

This step gives recognized bank fees their own transaction kind, filter, and
summary fields. The Transactions kind filter now offers `fee` (label
"Poplatok") alongside every other `TxKind`, and Overview shows a "Poplatky"
KPI tile plus a "Poplatky" series in the Príjmy a výdavky chart, both reading
`Summary.fee_cents`/`by_month[].fee_cents` directly. No new chart type was
added; the fee data is surfaced through the existing KPI and bar-chart
components.

## Recognition and compatibility

- `TxKind::Fee` serializes as `fee`.
- A valid transaction first line is a fee when `parser::fold(description)`
  starts with `poplat`. This keeps the bank wording already accepted by the
  parser.
- A truly unknown description stays `TxKind::Other` and keeps the Slovak
  unknown-type warning.
- The transaction fingerprint remains SHA-256 over IBAN, posted date, amount,
  and normalized raw block. Kind is not part of it. Importing the same row
  after the parser starts returning `Fee` remains a duplicate.

## Additive API inventory

- `TxKind`: new enum value `fee`, also included in CLI kind counts.
- `parser::raw_block_is_fee`: public recognition helper for the legacy
  migration. It validates the transaction first line before testing its description.
- `TxFilter.kind`: optional exact transaction-kind filter. Omitted or `null`
  keeps all kinds. It intersects with date, account, account kind, category,
  status, statement, and literal text filters.
- `Summary.fee_cents`: signed integer cents contributed by fee rows to the
  existing expense total in the selected account and inclusive date scope.
- `Summary.by_month[].fee_cents`: the same signed fee contribution grouped by
  transaction month. It feeds the Poplatky bar series in the IncomeExpense chart on the Overview screen.
- Schema rule: version 5 changes only `transactions.kind` from `other` to
  `fee` when the stored raw block parses as a recognized fee and status is not
  `transfer`.

## Money semantics

`fee_cents` is a subset of `expense_cents`, not an extra total. A negative fee
amount contributes a positive expense magnitude. A positive correction in an
expense category contributes a negative amount to both fields. A fee cannot
add a second contribution to income or transfer totals. Existing category
classification still controls those totals. `net_cents` remains `income_cents -
expense_cents`.

## Legacy data guarantees

The version 5 backfill preserves transaction ID, statement and account links,
fingerprint, dates, signed amount, original currency fields, merchant and
counterparty fields, category, status, rule, source, raw block, and note. It
skips every transfer-status row even if synthetic damaged legacy data gives
that row fee-like raw text. Initialization keeps the migration atomic. A
failed write leaves both the rows and schema version unchanged, and a later
reopen can retry it.

## Verification

Synthetic tests cover recognized fee versus unknown parsing, filter
composition and CSV parity, date and account scopes, positive corrections,
income and transfer isolation, stable reimport deduplication, exact legacy
column preservation, transfer protection, rollback, retry, and idempotent
reopen. No private statement fixture or user database is read.

The frontend kind filter is covered by `src/screens/Transactions.test.tsx`
("offers Poplatok in the kind filter and sends kind: fee to
listTransactions"); the Overview KPI and chart series are covered by
`src/screens/Overview.test.tsx` ("Overview shows fee totals (point 13)") and
`src/components/charts/IncomeExpense.test.tsx`.
