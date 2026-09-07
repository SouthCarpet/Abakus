import type { Account, AccountKind, StatementHistoryRow } from '../../api'
import { addDaysIso, type DateRange } from './date'

export interface AccountCoverage {
  accountId: number
  accountLabel: string
  accountKind: AccountKind
  hasStatements: boolean
  knownFrom: string | null
  knownTo: string | null
  gaps: DateRange[]
  invalidRanges: DateRange[]
}

/**
 * Merges overlapping, nested, duplicate and immediately adjacent inclusive
 * ranges. A reversed (from > to) range never merges and is reported
 * separately: it must never grant coverage.
 */
export function mergeRanges(ranges: DateRange[]): { merged: DateRange[]; invalid: DateRange[] } {
  const invalid = ranges.filter((r) => r.from > r.to)
  const valid = ranges.filter((r) => r.from <= r.to)
  const sorted = [...valid].sort((a, b) => (a.from < b.from ? -1 : a.from > b.from ? 1 : 0))
  const merged: DateRange[] = []
  for (const range of sorted) {
    const last = merged[merged.length - 1]
    if (last && range.from <= addDaysIso(last.to, 1)) {
      if (range.to > last.to) last.to = range.to
    } else {
      merged.push({ ...range })
    }
  }
  return { merged, invalid }
}

function clipToWindow(merged: DateRange[], window: DateRange): DateRange[] {
  const clipped: DateRange[] = []
  for (const range of merged) {
    const from = range.from < window.from ? window.from : range.from
    const to = range.to > window.to ? window.to : range.to
    if (from <= to) clipped.push({ from, to })
  }
  return clipped
}

/** Gaps inside a finite window, including the leading and trailing edges. A
 * range that only partially overlaps the window contributes only the part
 * actually inside it. */
export function gapsWithinWindow(merged: DateRange[], window: DateRange): DateRange[] {
  const clipped = clipToWindow(merged, window)
  const gaps: DateRange[] = []
  let cursor = window.from
  for (const range of clipped) {
    if (range.from > cursor) gaps.push({ from: cursor, to: addDaysIso(range.from, -1) })
    cursor = addDaysIso(range.to, 1)
  }
  if (cursor <= window.to) gaps.push({ from: cursor, to: window.to })
  return gaps
}

/** Gaps for the Všetko (all-time) view: only between known ranges, never
 * invented before the first or after the last. */
export function internalGaps(merged: DateRange[]): DateRange[] {
  const gaps: DateRange[] = []
  for (let i = 1; i < merged.length; i++) {
    gaps.push({ from: addDaysIso(merged[i - 1].to, 1), to: addDaysIso(merged[i].from, -1) })
  }
  return gaps
}

/** `window: null` means Všetko (all-time): report only internal gaps and the
 * known-history bounds. A finite window reports edge gaps too. Coverage is
 * built strictly from this one account's own statements; it is never
 * borrowed from another account. */
export function buildAccountCoverage(
  account: Pick<Account, 'id' | 'label' | 'kind'>,
  statements: StatementHistoryRow[],
  window: DateRange | null,
): AccountCoverage {
  const rows = statements.filter((s) => s.account_id === account.id)
  const { merged, invalid } = mergeRanges(rows.map((s) => ({ from: s.period_start, to: s.period_end })))
  const base = {
    accountId: account.id,
    accountLabel: account.label,
    accountKind: account.kind,
    hasStatements: rows.length > 0,
    invalidRanges: invalid,
  }
  if (merged.length === 0) {
    return { ...base, knownFrom: null, knownTo: null, gaps: window ? [window] : [] }
  }
  const gaps = window ? gapsWithinWindow(merged, window) : internalGaps(merged)
  return { ...base, knownFrom: merged[0].from, knownTo: merged[merged.length - 1].to, gaps }
}

export function buildCoverage(
  accounts: Pick<Account, 'id' | 'label' | 'kind'>[],
  statements: StatementHistoryRow[],
  accountKind: AccountKind | null,
  window: DateRange | null,
): AccountCoverage[] {
  return accounts
    .filter((a) => accountKind === null || a.kind === accountKind)
    .map((a) => buildAccountCoverage(a, statements, window))
}

export function anyGaps(coverages: AccountCoverage[]): boolean {
  return coverages.some((c) => c.gaps.length > 0)
}
