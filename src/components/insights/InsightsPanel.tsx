import { useEffect, useMemo, useState } from 'react'
import type { Account, AccountKind, StatementHistoryRow, Summary } from '../../api'
import { api } from '../../api'
import type { DateRange } from '../../lib/insights/date'
import { anyGaps, buildCoverage } from '../../lib/insights/coverage'
import { compareCategories, previousEqualRange, type CategoryComparison } from '../../lib/insights/comparison'
import { statementsOverlappingWindow } from '../../lib/insights/balances'
import { periodRange, validPeriod } from '../../lib/period'
import type { PeriodValue } from '../PeriodPicker'
import { CoverageCard } from './CoverageCard'
import { CategoryComparisonCard } from './CategoryComparisonCard'
import { BalanceHistoryCard } from './BalanceHistoryCard'

const todayIso = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`

function periodWindow(period: PeriodValue): DateRange | null {
  if (!validPeriod(period)) return null
  const { from, to } = periodRange(period.kind, new Date(), period.custom)
  return from && to ? { from, to } : null
}

function ComparisonSection({
  comparisonRange,
  previousError,
  comparison,
  currentCoverageIncomplete,
  previousCoverageIncomplete,
}: {
  comparisonRange: DateRange | null
  previousError: string
  comparison: CategoryComparison | null
  currentCoverageIncomplete: boolean
  previousCoverageIncomplete: boolean
}) {
  if (!comparisonRange) return null
  if (previousError) return <p role="alert">{previousError}</p>
  if (!comparison) return null
  return (
    <CategoryComparisonCard
      comparison={comparison}
      currentCoverageIncomplete={currentCoverageIncomplete}
      previousCoverageIncomplete={previousCoverageIncomplete}
    />
  )
}

/**
 * Coverage and balance history come only from statementHistory + listAccounts
 * and render whenever those resolve, independent of the transaction summary.
 * Category comparison additionally needs the current period's summary (owned
 * by Overview, passed in) and its own previous-range summary fetch. Each of
 * these three sources keeps its own error state, so one failing fetch never
 * reads as an honest empty or complete result for another.
 */
export function InsightsPanel({
  period,
  accountKind,
  summary,
}: {
  period: PeriodValue
  accountKind: 'all' | AccountKind
  summary: Summary | null
}) {
  const kind = accountKind === 'all' ? null : accountKind
  const window = useMemo(() => periodWindow(period), [period])
  const comparisonRange = useMemo<DateRange | null>(() => (period.kind === 'all' ? null : window), [period.kind, window])

  const [accounts, setAccounts] = useState<Account[]>([])
  const [accountsError, setAccountsError] = useState('')
  const [history, setHistory] = useState<StatementHistoryRow[] | null>(null)
  const [historyError, setHistoryError] = useState('')
  const [previousSummary, setPreviousSummary] = useState<Summary | null>(null)
  const [previousError, setPreviousError] = useState('')

  useEffect(() => {
    void api.listAccounts().then(setAccounts).catch((e) => setAccountsError(String(e)))
  }, [])

  useEffect(() => {
    let active = true
    setHistory(null); setHistoryError('')
    void api.statementHistory(null, kind).then((rows) => { if (active) setHistory(rows) }).catch((e) => { if (active) setHistoryError(String(e)) })
    return () => { active = false }
  }, [kind])

  useEffect(() => {
    let active = true
    setPreviousSummary(null); setPreviousError('')
    if (comparisonRange) {
      const prev = previousEqualRange(comparisonRange)
      void api.summary(prev.from, prev.to, null, kind).then((v) => { if (active) setPreviousSummary(v) }).catch((e) => { if (active) setPreviousError(String(e)) })
    }
    return () => { active = false }
  }, [comparisonRange, kind])

  const coverages = useMemo(() => (history ? buildCoverage(accounts, history, kind, window) : []), [accounts, history, kind, window])
  const previousCoverages = useMemo(
    () => (history && comparisonRange ? buildCoverage(accounts, history, kind, previousEqualRange(comparisonRange)) : []),
    [accounts, history, kind, comparisonRange],
  )
  const balanceRows = useMemo(() => (history ? statementsOverlappingWindow(history, window) : []), [history, window])
  const comparison = useMemo(
    () => (comparisonRange && summary && previousSummary ? compareCategories(comparisonRange, todayIso(new Date()), summary, previousSummary) : null),
    [comparisonRange, summary, previousSummary],
  )

  return (
    <div className="k-section">
      {accountsError ? <p role="alert">{accountsError}</p> : null}
      {historyError ? <p role="alert">{historyError}</p> : null}
      {history ? <CoverageCard coverages={coverages} allTime={window === null} /> : null}
      {history ? <BalanceHistoryCard statements={balanceRows} /> : null}
      <ComparisonSection
        comparisonRange={comparisonRange}
        previousError={previousError}
        comparison={comparison}
        currentCoverageIncomplete={anyGaps(coverages)}
        previousCoverageIncomplete={anyGaps(previousCoverages)}
      />
    </div>
  )
}
