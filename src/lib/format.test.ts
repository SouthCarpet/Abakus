import { describe, expect, it } from 'vitest'
import { checksumLabel, formatDate, formatEur, importStatusLabel } from './format'
describe('format', () => {
  it('formats cents in Slovak style', () => { expect(formatEur(-123456)).toBe('-1 234,56 €'); expect(formatEur(5)).toBe('0,05 €') })
  it('formats ISO dates without leading zeros', () => { expect(formatDate('2026-06-01')).toBe('1. 6. 2026') })
  it('labels checksums', () => {
    expect(checksumLabel({ status: 'ok' })).toBe('Kontrolný súčet sedí')
    expect(checksumLabel({ status: 'off_by', off_by: 32 })).toBe('Kontrolný súčet nesedí o 0,32 €')
    expect(checksumLabel({ status: 'not_verifiable' })).toBe('Kontrolný súčet sa nedá overiť')
    expect(checksumLabel(null)).toBe('')
  })
  it('labels import statuses', () => { expect(importStatusLabel('locked')).toBe('Zamknuté PDF'); expect(importStatusLabel('unknown_account')).toBe('Neznámy účet') })
})
