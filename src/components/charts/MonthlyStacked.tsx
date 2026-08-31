import { Bar, BarChart, CartesianGrid, Legend, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import type { Summary } from '../../api'
import { formatEur } from '../../api'
import { OSTATNE_ID, topCategories } from '../../lib/charts'
import { ChartTooltip } from './ChartTooltip'

// Series 1..6 map to the six fixed chart-N tokens in order, never cycling
// back: topCategories caps the real series at 6, so this index never runs
// past the array. The folded Ostatné series (if any) is always drawn with
// --color-border, never a seventh chart colour.
const CHART_COLORS = [
  'var(--color-chart-1)',
  'var(--color-chart-2)',
  'var(--color-chart-3)',
  'var(--color-chart-4)',
  'var(--color-chart-5)',
  'var(--color-chart-6)',
]

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
  const categories = topCategories(rows, 6)
  const topIds = new Set(categories.filter((c) => c.category_id !== OSTATNE_ID).map((c) => c.category_id))
  const data = pivotByMonth(rows, topIds)
  return (
    <div className="k-well" style={{ padding: 'var(--space-3)' }}>
      <ResponsiveContainer width="100%" height={220}>
        <BarChart data={data}>
          <CartesianGrid stroke="var(--color-chart-grid)" vertical={false} />
          <XAxis dataKey="month" stroke="var(--color-text-muted)" fontSize={12} tickLine={false} />
          <YAxis stroke="var(--color-text-muted)" fontSize={12} tickLine={false} domain={[0, 'auto']} tickFormatter={(v: number) => formatEur(v)} />
          <Tooltip content={ChartTooltip} cursor={{ className: 'k-chart-cursor' }} />
          <Legend />
          {categories.map((cat, i) =>
            cat.category_id === OSTATNE_ID ? (
              <Bar key={cat.category_id} dataKey={String(cat.category_id)} name={cat.name} stackId="a" fill="var(--color-border)" className="k-chart-ostatne" />
            ) : (
              <Bar
                key={cat.category_id}
                dataKey={String(cat.category_id)}
                name={cat.name}
                stackId="a"
                fill={CHART_COLORS[i]}
                className="k-chart-slice"
                onClick={() => onCategoryClick(cat.category_id)}
              />
            ),
          )}
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}
