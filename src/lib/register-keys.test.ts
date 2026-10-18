import { describe, expect, it } from 'vitest'
import type { TxRow } from '../api'
import { nextFocus, rangeIds, selectAll } from './register-keys'

const row = (id: number, status: TxRow['status'] = 'unassigned') => ({ id, status }) as TxRow
const rows = Array.from({ length: 25 }, (_, i) => row(i + 1, i === 2 ? 'transfer' : 'unassigned'))

// Oracle: Kaliber components.md "Selection model" and "Keyboard map" (Home, End, PgUp, PgDn;
// Shift range from the anchor; Ctrl+A all, a second Ctrl+A none); PAGE_ROWS = 10.
describe('register keys', () => {
  it('moves the focus by one, by a page and to the ends, clamped', () => {
    expect(nextFocus(rows, 5, 'ArrowDown')).toBe(6)
    expect(nextFocus(rows, 5, 'ArrowUp')).toBe(4)
    expect(nextFocus(rows, 5, 'PageDown')).toBe(15)
    expect(nextFocus(rows, 20, 'PageDown')).toBe(25)
    expect(nextFocus(rows, 5, 'PageUp')).toBe(1)
    expect(nextFocus(rows, 5, 'Home')).toBe(1)
    expect(nextFocus(rows, 5, 'End')).toBe(25)
    expect(nextFocus(rows, 1, 'ArrowUp')).toBe(1)
  })

  it('starts at the first or the last row when nothing has the focus', () => {
    expect(nextFocus(rows, null, 'ArrowDown')).toBe(1)
    expect(nextFocus(rows, null, 'End')).toBe(25)
    expect(nextFocus(rows, null, 'ArrowUp')).toBe(25)
    expect(nextFocus([], null, 'ArrowDown')).toBeNull()
  })

  it('selects a range from the anchor in either direction and skips a transfer', () => {
    expect(rangeIds(rows, 1, 5)).toEqual([1, 2, 4, 5])
    expect(rangeIds(rows, 5, 1)).toEqual([1, 2, 4, 5])
    expect(rangeIds(rows, null, 4)).toEqual([4])
  })

  it('selects every visible selectable row, then none on the second press', () => {
    const first = selectAll(rows, new Set())
    expect(first.size).toBe(24)
    expect(first.has(3)).toBe(false)
    expect(selectAll(rows, first).size).toBe(0)
    expect(selectAll(rows, new Set([1])).size).toBe(24)
  })
})
