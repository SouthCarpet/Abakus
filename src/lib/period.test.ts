import { describe, expect, it } from 'vitest'
import { monthRange, periodRange } from './period'
const today = new Date(2026, 7, 30) // 30. 8. 2026
describe('periodRange', () => {
  it('this month', () => { expect(periodRange('this_month', today)).toEqual({ from: '2026-08-01', to: '2026-08-31' }) })
  it('last month', () => { expect(periodRange('last_month', today)).toEqual({ from: '2026-07-01', to: '2026-07-31' }) })
  it('3 months back from the first of the month, to today', () => { expect(periodRange('m3', today)).toEqual({ from: '2026-06-01', to: '2026-08-30' }) })
  it('6 months and 1 year', () => { expect(periodRange('m6', today).from).toBe('2026-03-01'); expect(periodRange('y1', today).from).toBe('2025-09-01') })
  it('all is open', () => { expect(periodRange('all', today)).toEqual({ from: null, to: null }) })
  it('custom passes through', () => { expect(periodRange('custom', today, { from: '2026-01-01', to: '2026-01-31' })).toEqual({ from: '2026-01-01', to: '2026-01-31' }) })
  it('january wraps the year', () => { expect(periodRange('last_month', new Date(2026, 0, 15))).toEqual({ from: '2025-12-01', to: '2025-12-31' }) })
})

describe('monthRange', () => {
  it('spans a 31-day month', () => { expect(monthRange('2026-01')).toEqual({ from: '2026-01-01', to: '2026-01-31' }) })
  it('spans February in a non-leap year', () => { expect(monthRange('2026-02')).toEqual({ from: '2026-02-01', to: '2026-02-28' }) })
  it('spans February in a leap year', () => { expect(monthRange('2028-02')).toEqual({ from: '2028-02-01', to: '2028-02-29' }) })
})
