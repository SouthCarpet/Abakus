import { useEffect, useMemo, useState } from 'react'
import type { Account, AccountKind, BadChecksum, Status, Summary } from '../api'
import { api, formatEur } from '../api'
import { Button } from '../components/Button'
import { Card } from '../components/Card'
import { CategoryDonut } from '../components/charts/CategoryDonut'
import { IncomeExpense } from '../components/charts/IncomeExpense'
import { MonthlyStacked } from '../components/charts/MonthlyStacked'
import { Kpi } from '../components/Kpi'
import { PeriodPicker, usePeriod } from '../components/PeriodPicker'
import { monthRange, periodRange } from '../lib/period'

const ACCOUNT_KINDS: { id: 'all' | AccountKind; label: string }[] = [
  { id: 'all', label: 'Všetko' },
  { id: 'personal', label: 'Osobný' },
  { id: 'business', label: 'Firemný' },
]

export function ChecksumBanner({ row, onNavigate }: { row: BadChecksum; onNavigate: (statementId: number) => void }) {
  return (
    <Card
      title="Kontrolný súčet"
      footer={
        <Button variant="secondary" onClick={() => onNavigate(row.statement_id)}>
          Zobraziť transakcie
        </Button>
      }
    >
      <p className="k-text-danger">
        {`Výpis č. ${row.number} (účet ${row.account_label}) nesedí o ${formatEur(Math.abs(row.off_by_cents))}`}
      </p>
    </Card>
  )
}

function UnassignedTile({ count, suggested, onClick }: { count: number; suggested: number; onClick: () => void }) {
  return (
    <button type="button" className="k-kpi k-well k-section-body" onClick={onClick}>
      <span className="k-kpi-label">Nezaradené</span>
      <span className="k-kpi-value k-num">
        {count} (odhady {suggested})
      </span>
    </button>
  )
}

function TopMerchantsTable({ rows }: { rows: Summary['top_merchants'] }) {
  return (
    <Card title="Najčastejší obchodníci">
      <table className="k-table">
        <thead>
          <tr>
            <th>Obchodník</th>
            <th className="k-num">Suma</th>
            <th className="k-num">Počet</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.merchant}>
              <td>{r.merchant}</td>
              <td className="k-num">{formatEur(r.cents)}</td>
              <td className="k-num">{r.count}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </Card>
  )
}

function monthlySpan(rows: Summary['by_month'], pick: (m: Summary['by_month'][number]) => number): { min: number; max: number } | undefined {
  if (rows.length === 0) return undefined
  const values = rows.map(pick)
  return { min: Math.min(0, ...values), max: Math.max(...values) }
}

function EmptyOverview({ onNavigateToImport }: { onNavigateToImport: () => void }) {
  return (
    <Card
      title="Prehľad"
      footer={
        <Button variant="primary" onClick={onNavigateToImport}>
          Importovať
        </Button>
      }
    >
      Zatiaľ nič. Importuj prvý výpis.
    </Card>
  )
}

function SummaryBody({
  summary,
  incomeSpan,
  expenseSpan,
  onNavigateToTransactions,
  onMonthClick,
}: {
  summary: Summary
  incomeSpan?: { min: number; max: number }
  expenseSpan?: { min: number; max: number }
  onNavigateToTransactions: (entry: { statementId?: number; status?: Status; categoryId?: number }) => void
  onMonthClick: (month: string) => void
}) {
  const onCategoryClick = (categoryId: number) => onNavigateToTransactions({ categoryId })
  return (
    <>
      <div className="k-kpi-row">
        <Kpi label="Príjem" cents={summary.income_cents} min={incomeSpan?.min} max={incomeSpan?.max} tone="success" />
        <Kpi label="Výdavky" cents={summary.expense_cents} min={expenseSpan?.min} max={expenseSpan?.max} tone="danger" />
        <Kpi label="Čisté" cents={summary.net_cents} tone={summary.net_cents >= 0 ? 'success' : 'danger'} baselineOnly />
        <Kpi label="Prevody vylúčené" cents={summary.transfer_cents} />
      </div>
      <UnassignedTile
        count={summary.unassigned_count}
        suggested={summary.suggested_count}
        onClick={() => onNavigateToTransactions({ status: 'unassigned' })}
      />
      <div className="k-row" style={{ alignItems: 'stretch' }}>
        <div style={{ flex: 1, minWidth: 320 }}>
          <Card title="Podľa kategórií a mesiacov">
            <MonthlyStacked rows={summary.by_month_category} onCategoryClick={onCategoryClick} />
          </Card>
        </div>
        <div style={{ flex: 1, minWidth: 320 }}>
          <Card title="Príjmy a výdavky">
            <IncomeExpense rows={summary.by_month} onMonthClick={onMonthClick} />
          </Card>
        </div>
      </div>
      <Card title="Podľa kategórií">
        <CategoryDonut rows={summary.by_month_category} onCategoryClick={onCategoryClick} />
      </Card>
      <TopMerchantsTable rows={summary.top_merchants} />
    </>
  )
}

export function Overview({
  onNavigateToImport,
  onNavigateToTransactions,
}: {
  onNavigateToImport: () => void
  onNavigateToTransactions: (entry: { statementId?: number; status?: Status; categoryId?: number }) => void
}) {
  const [period, setPeriod] = usePeriod()
  const onMonthClick = (month: string) => setPeriod({ kind: 'custom', custom: monthRange(month) })
  const [accountKind, setAccountKind] = useState<'all' | AccountKind>('all')
  const [accounts, setAccounts] = useState<Account[]>([])
  const [summary, setSummary] = useState<Summary | null>(null)
  const [badChecksums, setBadChecksums] = useState<BadChecksum[]>([])

  useEffect(() => {
    void api.listAccounts().then(setAccounts)
    void api.badChecksums().then(setBadChecksums)
  }, [])

  const accountId = useMemo(() => {
    if (accountKind === 'all') return null
    return accounts.find((a) => a.kind === accountKind)?.id ?? null
  }, [accountKind, accounts])

  useEffect(() => {
    const { from, to } = periodRange(period.kind, new Date(), period.custom)
    void api.summary(from, to, accountId).then(setSummary)
  }, [period, accountId])

  const incomeSpan = summary ? monthlySpan(summary.by_month, (m) => m.income_cents) : undefined
  const expenseSpan = summary ? monthlySpan(summary.by_month, (m) => m.expense_cents) : undefined

  return (
    <div className="k-section">
      <div className="k-row">
        <PeriodPicker value={period} onChange={setPeriod} />
        <div className="k-row" role="group" aria-label="Účet">
          {ACCOUNT_KINDS.map((option) => (
            <Button key={option.id} variant={accountKind === option.id ? 'primary' : 'secondary'} onClick={() => setAccountKind(option.id)}>
              {option.label}
            </Button>
          ))}
        </div>
      </div>

      {badChecksums.map((row) => (
        <ChecksumBanner key={row.statement_id} row={row} onNavigate={(statementId) => onNavigateToTransactions({ statementId })} />
      ))}

      {summary !== null ? (
        summary.by_month.length === 0 ? (
          <EmptyOverview onNavigateToImport={onNavigateToImport} />
        ) : (
          <SummaryBody
            summary={summary}
            incomeSpan={incomeSpan}
            expenseSpan={expenseSpan}
            onNavigateToTransactions={onNavigateToTransactions}
            onMonthClick={onMonthClick}
          />
        )
      ) : null}
    </div>
  )
}
