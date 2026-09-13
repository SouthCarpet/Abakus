import { useEffect, useMemo, useState } from 'react'
import type { AccountKind, Summary } from '../../api'
import { api, formatEur } from '../../api'
import type { DateRange } from '../../lib/insights/date'
import { percentDelta, yearComparisonRanges, type YearComparisonPeriod, type YearComparisonRanges } from '../../lib/insights/comparison'
import { formatDate } from '../../lib/format'
import { Button } from '../Button'
import { Card } from '../Card'

const PERIOD_OPTIONS: { id: YearComparisonPeriod; label: string }[] = [
  { id: 'month', label: 'Jeden mesiac' },
  { id: 'three-months', label: 'Tri mesiace' },
]

function rangeLabel(range: DateRange): string {
  return `${formatDate(range.from)}–${formatDate(range.to)}`
}

function percentText(percent: number | null): string {
  if (percent === null) return 'nedostupné'
  return `${percent > 0 ? '+' : ''}${percent.toFixed(1)} %`
}

// yearComparisonRanges throws when a boundary would fall outside the
// representable YYYY-MM-DD year range: that must read as an honestly
// unavailable comparison, never a crashed Overview.
function safeRanges(selectedMonth: string, period: YearComparisonPeriod): { ranges: YearComparisonRanges | null; error: string } {
  try {
    return { ranges: yearComparisonRanges(selectedMonth, period), error: '' }
  } catch (e) {
    return { ranges: null, error: String(e) }
  }
}

function TotalsRow({ label, current, previous }: { label: string; current: number; previous: number }) {
  return (
    <tr>
      <td>{label}</td>
      <td className="k-num">{formatEur(current)}</td>
      <td className="k-num">{formatEur(previous)}</td>
      <td className="k-num">{percentText(percentDelta(current, previous))}</td>
    </tr>
  )
}

export function YearComparisonCard({ accountKind, today }: { accountKind: AccountKind | null; today: string }) {
  const [selectedMonth, setSelectedMonth] = useState(today.slice(0, 7))
  const [period, setPeriod] = useState<YearComparisonPeriod>('month')
  const [current, setCurrent] = useState<Summary | null>(null)
  const [previous, setPrevious] = useState<Summary | null>(null)
  const [error, setError] = useState('')

  const { ranges, error: rangeError } = useMemo(() => safeRanges(selectedMonth, period), [selectedMonth, period])

  useEffect(() => {
    let active = true
    setCurrent(null); setPrevious(null); setError('')
    if (!ranges) return
    void Promise.all([
      api.summary(ranges.current.from, ranges.current.to, null, accountKind),
      api.summary(ranges.previous.from, ranges.previous.to, null, accountKind),
    ])
      .then(([c, p]) => { if (active) { setCurrent(c); setPrevious(p) } })
      .catch((e) => { if (active) setError(String(e)) })
    return () => { active = false }
  }, [ranges, accountKind])

  return (
    <Card title="Porovnanie s rovnakým obdobím vlani">
      <div className="k-row" role="group" aria-label="Dĺžka porovnávaného obdobia">
        <input
          className="k-input k-well"
          aria-label="Mesiac na porovnanie"
          type="month"
          value={selectedMonth}
          onChange={(e) => setSelectedMonth(e.target.value)}
        />
        {PERIOD_OPTIONS.map((option) => (
          <Button
            key={option.id}
            aria-pressed={period === option.id}
            variant={period === option.id ? 'primary' : 'secondary'}
            onClick={() => setPeriod(option.id)}
          >
            {option.label}
          </Button>
        ))}
      </div>
      {rangeError ? <p role="alert">{rangeError}</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      {ranges && current && previous ? (
        <div className="k-table-scroll">
          <table className="k-table">
            <thead>
              <tr>
                <th></th>
                <th>{rangeLabel(ranges.current)}</th>
                <th>{rangeLabel(ranges.previous)}</th>
                <th className="k-num">Zmena</th>
              </tr>
            </thead>
            <tbody>
              <TotalsRow label="Príjem" current={current.income_cents} previous={previous.income_cents} />
              <TotalsRow label="Výdavky" current={current.expense_cents} previous={previous.expense_cents} />
              <TotalsRow label="Čisté" current={current.net_cents} previous={previous.net_cents} />
            </tbody>
          </table>
        </div>
      ) : null}
    </Card>
  )
}
