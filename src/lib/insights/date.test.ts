import { describe, expect, it } from 'vitest'
import { addDaysIso, dayCountInclusive, isValidIsoDate, isValidRange, rangesOverlap } from './date'

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
  // Oracle: direct Node proof before the repair found isValidRange({from:
  // '2026-02-30', to: '2026-03-01'}) === true. Lexical from<=to comparison
  // alone cannot see that February never has a 30th.
  it('is false when from names a day that does not exist on the calendar, even though it lexically precedes to', () => {
    expect(isValidRange({ from: '2026-02-30', to: '2026-03-01' })).toBe(false)
  })
  it('is false when to names a day that does not exist on the calendar', () => {
    expect(isValidRange({ from: '2026-03-01', to: '2026-04-31' })).toBe(false)
  })
})

describe('isValidIsoDate', () => {
  it('accepts a real calendar date', () => {
    expect(isValidIsoDate('2026-02-28')).toBe(true)
  })
  it('accepts the leap day in a leap year', () => {
    expect(isValidIsoDate('2028-02-29')).toBe(true)
  })
  it('rejects the leap day in a non-leap year', () => {
    expect(isValidIsoDate('2026-02-29')).toBe(false)
  })
  it('rejects a malformed string', () => {
    expect(isValidIsoDate('2026-2-1')).toBe(false)
  })
})

// Oracle: direct Node proof before the repair (`node --experimental-strip-types`)
// found addDaysIso('0026-01-01', 1) === '1926-01-02'. Date.UTC(26, 0, 1)
// silently remaps a 0-99 year argument to 1900-1999; setUTCFullYear does not.
describe('addDaysIso preserves a low four-digit year instead of remapping it to 19xx', () => {
  it('keeps year 0026 as 0026, not 1926, stepping one day forward', () => {
    expect(addDaysIso('0026-01-01', 1)).toBe('0026-01-02')
  })
  it('keeps year 0099 as 0099 stepping across a year boundary', () => {
    expect(addDaysIso('0099-12-31', 1)).toBe('0100-01-01')
  })
})

describe('addDaysIso rejects arithmetic that would leave the representable YYYY-MM-DD year range', () => {
  it('throws instead of emitting a negative or truncated year before 0000', () => {
    expect(() => addDaysIso('0000-01-01', -1)).toThrow(RangeError)
  })
  it('throws instead of emitting a 5-digit year after 9999', () => {
    expect(() => addDaysIso('9999-12-31', 1)).toThrow(RangeError)
  })
})
