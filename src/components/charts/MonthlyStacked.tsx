import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import type { Summary } from '../../api'
import { OSTATNE_ID, formatAxisEur, topCategories } from '../../lib/charts'
import { ChartLegend } from './ChartLegend'
import type { SeriesStep } from './ChartLegend'
import { ChartTooltip } from './ChartTooltip'

// Series 1..6 take ramp steps 1..6 in order, never cycling: topCategories caps the real series at 6.
// Each step picks the fill lightness and the outline dash (dataviz.md), as in IncomeExpense. The
// folded Ostatné series is drawn muted, never a seventh step.
export function stackedSeries(categories: { name: string; category_id: number }[]): SeriesStep[] {
  return categories.map((cat, i) =>
    cat.category_id === OSTATNE_ID ? { name: cat.name, muted: true } : { name: cat.name, step: (i + 1) as 1 | 2 | 3 | 4 | 5 | 6 },
  )
}

function pivotByMonth(rows: Summary['by_month_category'], categoryIds: Set<number>) {
  const months = [...new Set(rows.map((r) => r.month))].sort()
  return months.map((month) => {
    const point: Record<string, number | string> = { month }
    for (const row of rows) {
      if (row.month !== month) continue
      const key = String(categoryIds.has(row.category_id) ? row.category_id : OSTATNE_ID)
      point[key] = (Number(point[key]) || 0) + row.cents
    }
    return point
  })
}

export function MonthlyStacked({
  rows,
  onCategoryClick,
}: {
  rows: Summary['by_month_category']
  onCategoryClick: (categoryId: number) => void
}) {
  if (rows.length === 0) return <p>Za vybrané obdobie nie sú údaje podľa kategórií a mesiacov.</p>
  const categories = topCategories(rows, 6)
  const topIds = new Set(categories.filter((c) => c.category_id !== OSTATNE_ID).map((c) => c.category_id))
  const data = pivotByMonth(rows, topIds)
  return (
    <div className="ab-chart-well">
      <ResponsiveContainer width="100%" height={220}>
        <BarChart data={data}>
          <CartesianGrid stroke="var(--color-chart-grid)" vertical={false} />
          <XAxis dataKey="month" stroke="var(--color-text-muted)" fontSize={12} tickLine={false} />
          <YAxis width={72} stroke="var(--color-text-muted)" fontSize={12} tickLine={false} domain={[0, 'auto']} tickFormatter={formatAxisEur} />
          <Tooltip content={ChartTooltip} cursor={{ className: 'ab-chart-cursor' }} />
                    {categories.map((cat, i) =>
            cat.category_id === OSTATNE_ID ? (
              <Bar key={cat.category_id} dataKey={String(cat.category_id)} name={cat.name} stackId="a" fill="var(--color-border)" stroke="var(--color-text)" strokeWidth={1.5} className="ab-chart-ostatne" />
            ) : (
              <Bar
                key={cat.category_id}
                dataKey={String(cat.category_id)}
                name={cat.name}
                stackId="a"
                fill={`var(--color-chart-${i + 1})`}
                stroke="var(--color-text)"
                strokeWidth={1.5}
                strokeDasharray={`var(--chart-dash-${i + 1})`}
                className="ab-chart-slice"
                onClick={() => onCategoryClick(cat.category_id)}
              />
            ),
          )}
        </BarChart>
      </ResponsiveContainer>
      <ChartLegend series={stackedSeries(categories)} />
    </div>
  )
}
