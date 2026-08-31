import { Cell, Pie, PieChart, ResponsiveContainer, Tooltip } from 'recharts'
import type { PieSectorDataItem } from 'recharts'
import type { Summary } from '../../api'
import { formatEur } from '../../api'
import { OSTATNE_ID, sliceCategoryId, topCategories, type CategoryTotal } from '../../lib/charts'
import { ChartTooltip } from './ChartTooltip'

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

export function CategoryDonut({
  rows,
  onCategoryClick,
}: {
  rows: Summary['by_month_category']
  onCategoryClick: (categoryId: number) => void
}) {
  const categories = topCategories(rows, 6)

  function handleSliceClick(entry: PieSectorDataItem) {
    const payload = entry.payload as CategoryTotal
    const categoryId = sliceCategoryId(payload.category_id)
    if (categoryId !== null) onCategoryClick(categoryId)
  }

  return (
    <div className="k-well" style={{ padding: 'var(--space-3)' }}>
      <ResponsiveContainer width="100%" height={220}>
        <PieChart>
          <Pie data={categories} dataKey="cents" nameKey="name" innerRadius="60%" outerRadius="85%" onClick={handleSliceClick}>
            {categories.map((cat, i) => (
              <Cell
                key={cat.category_id}
                fill={sliceColor(cat.category_id, i)}
                className={cat.category_id === OSTATNE_ID ? 'k-chart-ostatne' : 'k-chart-slice'}
              />
            ))}
          </Pie>
          <Tooltip content={ChartTooltip} cursor={{ className: 'k-chart-cursor' }} />
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
