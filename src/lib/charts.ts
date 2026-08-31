export interface CategoryTotal {
  category_id: number
  name: string
  cents: number
}

// Sentinel id for the folded "Ostatné" (everything else) series. Never a
// real category id (those come from SQLite autoincrement, always positive).
export const OSTATNE_ID = -1
const OSTATNE_NAME = 'Ostatné'

/**
 * Keeps the `limit` largest categories by total cents (rows for the same
 * category_id are summed first, so a monthly breakdown collapses correctly)
 * and folds everything else into one `Ostatné` row. A chart series never
 * cycles back through the chart-1..6 palette: this caps the real series at
 * `limit` and gives the remainder a single, separately-styled slot.
 */
// Donut/stacked-bar drilldown: a clicked slice's category_id maps to itself,
// except the folded Ostatné slice, which maps to null (no drilldown target).
export function sliceCategoryId(categoryId: number): number | null {
  return categoryId === OSTATNE_ID ? null : categoryId
}

export function topCategories(rows: CategoryTotal[], limit: number): CategoryTotal[] {
  const totals = new Map<number, CategoryTotal>()
  for (const row of rows) {
    const prev = totals.get(row.category_id)
    totals.set(row.category_id, { category_id: row.category_id, name: row.name, cents: (prev?.cents ?? 0) + row.cents })
  }
  const sorted = [...totals.values()].sort((a, b) => b.cents - a.cents)
  const top = sorted.slice(0, limit)
  const rest = sorted.slice(limit)
  if (rest.length > 0) {
    top.push({ category_id: OSTATNE_ID, name: OSTATNE_NAME, cents: rest.reduce((sum, r) => sum + r.cents, 0) })
  }
  return top
}
