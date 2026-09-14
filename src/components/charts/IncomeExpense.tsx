import { Bar, BarChart, CartesianGrid, Legend, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import type { BarRectangleItem } from 'recharts'
import type { Summary } from '../../api'
import { formatEur } from '../../api'
import { ChartTooltip } from './ChartTooltip'

export function IncomeExpense({
  rows,
  onMonthClick,
}: {
  rows: Summary['by_month']
  onMonthClick: (month: string) => void
}) {
  function handleBarClick(data: BarRectangleItem) {
    const payload = data.payload as Summary['by_month'][number]
    onMonthClick(payload.month)
  }
  return (
    <div className="k-well" style={{ padding: 'var(--space-3)' }}>
      <ResponsiveContainer width="100%" height={220}>
        <BarChart data={rows}>
          <CartesianGrid stroke="var(--color-chart-grid)" vertical={false} />
          <XAxis dataKey="month" stroke="var(--color-text-muted)" fontSize={12} tickLine={false} />
          <YAxis stroke="var(--color-text-muted)" fontSize={12} tickLine={false} domain={[0, 'auto']} tickFormatter={(v: number) => formatEur(v)} />
          <Tooltip content={ChartTooltip} cursor={{ className: 'k-chart-cursor' }} />
          <Legend />
          <Bar dataKey="income_cents" name="Príjem" fill="var(--color-chart-1)" className="k-chart-slice" onClick={handleBarClick} />
          <Bar dataKey="expense_cents" name="Výdavky" fill="var(--color-chart-2)" className="k-chart-slice" onClick={handleBarClick} />
          {/* Point 13: fee_cents is already a subset of expense_cents on the
              backend; this series only surfaces it, it does not add a second
              total. */}
          <Bar dataKey="fee_cents" name="Poplatky" fill="var(--color-chart-3)" className="k-chart-slice" onClick={handleBarClick} />
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}
