import { useEffect, useState } from 'react'
import type { Account, AccountKind, BadChecksum, StatementHistoryRow, Status, Summary } from '../api'
import { api, formatEur } from '../api'
import { Button } from '../components/Button'
import { Card } from '../components/Card'
import { CategoryBars } from '../components/charts/CategoryBars'
import { IncomeExpense } from '../components/charts/IncomeExpense'
import { MonthlyStacked } from '../components/charts/MonthlyStacked'
import { InsightsPanel } from '../components/insights/InsightsPanel'
import { StatementReminderBanner } from '../components/insights/StatementReminderBanner'
import { YearComparisonCard } from '../components/insights/YearComparisonCard'
import { PageHead } from '../components/PageHead'
import { ExportPdfAction } from '../components/report/ExportPdfAction'
import { PeriodPicker, usePeriod } from '../components/PeriodPicker'
import { monthRange, periodRange, validPeriod } from '../lib/period'
import { countPhrase, pluralSk, tx } from '../lib/plural'
import { localTodayIso } from '../lib/recurring-labels'
import { RecurringPanel } from '../components/recurring'

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
      <p className="ab-text-danger">
        {`Výpis č. ${row.number} (účet ${row.account_label}) nesedí o ${formatEur(Math.abs(row.off_by_cents))}`}
      </p>
    </Card>
  )
}

// The one ink row near the top (R34 d): it exists only while a real task waits, carries the
// count and one action that opens the unassigned payments in Transakcie.
function WaitingRow({ count, suggested, onClick }: { count: number; suggested: number; onClick: () => void }) {
  if (count === 0) return null
  const waits = pluralSk(count, ['čaká', 'čakajú', 'čaká'])
  return (
    <div className="k-ink-row" role="status" aria-label="Čakajú na zaradenie">
      <div>
        <div className="k-ink-row-text">{countPhrase(count, tx.platba)} {waits} na zaradenie</div>
        {suggested > 0 ? (
          <div className="k-ink-row-note">{pluralSk(suggested, ['Čaká', 'Čakajú', 'Čaká'])} {countPhrase(suggested, tx.odhad)} na potvrdenie.</div>
        ) : null}
      </div>
      <button type="button" className="k-ink-action" onClick={onClick}>Zaradiť platby</button>
    </div>
  )
}

// R27: the totals of the period are a ledger on the sheet. Čisté is the lead row with the
// accent fill. Fees are already inside Výdavky and transfers stay outside, so both are
// named under the ledger and never added to it.
function PeriodLedger({ summary }: { summary: Summary }) {
  return (
    <section aria-label="Súhrn obdobia" className="ab-pad">
      <div className="k-ledger ab-ledger-end">
        <div className="k-ledger-row"><span>Príjem<span className="k-sr-only">:</span></span>{' '}<span className="k-ledger-amount">{formatEur(summary.income_cents)}</span></div>
        <div className="k-ledger-row"><span>Výdavky<span className="k-sr-only">:</span></span>{' '}<span className="k-ledger-amount">{formatEur(summary.expense_cents)}</span></div>
        <div className="k-ledger-total k-ledger-total--lead"><span>Čisté<span className="k-sr-only">:</span></span>{' '}<span className="k-ledger-amount">{formatEur(summary.net_cents)}</span></div>
      </div>
      <dl className="ab-ledger-memo">
        <div><dt>Poplatky</dt>{' '}<dd className="k-num">{formatEur(summary.fee_cents)}</dd>{' '}<dd className="ab-muted">už vo výdavkoch</dd></div>
        <div><dt>Prevody mimo súčtov</dt>{' '}<dd className="k-num">{formatEur(summary.transfer_cents)}</dd></div>
      </dl>
    </section>
  )
}

// Point 17: the merchant name is the drilldown affordance into Transactions,
// pre-filtered by that merchant's text (point 9's substring search).
function TopMerchantsTable({ rows, onMerchantClick }: { rows: Summary['top_merchants']; onMerchantClick: (merchant: string) => void }) {
  return (
    <Card title="Najčastejší obchodníci">
      <table className="k-table">
        <thead>
          <tr>
            <th>Obchodník</th>
            <th className="k-num k-cell-number">Suma</th>
            <th className="k-num k-cell-number">Počet</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.merchant}>
              <td>
                <Button variant="ghost" onClick={() => onMerchantClick(r.merchant)}>
                  {r.merchant}
                </Button>
              </td>
              <td className="k-num k-cell-number">{formatEur(r.cents)}</td>
              <td className="k-num k-cell-number">{r.count}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </Card>
  )
}

// A17: "Importuj prvý výpis" is wrong advice once imports already exist; the
// real cause is then the period filter, so the two cases get two sentences.
function EmptyOverview({ onNavigateToImport, hasAnyImports }: { onNavigateToImport: () => void; hasAnyImports: boolean }) {
  if (hasAnyImports) {
    return <Card title="Prehľad">Za vybrané obdobie nič nie je. Skús iné obdobie hore, napríklad Všetko.</Card>
  }
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
  onNavigateToTransactions,
  onMonthClick,
}: {
  summary: Summary
  onNavigateToTransactions: (entry: { statementId?: number; status?: Status; categoryId?: number; merchantText?: string }) => void
  onMonthClick: (month: string) => void
}) {
  const onCategoryClick = (categoryId: number) => onNavigateToTransactions({ categoryId })
  const onMerchantClick = (merchant: string) => onNavigateToTransactions({ merchantText: merchant })
  return (
    <>
      <WaitingRow
        count={summary.unassigned_count}
        suggested={summary.suggested_count}
        onClick={() => onNavigateToTransactions({ status: 'unassigned' })}
      />
      <PeriodLedger summary={summary} />
      <div className="ab-row" style={{ alignItems: 'stretch' }}>
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
        <CategoryBars rows={summary.by_month_category} onCategoryClick={onCategoryClick} />
      </Card>
      <TopMerchantsTable rows={summary.top_merchants} onMerchantClick={onMerchantClick} />
    </>
  )
}

