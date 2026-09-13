---
title: Plan 091, calendar-year comparison helper
status: shipped
plan: 091_abakus-020-ux
point: 20
---

# Scope

`yearComparisonRanges` prepares the two date ranges needed for a comparison with the same period one year earlier. This is an additive helper. The existing `previousEqualRange` contract and its callers keep the immediately preceding range with the same number of days.

The UI integration is shipped in the YearComparisonCard component (see UI wiring below). The helper itself does not fetch summaries, select an account kind, or render the period choice and date labels; the card owns those responsibilities.

# Input

- `selectedMonth` is an exact `YYYY-MM` calendar month. Invalid text and invalid month numbers cause a `RangeError` instead of date rollover.
- `period` uses the exported `YearComparisonPeriod` type: `month` or `three-months`.
- `month` selects the full calendar month.
- `three-months` selects three full calendar months ending in the selected month.
- Both current and previous periods must fit the supported years 0000 through 9999. A range that would cross this boundary causes a `RangeError`.

# Output

The helper returns a `YearComparisonRanges` value with two explicit inclusive ranges:

- `current` is the selected full month or the three-month window that ends in it.
- `previous` contains the same calendar month or months one year earlier.
- Each range has `from` and `to` dates in `YYYY-MM-DD` format.

Calendar boundaries are calculated without local time. February uses 28 or 29 days for its own year. A three-month window can cross a year boundary. For example, January 2026 produces November 2025 through January 2026, compared with November 2024 through January 2025.

# UI wiring

`YearComparisonCard` (`src/components/insights/YearComparisonCard.tsx`) provides
the month input and the one-month/three-month choice, fetches `api.summary`
once for each returned range with the current account-kind filter, and shows
both ranges next to income, expense and net totals. It lives directly in
Overview, next to `InsightsPanel`, because its own month/period controls are
independent of Overview's `PeriodPicker` (unlike `CategoryComparisonCard`,
whose comparison follows that picker). No new aggregation API was needed:
the existing arbitrary-range `api.summary(from, to, accountId, accountKind)`
covers both ranges.
