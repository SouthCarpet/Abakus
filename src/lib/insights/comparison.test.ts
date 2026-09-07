import { describe, expect, it } from 'vitest'
import type { Summary } from '../../api'
import { compareCategories, isIncompletePeriod, percentDelta, previousEqualRange } from './comparison'

describe('previousEqualRange', () => {
  it('gives the immediately preceding 31 days for a 31-day August window', () => {
    expect(previousEqualRange({ from: '2026-08-01', to: '2026-08-31' })).toEqual({ from: '2026-07-01', to: '2026-07-31' })
  })
  it('gives an equal-day-count range, not a full previous month, for a 28-day February window', () => {
    expect(previousEqualRange({ from: '2026-02-01', to: '2026-02-28' })).toEqual({ from: '2026-01-04', to: '2026-01-31' })
  })
  it('handles a leap-year 29-day February window', () => {
    expect(previousEqualRange({ from: '2028-02-01', to: '2028-02-29' })).toEqual({ from: '2028-01-03', to: '2028-01-31' })
  })
  it('handles a single-day window', () => {
    expect(previousEqualRange({ from: '2026-06-15', to: '2026-06-15' })).toEqual({ from: '2026-06-14', to: '2026-06-14' })
  })
  it('crosses a year boundary', () => {
    expect(previousEqualRange({ from: '2026-01-01', to: '2026-01-10' })).toEqual({ from: '2025-12-22', to: '2025-12-31' })
  })
})

describe('isIncompletePeriod', () => {
  it('is incomplete when the range end is today (not yet fully elapsed)', () => {
    expect(isIncompletePeriod({ from: '2026-09-01', to: '2026-09-07' }, '2026-09-07')).toBe(true)
  })
  it('is incomplete when the range end is in the future', () => {
    expect(isIncompletePeriod({ from: '2026-09-01', to: '2026-09-30' }, '2026-09-07')).toBe(true)
  })
  it('is complete when the range end is fully in the past', () => {
    expect(isIncompletePeriod({ from: '2026-08-01', to: '2026-08-31' }, '2026-09-07')).toBe(false)
  })
})

describe('percentDelta', () => {
  it('computes percent change against the previous value', () => {
    expect(percentDelta(150, 100)).toBe(50)
  })
  it('divides by the absolute value of a negative previous total, so a refund-heavy category still reads as a positive-direction percent', () => {
    expect(percentDelta(-50, -100)).toBe(50)
  })
  it('returns null, never 0% or Infinity, when the previous value is zero', () => {
    expect(percentDelta(100, 0)).toBeNull()
  })
  it('returns 0 when current equals previous', () => {
    expect(percentDelta(100, 100)).toBe(0)
  })
})

const range = { from: '2026-08-01', to: '2026-08-31' }
const category = (overrides: Partial<Summary['by_category'][number]> = {}): Summary['by_category'][number] => ({
  category_id: 1,
  name: 'Jedlo',
  parent_name: null,
  cents: -1000,
  ...overrides,
})

describe('compareCategories', () => {
  it('unions a category present only in the previous range, with a zero current total', () => {
    const current = { by_category: [], expense_cents: 0 }
    const previous = { by_category: [category({ category_id: 7, name: 'Doprava', cents: -500 })], expense_cents: -500 }
    const result = compareCategories(range, '2026-09-07', current, previous)
    expect(result.rows.find((r) => r.categoryId === 7)).toEqual({
      categoryId: 7, name: 'Doprava', parentName: null, currentCents: 0, previousCents: -500, deltaCents: 500, percent: 100,
    })
  })
  it('unions a category present only in the current range, with an unavailable previous percent', () => {
    const current = { by_category: [category({ category_id: 9, name: 'Zdravie', cents: -300 })], expense_cents: -300 }
    const previous = { by_category: [], expense_cents: 0 }
    const result = compareCategories(range, '2026-09-07', current, previous)
    expect(result.rows.find((r) => r.categoryId === 9)).toEqual({
      categoryId: 9, name: 'Zdravie', parentName: null, currentCents: -300, previousCents: 0, deltaCents: -300, percent: null,
    })
  })
  it('keeps a sign flip between a refund-net previous period and an expense-net current period', () => {
    const current = { by_category: [category({ category_id: 1, cents: 200 })], expense_cents: 200 }
    const previous = { by_category: [category({ category_id: 1, cents: -1000 })], expense_cents: -1000 }
    const result = compareCategories(range, '2026-09-07', current, previous)
    const row = result.rows.find((r) => r.categoryId === 1)
    expect(row?.currentCents).toBe(200)
    expect(row?.previousCents).toBe(-1000)
    expect(row?.deltaCents).toBe(1200)
  })
  it('adds a residual uncategorized row reconciling to expense_cents minus the categorized sum', () => {
    const current = { by_category: [category({ category_id: 1, cents: -700 })], expense_cents: -1000 }
    const previous = { by_category: [], expense_cents: 0 }
    const result = compareCategories(range, '2026-09-07', current, previous)
    const residual = result.rows.find((r) => r.categoryId === null)
    expect(residual?.currentCents).toBe(-300)
    expect(residual?.name).toBe('Nezaradené výdavky')
  })
  it('gives a zero residual when the categorized sum already reconciles to expense_cents', () => {
    const current = { by_category: [category({ category_id: 1, cents: -1000 })], expense_cents: -1000 }
    const previous = { by_category: [], expense_cents: 0 }
    const result = compareCategories(range, '2026-09-07', current, previous)
    expect(result.rows.find((r) => r.categoryId === null)?.currentCents).toBe(0)
  })
  it('carries the equal-day-count previous range alongside the current one', () => {
    const current = { by_category: [], expense_cents: 0 }
    const previous = { by_category: [], expense_cents: 0 }
    const result = compareCategories(range, '2026-09-07', current, previous)
    expect(result.previousRange).toEqual({ from: '2026-07-01', to: '2026-07-31' })
  })
  it('marks a current range ending in the future as incomplete', () => {
    const current = { by_category: [], expense_cents: 0 }
    const previous = { by_category: [], expense_cents: 0 }
    const result = compareCategories({ from: '2026-09-01', to: '2026-09-30' }, '2026-09-07', current, previous)
    expect(result.incomplete).toBe(true)
  })
  it('marks a fully-elapsed current range as complete', () => {
    const current = { by_category: [], expense_cents: 0 }
    const previous = { by_category: [], expense_cents: 0 }
    const result = compareCategories(range, '2026-09-07', current, previous)
    expect(result.incomplete).toBe(false)
  })
})
