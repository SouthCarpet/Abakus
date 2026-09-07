# 012 Recurring payments UI

Lane B of Abakus 0.1.2 (worktree `080-recurring-ui`, contract
`A:/projects-vault/animus/data/agent-runs/abakus-012-20260907/recurring-contract.md`).
Adds the Prehľad panel, exact-membership detail, shared editor and the
transaction-row action that lane C mounts after `NoteEditor`. Detection,
persistence and money arithmetic stay in lane A. Category creation UI stays
in lane C's `CategoryDialog`.

## What shipped

- `src/lib/recurring-api.ts`: TypeScript wire types matching the contract
  structs one-for-one, plus `recurringApi` invoke wrappers
  (`recurring_overview`, `recurring_detail`, `transaction_recurring_context`,
  `save_recurring`, `reset_recurring`). Outer invoke arguments stay camelCase;
  object fields stay snake_case.
- `src/lib/recurring-labels.ts`: Slovak copy and integer money/percent
  formatting. No float division of cents. Historical remaining-month labels
  name the month (`Zostávalo v auguste 2026`) and never call a past month
  "tento mesiac".
- `src/components/recurring/RecurringPanel.tsx`: mounted from Overview even
  when the selected-period transaction summary is empty. Generation-guarded
  fetch so a slower earlier account/period response cannot overwrite the
  latest. Loading and query failure show a status/alert, never `0,00 €`
  totals. Future-only ranges explain that the panel is empty rather than
  showing current data under a future heading. Unfinished ranges show
  `Obdobie ešte neskončilo`.
- `src/components/recurring/RecurringKpis.tsx`: confirmed monthly/annual
  expense and income, net, remaining-month charges and receipts (confirmed
  vs `Ďalšie odhady`), exclusion counts, and expense-share from backend
  basis points. The annual toggle changes projected totals and net, not
  remaining-month amounts. Share `null` reads as unavailable, never `0 %`.
- `src/components/recurring/RecurringTable.tsx`: expenses, incomes, estimates
  and a collapsed Ignorované list. Interval, actual charge (original
  currency beside booked EUR), monthly apportionment for quarterly/yearly
  rows, last/next dates, state text (not color-only), price-change badge,
  foreign EUR estimate caption. Estimate actions: confirm, edit cadence,
  ignore. Confirmed: edit, ignore, reset. Ignored: edit, restore.
- `src/components/recurring/RecurringDetail.tsx`: exact member table for the
  selected period or full known history. The switch is explicit; the
  displayed period is never silently widened. Compatible rows are listed
  separately. Same-name rows on another account are not treated as members.
- `src/components/recurring/RecurringEditor.tsx`: cadence, anchor, selected
  membership, ignore and reset. Dialog drafts survive a failed save.
  Mutation success plus refresh failure reads as saved, and retrying the
  same draft only refreshes. Busy state disables duplicate saves and
  membership edits. Category assignment is a separate explicit step and
  never runs as a side effect of confirming recurrence.
- `src/components/recurring/RecurringTransactionAction.tsx`: public props
  `{ row: TxRow; categories: Category[]; onChanged: () => Promise<void> }`.
  Renders nothing for transfer, refund or zero rows. Loads context, opens
  the shared editor.

## Actions and estimate vs confirmed

An **Odhad** is detection output, including Predplatné. It is not a confirmed
total. Confirm writes a decision with the shown cadence and anchor. Edit
opens the dialog so the user can change interval, anchor or (for selected
scope) membership. Ignore hides the row under Ignorované and removes it
from all totals. Reset returns control to detection; one observation then
produces no candidate.

Confirmed rows with missing statement evidence stay visible as unknown.
They do not fall back to an estimate in the UI. The backend owns that
rule; the panel only renders the returned `state` and `unknown_reason`.

## Historical as-of

The panel is a schedule snapshot at `as_of`, not a sum of transactions
inside the filter. Changing only `from` refetches (detail range and expense
share may change) but the query still sends the same `today`. Remaining
charges use the backend remaining-month cents. If `as_of` is in the same
calendar month as `today`, the tile says `Ešte príde tento mesiac`.
Otherwise it names the month.

## Manual membership

Blank identity, ambiguous membership and existing `scope: selected` rows
force "Len označené transakcie". The editor lists compatible transactions
and explains `Ďalšie platby priraďte ručne`. Selected scope never claims
future matching rows in this UI.

## Original currency and projection

Foreign rows show original cents and ISO code beside booked EUR. A
`foreign_eur_estimate` row adds `Prepočet podľa poslednej platby; kurz sa
môže zmeniť`. Quarterly and yearly rows show the backend `monthly_cents`
as `mesačne (štvrťročne|ročne)`, never a frontend `amount/12`.

## CategoryDialog seam

Lane C owns `src/components/CategoryDialog.tsx`. This lane does not ship a
placeholder. `RecurringEditor` accepts an optional `CategoryDialog`
component with the contract props `{ open, initialParentId?, initialKind?,
onClose, onCreated }`. After parent copies C's file into this tree, pass
that export into `RecurringEditor` from `RecurringTransactionAction` and
from `RecurringPanel`. Until then, "Nová kategória..." is hidden; explicit
assignment of an existing category still works through `CategoryPicker` and
`api.assign([id], categoryId, false)`.

## Honest limits

- No network, scheduler, notifications, bank sync, budgets or restore UI.
- Amounts on screen are backend integers. The UI does not recompute
  monthly/annual totals.
- Keyboard: real buttons, existing `Dialog` focus trap, Escape, restored
  focus. Dense tables scroll inside a widened dialog (`k-dialog` is 300px
  in Kaliber; this lane uses `:has([data-recurring-wide])` with space
  tokens so 1024 and 1280 layouts can show a member table).
- `Dialog.tsx` and `kaliber.css` are not owned here, so the width override
  is scoped CSS rather than a new dialog API.
- Native acceptance, installer and README integration are parent-owned.
- PDF export is a separate lane; Overview was not reserved for it.