export function Overview({
  onNavigateToImport,
  onNavigateToTransactions,
}: {
  onNavigateToImport: () => void
  onNavigateToTransactions: (entry: { statementId?: number; status?: Status; categoryId?: number; merchantText?: string; accountKind?: AccountKind }) => void
}) {
  const [period, setPeriod] = usePeriod()
  const [error, setError] = useState('')
  const [summaryError, setSummaryError] = useState('')
  const onMonthClick = (month: string) => setPeriod({ kind: 'custom', custom: monthRange(month) })
  const [accountKind, setAccountKind] = useState<'all' | AccountKind>('all')
  // A17/F3: filters by the whole KIND, so a second personal or business
  // account is never dropped out of the totals (it used to send the id of
  // just one account of that kind).
  const kind = accountKind === 'all' ? null : accountKind
  const [summary, setSummary] = useState<Summary | null>(null)
  const [badChecksums, setBadChecksums] = useState<BadChecksum[]>([])
  // A17: whether ANY statement was ever imported, independent of the period
  // filter, so the empty state can tell "no imports" from "wrong period".
  const [hasAnyImports, setHasAnyImports] = useState<boolean | null>(null)
  // Point 16/18: the statement-coverage reminder banner owns its own fetch
  // (same independence principle as the rest of the insights section), so a
  // failed or slow reminder never blocks or is blocked by the transaction
  // summary above it.
  const [reminderAccounts, setReminderAccounts] = useState<Account[]>([])
  const [reminderHistory, setReminderHistory] = useState<StatementHistoryRow[]>([])

  // A17: a drill-down out of a kind-filtered Prehľad must land on the same
  // filter in Transakcie, or it can show rows from an account the user just
  // filtered out.
  function navigateFiltered(entry: { statementId?: number; status?: Status; categoryId?: number; merchantText?: string }) {
    onNavigateToTransactions(accountKind === 'all' ? entry : { ...entry, accountKind })
  }

  useEffect(() => {
    void api.badChecksums().then(setBadChecksums).catch((e) => setError(String(e)))
  }, [])

  useEffect(() => {
    void api.recentStatements(1).then((rows) => setHasAnyImports(rows.length > 0)).catch((e) => setError(String(e)))
  }, [])

  useEffect(() => {
    void api.listAccounts().then(setReminderAccounts).catch((e) => setError(String(e)))
  }, [])

  useEffect(() => {
    void api.statementHistory(null, kind).then(setReminderHistory).catch((e) => setError(String(e)))
  }, [kind])

  useEffect(() => {
    let active = true
    setSummary(null); setSummaryError('')
    if (!validPeriod(period)) return
    const { from, to } = periodRange(period.kind, new Date(), period.custom)
    void api.summary(from, to, null, kind).then((value) => { if (active) setSummary(value) }).catch((e) => { if (active) setSummaryError(String(e)) })
    return () => { active = false }
  }, [period, kind])

  return (
    <div className="ab-screen">
      <PageHead title="Prehľad" meta="Súčty za vybrané obdobie. Prevody medzi vlastnými účtami sú mimo súčtov." primary={<ExportPdfAction variant="primary" />} />
      <div className="ab-row">
        <PeriodPicker value={period} onChange={setPeriod} />
        <div className="ab-period">
          <span id="ab-account-kind" className="ab-field-name">Účet</span>
          <div className="ab-row ab-period-buttons" role="group" aria-labelledby="ab-account-kind">
            {ACCOUNT_KINDS.map((option) => (
              <Button
                aria-pressed={accountKind === option.id}
                key={option.id}
                variant="secondary"
                className={accountKind === option.id ? 'ab-pressed' : undefined}
                onClick={() => setAccountKind(option.id)}
              >
                {option.label}
              </Button>
            ))}
          </div>
        </div>
      </div>

      {error ? <p role="alert">{error}</p> : null}
      {summaryError ? <p role="alert">{summaryError}</p> : null}
      <StatementReminderBanner
        accounts={reminderAccounts}
        statements={reminderHistory}
        accountKind={kind}
        today={localTodayIso()}
        onNavigateToImport={onNavigateToImport}
      />
      {badChecksums.map((row) => (
        <ChecksumBanner key={row.statement_id} row={row} onNavigate={(statementId) => onNavigateToTransactions({ statementId })} />
      ))}

      {summary !== null && hasAnyImports !== null ? (
        summary.by_month.length === 0 ? (
          <EmptyOverview onNavigateToImport={onNavigateToImport} hasAnyImports={hasAnyImports} />
        ) : (
          <SummaryBody
            summary={summary}
            onNavigateToTransactions={navigateFiltered}
            onMonthClick={onMonthClick}
          />
        )
      ) : null}

      {/* Recurring schedules are a snapshot at period end, not a sum of the
          selected-period transactions, so the panel stays mounted even when
          that summary is empty. */}
      <RecurringPanel period={period} accountKind={accountKind} today={localTodayIso()} />

      {/* Coverage, balance history and category comparison own their own
          fetches and stay visible even when the transaction summary above is
          empty or still loading. */}
      <InsightsPanel period={period} accountKind={accountKind} summary={summary} />

      {/* Point 20: this comparison has its own month/period-window controls,
          independent of the PeriodPicker above, so it lives directly in
          Overview rather than inside InsightsPanel (whose category
          comparison is gated on that PeriodPicker's period instead). */}
      <YearComparisonCard accountKind={kind} today={localTodayIso()} />
    </div>
  )
}
