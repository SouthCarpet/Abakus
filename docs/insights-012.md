# 012 Overview insights (coverage, category comparison, balance history)

Insights lane of Abakus 0.1.2 (worktree `080-insights`, branch
`feature/080-insights`). Adds three read-only insight views to Prehľad
(Overview): statement coverage per account, an expense category comparison
against the previous equal-length period, and a statement closing-balance
table. All three read existing backend endpoints only (`statement_history`,
`list_accounts`, `summary`); no backend change.

## What shipped

- `src/lib/insights/date.ts`: UTC calendar-day arithmetic (`addDaysIso`,
  `dayCountInclusive`, `rangesOverlap`, `isValidRange`) for inclusive
  `YYYY-MM-DD` ranges. Always `Date.UTC`, never the local constructor, so a
  leap day or a DST transition in the host's local time zone never shifts a
  day count.
- `src/lib/insights/coverage.ts`: merges overlapping, nested, duplicate and
  immediately-adjacent statement date ranges per account
  (`mergeRanges`), then reports gaps two ways: `internalGaps` for Všetko
  (all-time) reports only gaps between the account's own known ranges, never
  a gap invented before the first or after the last; `gapsWithinWindow` for a
  finite selected period reports leading/trailing edge gaps too, and clips a
  range that only partially overlaps the window to its actual coverage. A
  reversed (`from > to`) statement range never merges into coverage; it is
  set aside in `invalidRanges` and flagged for review instead.
  `buildAccountCoverage`/`buildCoverage` build this strictly per account (an
  account can never borrow another account's coverage) and, given `list
  Accounts`, cover an account with zero statements as `hasStatements: false`
  (rendered "Bez výpisov"), not silently dropped or shown as complete.
- `src/lib/insights/comparison.ts`: `previousEqualRange` returns the
  immediately preceding inclusive range with the same UTC calendar-day
  count as the current one (not calendar-month aligned: a 28-day February
  compares against the last 28 days of January, day-4 through day-31, not
  all of January). `compareCategories` unions current and previous
  `Summary.by_category` rows by category id (a category present on only one
  side still gets a row), adds a non-drilldown residual row
  ("Nezaradené výdavky") equal to `expense_cents` minus the categorized sum
  so the table reconciles, and computes `percentDelta` as
  `(current - previous) / abs(previous) * 100`, returning `null` (never `0%`
  or `Infinity`) when `previous` is `0`. `isIncompletePeriod` flags a range
  whose end has not fully elapsed (today or later) as incomplete.
- `src/lib/insights/balances.ts`: `statementsOverlappingWindow` selects and
  sorts (by account, then period) the statements whose period overlaps the
  selected window, or every statement for Všetko. Closing balance and
  checksum are passed through exactly as the statement reported them: never
  summed, interpolated or replaced with a live total.
- `src/components/insights/CoverageCard.tsx`,
  `CategoryComparisonCard.tsx`, `BalanceHistoryCard.tsx`: plain
  `k-table`/`k-well` tables, no new design system, no chart. The balance
  table's caption states explicitly that a row is the statement's closing
  balance, not the account's current balance. The comparison table shows
  both compared date ranges, a `%` column that reads "nedostupné" instead of
  a percent when the previous total is zero, and one warning line each for
  an unfinished current period, a coverage gap in the current window, and a
  coverage gap in the previous window.
- `src/components/insights/InsightsPanel.tsx`: owns the three new fetches
  (`listAccounts`, `statementHistory`, and a previous-range `summary` call)
  and mounts under Overview, one `useEffect`/`active`-flag pair per fetch
  (the same guard `Overview.tsx` already uses for its own `summary` fetch),
  so a slow response for an account-kind selection the user has since
  changed can never overwrite the newer selection's state. Each of the three
  sources keeps its own error state: a failed `statementHistory` fetch shows
  an alert and hides the coverage/balance cards, it does not fall back to an
  empty-looking (and therefore falsely "complete") table. Coverage and
  balance history render as soon as they load, independent of the
  transaction summary; the category comparison additionally needs the
  current period's `summary` (passed down from `Overview.tsx`, not
  re-fetched) and its own previous-range fetch. Všetko has no comparison:
  `comparisonRange` is `null` for `period.kind === 'all'`, and no previous
  `summary` call is made for it.
- `src/screens/Overview.tsx`: renders `<InsightsPanel>` unconditionally,
  after (not inside) the existing summary/empty-state block, so coverage and
  balance history stay visible under the "Zatiaľ nič" / "Za vybrané obdobie
  nič nie je" empty-overview card, and even while the transaction summary is
  still loading. No existing Overview behavior (account-kind filter, period
  filter, drill-downs, empty-state copy) changed.

## Period semantics

- **Coverage.** Všetko (window `null`): gaps only between the account's own
  earliest and latest known range; the known-history bounds ("Známa
  história") are stated explicitly, no gap is invented before the first
  range or after the last. A finite selected period: gaps within exactly
  that period, including a leading gap (before the first statement inside
  the window), a trailing gap (after the last), and an account with zero
  statements in scope (whole window is a gap, labelled "Bez výpisov"
  distinctly from a gap on an account that does have statements).
- **Comparison.** The previous range has the same UTC calendar-day count as
  the currently displayed range, immediately before it; differing month
  lengths are shown by displaying both date ranges, not hidden by aligning
  to calendar months. A current range whose end has not fully elapsed
  (today or later) is marked incomplete. Všetko has no comparison at all.
- **Balances.** The table lists statements whose period overlaps the
  selected window (every statement, for Všetko), sorted by account then by
  period. A statement's `closing_cents: null` renders "Nedostupný"
  (unavailable), never a substituted or synthesized value.

## Empty, error and coverage limitations (honest by design)

- A statement-history or account-list fetch failure shows a role="alert"
  message and hides the coverage/balance cards entirely; it never renders an
  empty-looking table that could be misread as "fully covered" or "no
  statements".
- A previous-range `summary` fetch failure shows its own alert and hides
  only the comparison card; coverage and balance history are unaffected
  (they do not depend on `summary` at all).
- Coverage status is derived only from statement date ranges, never from
  transaction counts, so it cannot claim verified completeness the backend
  never asserted.
- A reversed/invalid statement date range is set aside and listed under "Na
  kontrolu" (for review) on its account's coverage row; it never counts
  toward coverage.
- The comparison table's residual "Nezaradené výdavky" row is not a
  drilldown target (there is no real category id for it); the rest of the
  comparison rows are a plain table, not clickable, matching the "simple
  contained table" scope of this change.
- No new backend field, no new IPC command, no new npm dependency. Internal
  transfers are excluded upstream by `Summary.by_category` already; nothing
  here re-implements that exclusion.

## Tests (2026-09-07)

81 tests across 6 owned files, all passing:

| File | Tests |
| --- | --- |
| `src/lib/insights/date.test.ts` | 15 |
| `src/lib/insights/coverage.test.ts` | 23 |
| `src/lib/insights/comparison.test.ts` | 20 |
| `src/lib/insights/balances.test.ts` | 6 |
| `src/components/insights/InsightsPanel.test.tsx` | 8 |
| `src/screens/Overview.test.tsx` | 9 (8 pre-existing + 1 new) |

Coverage highlights: range overlap/nesting/duplicate/adjacency merging, a
reversed range flagged instead of merged, an account borrowing no coverage
from another, all-time vs. finite-window gap rules (no invented edge gaps
for Všetko, edge + internal gaps for a finite period, a partially-overlapping
range clipped to actual coverage), leap-day and DST-date boundaries in the
UTC day arithmetic, equal-day-count previous-range math across a shorter
month/a leap year/a year boundary, category union in both directions
(current-only, previous-only), a refund-to-expense sign flip, the residual
row reconciling to `expense_cents`, a zero previous denominator returning
`null` (never `0%`/Infinity), an unfinished vs. fully-elapsed period, a
`closing_cents: null` staying unavailable, and (at the component level) the
independent-fetch guarantees: content visible with an empty/unloaded
summary, a distinct error instead of a false-empty table on fetch failure, a
slow stale account-kind response never overwriting a newer selection, and no
comparison fetch at all for Všetko.

## Commands and results (2026-09-07)

```powershell
node node_modules/vitest/vitest.mjs run --configLoader runner src/lib/insights src/components/insights src/screens/Overview.test.tsx
# 6 files, 81 tests, all passing
node node_modules/typescript/bin/tsc -b --pretty false
# 1 pre-existing error, unrelated: src/screens/Audit078.test.tsx(27,3), a
# TxRow fixture missing the `note` field this worktree's api.ts already
# requires; not an insights-owned file
node node_modules/eslint/bin/eslint.js src/screens/Overview.tsx src/screens/Overview.test.tsx src/components/insights src/lib/insights
# clean, 0 problems (complexity <= 12, max-depth <= 3 throughout)
```

## Honest limits

- Rendered appearance is not verified here: no screenshot or visual review
  was taken. That is a separate, independent step per the run contract.
- `App.test.tsx` and `src/screens/Audit078.test.tsx` mock `../api`/`./api`
  without `listAccounts`/`statementHistory`; since `InsightsPanel` now
  mounts unconditionally under `Overview`, their 4 pre-existing tests that
  render `Overview` (directly or via `App`) fail with
  `api.listAccounts is not a function` / `api.statementHistory is not a
  function`. Both files are outside this lane's ownership (Overview and new
  insight files only); see `insights-report.md` for the exact lines needing
  a mock addition.
- The comparison card's coverage warnings ("chýbajú výpisy aspoň pre jeden
  účet") are a boolean per range (at least one in-scope account has a gap),
  not a breakdown of which account or how much of the range is missing; a
  user who wants that detail already has the coverage card above it.
- No drill-down from any of the three new cards into Transactions. The
  contract asked for "a simple contained table", and the existing
  category/month charts already cover click-through elsewhere on the same
  screen.
