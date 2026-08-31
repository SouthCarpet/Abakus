import { Cell, Pie, PieChart, ResponsiveContainer, Tooltip } from 'recharts'
import type { Summary } from '../../api'
import { formatEur } from '../../api'
import { OSTATNE_ID, topCategories } from '../../lib/charts'

// Same never-cycle guarantee as MonthlyStacked: at most 6 real slices, the
// folded Ostatné slice always draws with --color-border, never a 7th hue.
const CHART_COLORS = [
  'var(--color-chart-1)',
  'var(--color-chart-2)',
  'var(--color-chart-3)',
  'var(--color-chart-4)',
  'var(--color-chart-5)',
  'var(--color-chart-6)',
]

function sliceColor(categoryId: number, index: number): string {
  return categoryId === OSTATNE_ID ? 'var(--color-border)' : CHART_COLORS[index]
}

export function CategoryDonut({ rows }: { rows: Summary['by_category'] }) {
  const categories = topCategories(rows, 6)
  return (
    <div className="k-well" style={{ padding: 'var(--space-3)' }}>
      <ResponsiveContainer width="100%" height={220}>
        <PieChart>
          <Pie data={categories} dataKey="cents" nameKey="name" innerRadius="60%" outerRadius="85%">
            {categories.map((cat, i) => (
              <Cell
                key={cat.category_id}
                fill={sliceColor(cat.category_id, i)}
                fillOpacity={cat.category_id === OSTATNE_ID ? 'var(--color-chart-band-alpha)' : undefined}
              />
            ))}
          </Pie>
          <Tooltip formatter={(value) => formatEur(Number(value))} />
        </PieChart>
      </ResponsiveContainer>
      <div className="k-legend">
        {categories.map((cat, i) => (
          <span key={cat.category_id} className="k-legend-item">
            <span className="k-legend-swatch" style={{ background: sliceColor(cat.category_id, i) }} />
            {cat.name} · {formatEur(cat.cents)}
          </span>
        ))}
      </div>
    </div>
  )
}
