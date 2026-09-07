import type { StatementHistoryRow } from '../../api'
import { rangesOverlap, type DateRange } from './date'

/** Statements whose period overlaps `window` (or every statement, for the
 * Všetko view where `window` is null), sorted by account, then period. The
 * closing balance and checksum stay exactly as the statement reported them:
 * this never sums or interpolates across statements. */
export function statementsOverlappingWindow(statements: StatementHistoryRow[], window: DateRange | null): StatementHistoryRow[] {
  const scoped = window === null
    ? statements
    : statements.filter((s) => rangesOverlap({ from: s.period_start, to: s.period_end }, window))
  return [...scoped].sort((a, b) =>
    a.account_id - b.account_id
    || a.period_start.localeCompare(b.period_start)
    || a.period_end.localeCompare(b.period_end)
    || a.statement_id - b.statement_id)
}
