import type { Summary } from '../../api'
import { formatEur } from '../../api'
import { topCategories } from '../../lib/charts'

export function CategoryBars({ rows, onCategoryClick }: {
  rows: Summary['by_month_category']
  onCategoryClick: (categoryId: number) => void
}) {
  // Keep every category: folding signed totals can hide offsetting refunds.
  const categories = topCategories(rows, rows.length)
  const extent = Math.max(1, ...categories.map((category) => Math.abs(category.cents)))
  if (categories.length === 0) return <p>Za vybrané obdobie nie sú údaje podľa kategórií.</p>

  return <div className="k-well k-category-bars">
    <p>Záporné hodnoty vľavo znižujú výdavky. Kladné hodnoty vpravo ich zvyšujú. Stred je 0 €.</p>
    {categories.every((category) => category.cents === 0) ? <p>Súčty všetkých kategórií sú nulové.</p> : null}
    {categories.map((category, index) => {
      // Absolute magnitude is used only for geometry; labels and totals stay signed cents.
      const width = Math.abs(category.cents) / extent * 50
      const start = category.cents < 0 ? 50 - width : 50
      const color = index < 6 ? `var(--color-chart-${index + 1})` : 'var(--color-border)'
      const label = `${category.name} · ${formatEur(category.cents)}`
      return <button key={category.category_id} type="button" className="k-category-bar" onClick={() => onCategoryClick(category.category_id)} aria-label={label}>
        <span className="k-category-bar-label"><span>{category.name}</span><span className="k-num k-money">{formatEur(category.cents)}</span></span>
        <svg viewBox="0 0 100 12" preserveAspectRatio="none" role="img" aria-label={label}>
          <rect x={start} y="3" width={width} height="6" fill={color} />
          <line x1="50" x2="50" y1="0" y2="12" stroke="var(--color-text-muted)" vectorEffect="non-scaling-stroke" />
        </svg>
      </button>
    })}
  </div>
}
