import { describe, expect, it } from 'vitest'
import { OSTATNE_ID, topCategories } from './charts'

const rows = [
  { category_id: 1, name: 'Jedlo', cents: 800 },
  { category_id: 2, name: 'Doprava', cents: 700 },
  { category_id: 3, name: 'Bývanie', cents: 600 },
  { category_id: 4, name: 'Zábava', cents: 500 },
  { category_id: 5, name: 'Zdravie', cents: 400 },
  { category_id: 6, name: 'Oblečenie', cents: 300 },
  { category_id: 7, name: 'Dary', cents: 200 },
  { category_id: 8, name: 'Iné', cents: 100 },
]

describe('topCategories', () => {
  it('keeps the 6 largest and folds the remaining 2 into one Ostatné series', () => {
    const result = topCategories(rows, 6)
    expect(result).toHaveLength(7)
    expect(result.slice(0, 6).map((c) => c.category_id)).toEqual([1, 2, 3, 4, 5, 6])
    expect(result[6]).toEqual({ category_id: OSTATNE_ID, name: 'Ostatné', cents: 300 })
  })

  it('sums duplicate category ids across rows before ranking', () => {
    const split = [
      { category_id: 1, name: 'Jedlo', cents: 100 },
      { category_id: 1, name: 'Jedlo', cents: 50 },
      { category_id: 2, name: 'Doprava', cents: 90 },
    ]
    expect(topCategories(split, 6)).toEqual([
      { category_id: 1, name: 'Jedlo', cents: 150 },
      { category_id: 2, name: 'Doprava', cents: 90 },
    ])
  })

  it('does not add an Ostatné row when everything fits under the limit', () => {
    const few = [{ category_id: 1, name: 'Jedlo', cents: 100 }]
    expect(topCategories(few, 6)).toEqual(few)
  })

  it('rolls up a multi-month input to the same per-category totals the stacked chart draws', () => {
    // Shape of Summary.by_month_category: one row per (month, category).
    // MonthlyStacked draws one stacked bar per month, one segment per
    // category; summing a category's segments across every bar gives its
    // total height across the whole chart. CategoryDonut feeds the same
    // rows through topCategories, which sums by category_id regardless of
    // month, so the two must land on identical per-category totals.
    const monthly = [
      { month: '2026-01', category_id: 1, name: 'Jedlo', cents: 300 },
      { month: '2026-02', category_id: 1, name: 'Jedlo', cents: 500 },
      { month: '2026-01', category_id: 2, name: 'Doprava', cents: 200 },
      { month: '2026-03', category_id: 2, name: 'Doprava', cents: 100 },
    ]
    const stackedTotalsByCategory = new Map<number, number>()
    for (const row of monthly) {
      stackedTotalsByCategory.set(row.category_id, (stackedTotalsByCategory.get(row.category_id) ?? 0) + row.cents)
    }

    const donut = topCategories(monthly, 6)

    expect(donut.map((c) => c.cents)).toEqual(donut.map((c) => stackedTotalsByCategory.get(c.category_id)))
    expect(new Map(donut.map((c) => [c.category_id, c.cents]))).toEqual(
      new Map([
        [1, 800],
        [2, 300],
      ]),
    )
  })
})
