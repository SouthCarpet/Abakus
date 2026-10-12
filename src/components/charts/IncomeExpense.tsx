import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import type { BarRectangleItem } from 'recharts'
import type { Summary } from '../../api'
import { formatAxisEur } from '../../lib/charts'
import { ChartLegend } from './ChartLegend'
import type { SeriesStep } from './ChartLegend'
import { ChartTooltip } from './ChartTooltip'

// Series identity is the step: it picks the lightness of the fill and the dash pattern of the
// outline (dataviz.md). Steps 1, 3 and 5 are spread apart so no two bars read as one green.
// Point 13: fee_cents is already a subset of expense_cents on the backend; the Poplatky series
// only surfaces it, it does not add a second total.
export const INCOME_EXPENSE_SERIES = [
  { key: 'income_cents', name: 'Príjem', step: 1 },
  { key: 'expense_cents', name: 'Výdavky', step: 3 },
  { key: 'fee_cents', name: 'Poplatky', step: 5 },
] as const satisfies readonly (SeriesStep & { key: keyof Summary['by_month'][number] })[]

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
    <div className="k-well ab-chart-well">
      <ResponsiveContainer width="100%" height={220}>
        <BarChart data={rows}>
          <CartesianGrid stroke="var(--color-chart-grid)" vertical={false} />
          <XAxis dataKey="month" stroke="var(--color-text-muted)" fontSize={12} tickLine={false} />
          <YAxis width={72} stroke="var(--color-text-muted)" fontSize={12} tickLine={false} domain={[0, 'auto']} tickFormatter={formatAxisEur} />
          <Tooltip content={ChartTooltip} cursor={{ className: 'ab-chart-cursor' }} />
          {INCOME_EXPENSE_SERIES.map(({ key, name, step }) => (
            <Bar
              key={key}
              dataKey={key}
              name={name}
              fill={`var(--color-chart-${step})`}
              stroke="var(--color-text)"
              strokeWidth={1.5}
              strokeDasharray={`var(--chart-dash-${step})`}
              className="ab-chart-slice"
              onClick={handleBarClick}
            />
          ))}
        </BarChart>
      </ResponsiveContainer>
      <ChartLegend series={INCOME_EXPENSE_SERIES} />
    </div>
  )
}
