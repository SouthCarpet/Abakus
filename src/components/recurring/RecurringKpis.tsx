import { formatEur } from '../../lib/format'
import {
  cashDirectionCaption,
  exclusionLabel,
  expenseShareLabel,
  remainingMonthLabel,
} from '../../lib/recurring-labels'
import type { RecurringAmounts, RecurringOverview } from '../../lib/recurring-api'
import { Button } from '../Button'

export function RecurringKpis({
  overview,
  today,
  annual,
  onAnnualChange,
}: {
  overview: RecurringOverview
  today: string
  annual: boolean
  onAnnualChange: (annual: boolean) => void
}) {
  const confirmed = overview.confirmed
  const estimates = overview.estimates
  const expense = annual ? confirmed.annual_expense_cents : confirmed.monthly_expense_cents
  const income = annual ? confirmed.annual_income_cents : confirmed.monthly_income_cents
  const net = annual ? confirmed.annual_net_cents : confirmed.monthly_net_cents
  const estimateExpense = annual ? estimates.annual_expense_cents : estimates.monthly_expense_cents
  const estimateIncome = annual ? estimates.annual_income_cents : estimates.monthly_income_cents
  const remainingLabel = remainingMonthLabel(overview.as_of, today)
  return (
    <>
      <div className="k-row" role="group" aria-label="Projekcia">
        <Button variant={annual ? 'secondary' : 'primary'} aria-pressed={!annual} onClick={() => onAnnualChange(false)}>
          Za mesiac
        </Button>
        <Button variant={annual ? 'primary' : 'secondary'} aria-pressed={annual} onClick={() => onAnnualChange(true)}>
          Za rok
        </Button>
      </div>
      <div className="k-kpi-row">
        <AmountTile
          label={annual ? 'Pravidelné výdavky za rok' : 'Pravidelné výdavky za mesiac'}
          cents={expense}
          tone="danger"
          extra={`Ďalšie odhady ${formatEur(estimateExpense)}`}
        />
        <AmountTile
          label={annual ? 'Pravidelné príjmy za rok' : 'Pravidelné príjmy za mesiac'}
          cents={income}
          tone="success"
          extra={`Ďalšie odhady ${formatEur(estimateIncome)}`}
        />
        <RemainingTile
          label={remainingLabel}
          confirmed={confirmed}
          estimates={estimates}
        />
        <AmountTile
          label="Voľné po pravidelných platbách"
          cents={net}
          tone={net >= 0 ? 'success' : 'danger'}
        />
      </div>
      <p>{exclusionLabel(overview)}</p>
      <p>{expenseShareLabel(overview)}</p>
      <p>{cashDirectionCaption()}</p>
    </>
  )
}

function AmountTile({
  label,
  cents,
  tone,
  extra,
}: {
  label: string
  cents: number
  tone: 'danger' | 'success'
  extra?: string
}) {
  const toneClass = tone === 'danger' ? ' k-text-danger' : ' k-text-success'
  return (
    <div className="k-kpi">
      <span className="k-kpi-label">{label}</span>
      <span className={`k-kpi-value k-num${toneClass}`}>{formatEur(cents)}</span>
      {extra ? <span className="k-field-label">{extra}</span> : null}
    </div>
  )
}

function RemainingTile({
  label,
  confirmed,
  estimates,
}: {
  label: string
  confirmed: RecurringAmounts
  estimates: RecurringAmounts
}) {
  return (
    <div className="k-kpi">
      <span className="k-kpi-label">{label}</span>
      <span className="k-kpi-value k-num k-text-danger">{formatEur(confirmed.remaining_expense_cents)}</span>
      <span className="k-field-label">Potvrdené výdavky {formatEur(confirmed.remaining_expense_cents)}</span>
      <span className="k-field-label">Potvrdené príjmy {formatEur(confirmed.remaining_income_cents)}</span>
      <span className="k-field-label">Ďalšie odhady výdavkov {formatEur(estimates.remaining_expense_cents)}</span>
      <span className="k-field-label">Ďalšie odhady príjmov {formatEur(estimates.remaining_income_cents)}</span>
    </div>
  )
}
