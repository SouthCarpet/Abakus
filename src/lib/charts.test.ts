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
})
