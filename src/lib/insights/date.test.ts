import { describe, expect, it } from 'vitest'
import { addDaysIso, dayCountInclusive, isValidRange, rangesOverlap } from './date'

describe('addDaysIso crosses date boundaries using UTC calendar days', () => {
  it('steps forward across a leap day: 2028-02-28 + 1 = 2028-02-29', () => {
    expect(addDaysIso('2028-02-28', 1)).toBe('2028-02-29')
  })
  it('steps backward across a leap day: 2028-03-01 - 1 = 2028-02-29', () => {
    expect(addDaysIso('2028-03-01', -1)).toBe('2028-02-29')
  })
  it('skips February 29 in a non-leap year: 2026-02-28 + 1 = 2026-03-01', () => {
    expect(addDaysIso('2026-02-28', 1)).toBe('2026-03-01')
  })
  it('steps forward across a year boundary: 2026-12-31 + 1 = 2027-01-01', () => {
    expect(addDaysIso('2026-12-31', 1)).toBe('2027-01-01')
  })
  it('steps across the Slovak DST spring-forward date unaffected by the missing local hour: 2026-03-28 + 1 = 2026-03-29', () => {
    expect(addDaysIso('2026-03-28', 1)).toBe('2026-03-29')
  })
  it('steps across the Slovak DST fall-back date unaffected by the repeated local hour: 2026-10-24 + 1 = 2026-10-25', () => {
    expect(addDaysIso('2026-10-24', 1)).toBe('2026-10-25')
  })
})

describe('dayCountInclusive', () => {
  it('counts a single day as 1', () => {
    expect(dayCountInclusive({ from: '2026-06-01', to: '2026-06-01' })).toBe(1)
  })
  it('counts a 31-day month inclusively', () => {
    expect(dayCountInclusive({ from: '2026-08-01', to: '2026-08-31' })).toBe(31)
  })
  it('counts across a leap day', () => {
    expect(dayCountInclusive({ from: '2028-02-01', to: '2028-02-29' })).toBe(29)
  })
})

describe('rangesOverlap', () => {
  it('is true when ranges share exactly one boundary day', () => {
    expect(rangesOverlap({ from: '2026-01-01', to: '2026-01-10' }, { from: '2026-01-10', to: '2026-01-20' })).toBe(true)
  })
  it('is false when one range ends the day before the other starts', () => {
    expect(rangesOverlap({ from: '2026-01-01', to: '2026-01-09' }, { from: '2026-01-10', to: '2026-01-20' })).toBe(false)
  })
  it('is true when one range nests fully inside the other', () => {
    expect(rangesOverlap({ from: '2026-01-01', to: '2026-01-31' }, { from: '2026-01-10', to: '2026-01-20' })).toBe(true)
  })
})

describe('isValidRange', () => {
  it('is true when from is before to', () => {
    expect(isValidRange({ from: '2026-01-01', to: '2026-01-02' })).toBe(true)
  })
  it('is true when from equals to', () => {
    expect(isValidRange({ from: '2026-01-01', to: '2026-01-01' })).toBe(true)
  })
  it('is false for a reversed range where from is after to', () => {
    expect(isValidRange({ from: '2026-01-02', to: '2026-01-01' })).toBe(false)
  })
})
