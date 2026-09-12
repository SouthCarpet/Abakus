# Statement coverage reminders

## Scope

`buildStatementReminders(accounts, statements, accountKind, today)` is a pure,
source-only helper for recent statement coverage checks. The caller supplies a
valid local-calendar `today` value in `YYYY-MM-DD` form and complete, successfully
loaded statement history. The helper does not read the clock, fetch data, store
state, schedule work, or infer completeness from transaction counts or checksums.
An empty result is not proof that bank data is complete.

The helper targets only the immediately previous completed calendar month. Seven
full days after month end form the grace period, so missing-range reminders start
on day 8. January targets December of the previous year, and calendar arithmetic
preserves leap years and supported years `0000..9999`. January in year `0000` has
no supported previous month, so `targetMonth` is `null`. An invalid `today` value
throws `RangeError`.

For each selected account, history contributes only when both the account ID and
account kind match. Account input order is preserved. The checked interval starts
at the later of the target month's first day and the earliest valid range start.
This clipping avoids an invented gap before the first known statement. Empty,
invalid-only, and future-only history does not create a missing-range reminder.
Earlier valid history can establish the start needed to report real gaps in the
target month. Full target coverage and coverage of the previous month suppress a
reminder, even when older history contains gaps.

The helper reuses the existing inclusive range merge and finite-window gap logic.
It returns exact uncovered ranges. It does not claim that transactions are
missing or that a bank must issue one statement each month. Any invalid range for
a selected account appears in `review` and conservatively suppresses that
account's reminder until review. Other accounts remain independent, and review
data remains available during the grace period.

This helper does not change `buildCoverage`, `buildAccountCoverage`, or the
all-time Všetko view. All-time coverage continues to show internal gaps only. No
UI or native application integration is included in this source-only change.

## Additive surface inventory

- Function `buildStatementReminders`: builds the source-only result from accounts,
  complete statement history, an account-kind filter, and injected `today`.
- Type `StatementReminderResult`: contains the target and the two output lists.
- Field `StatementReminderResult.targetMonth`: the previous completed month, or
  `null` when no supported previous month exists.
- Field `StatementReminderResult.reminders`: account reminders with exact missing
  ranges.
- Field `StatementReminderResult.review`: account ranges that need review.
- Type `StatementReminder`: identifies one account with missing target ranges.
- Field `StatementReminder.accountId`: the selected account ID.
- Field `StatementReminder.accountLabel`: the selected account label.
- Field `StatementReminder.accountKind`: the selected account kind.
- Field `StatementReminder.missingRanges`: exact uncovered inclusive target
  ranges.
- Type `StatementRangeReview`: identifies invalid ranges for one account.
- Field `StatementRangeReview.accountId`: the selected account ID.
- Field `StatementRangeReview.accountLabel`: the selected account label.
- Field `StatementRangeReview.accountKind`: the selected account kind.
- Field `StatementRangeReview.invalidRanges`: invalid same-account ranges that do
  not grant coverage.
- Rule `previous completed month`: only the month immediately before `today` is
  checked.
- Rule `day-8 start`: reminders remain empty through day 7 and start on day 8.
- Rule `first-history clipping`: the checked start never precedes the earliest
  valid range for that account.
- Rule `invalid-range review`: invalid same-account history produces review data
  and suppresses that account's reminder.
- Rule `account isolation`: both account ID and kind must match, and the optional
  kind filter controls which accounts are returned.
- Rule `complete-history input`: callers provide complete successfully loaded
  history rather than a UI-filtered subset or a failed fetch represented as empty.
- Rule `unchanged all-time coverage`: existing internal-gap behavior remains
  unchanged.

## Verification boundary

Deterministic Vitest cases cover the grace boundary, month and year rollover,
ordinary and leap February, low and minimum years, invalid dates, account and kind
isolation, first-history clipping, exact merged gaps, invalid-range review,
complete and partial coverage, empty and future-only history, input immutability,
and previous-month-only scope. Reused coverage and date tests guard the shared
range behavior. TypeScript and ESLint checks cover the new source. These checks do
not provide UI, native application, packaged application, or installer acceptance.
