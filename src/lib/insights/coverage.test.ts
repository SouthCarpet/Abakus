import { describe, expect, it } from 'vitest'
import type { Account, StatementHistoryRow } from '../../api'
import { anyGaps, buildAccountCoverage, buildCoverage, gapsWithinWindow, internalGaps, mergeRanges } from './coverage'

const account = (overrides: Partial<Account> = {}): Account => ({
  id: 1,
  iban: 'SK4411000000000012345678',
  kind: 'personal',
  label: 'Osobný',
  has_password: false,
  ...overrides,
})

const statement = (overrides: Partial<StatementHistoryRow> = {}): StatementHistoryRow => ({
  statement_id: 1,
  account_id: 1,
  account_label: 'Osobný',
  account_kind: 'personal',
  number: 1,
  period_start: '2026-01-01',
  period_end: '2026-01-31',
  opening_cents: 0,
  closing_cents: 0,
  checksum: { status: 'ok' },
  ...overrides,
})

describe('mergeRanges', () => {
  it('merges two overlapping ranges into one', () => {
    const { merged } = mergeRanges([{ from: '2026-01-01', to: '2026-01-20' }, { from: '2026-01-15', to: '2026-02-05' }])
    expect(merged).toEqual([{ from: '2026-01-01', to: '2026-02-05' }])
  })
  it('merges a range fully nested inside another', () => {
    const { merged } = mergeRanges([{ from: '2026-01-01', to: '2026-01-31' }, { from: '2026-01-10', to: '2026-01-20' }])
    expect(merged).toEqual([{ from: '2026-01-01', to: '2026-01-31' }])
  })
  it('merges an exact duplicate range into one', () => {
    const { merged } = mergeRanges([{ from: '2026-01-01', to: '2026-01-31' }, { from: '2026-01-01', to: '2026-01-31' }])
    expect(merged).toEqual([{ from: '2026-01-01', to: '2026-01-31' }])
  })
  it('merges two ranges that are immediately adjacent, with no gap day between them', () => {
    const { merged } = mergeRanges([{ from: '2026-01-01', to: '2026-01-31' }, { from: '2026-02-01', to: '2026-02-28' }])
    expect(merged).toEqual([{ from: '2026-01-01', to: '2026-02-28' }])
  })
  it('keeps two ranges separate when a one-day gap sits between them', () => {
    const { merged } = mergeRanges([{ from: '2026-01-01', to: '2026-01-30' }, { from: '2026-02-01', to: '2026-02-28' }])
    expect(merged).toEqual([{ from: '2026-01-01', to: '2026-01-30' }, { from: '2026-02-01', to: '2026-02-28' }])
  })
  it('sets a reversed range aside as invalid instead of merging it', () => {
    const { merged, invalid } = mergeRanges([{ from: '2026-01-01', to: '2026-01-31' }, { from: '2026-02-10', to: '2026-02-01' }])
    expect(merged).toEqual([{ from: '2026-01-01', to: '2026-01-31' }])
    expect(invalid).toEqual([{ from: '2026-02-10', to: '2026-02-01' }])
  })
  // Oracle: a statement naming a day that does not exist on the calendar
  // (2026-02-30) lexically precedes '2026-03-01', so a from<=to check alone
  // reads it as valid and would grant coverage for a nonexistent day.
  it('sets a range naming a nonexistent calendar day aside as invalid instead of granting coverage for it', () => {
    const { merged, invalid } = mergeRanges([{ from: '2026-02-30', to: '2026-03-01' }])
    expect(merged).toEqual([])
    expect(invalid).toEqual([{ from: '2026-02-30', to: '2026-03-01' }])
  })
})

describe('gapsWithinWindow (finite selected period)', () => {
  it('reports no gap when merged coverage fully spans the window', () => {
    expect(gapsWithinWindow([{ from: '2026-01-01', to: '2026-01-31' }], { from: '2026-01-01', to: '2026-01-31' })).toEqual([])
  })
  it('reports the leading edge gap before the first known range', () => {
    expect(gapsWithinWindow([{ from: '2026-01-10', to: '2026-01-31' }], { from: '2026-01-01', to: '2026-01-31' }))
      .toEqual([{ from: '2026-01-01', to: '2026-01-09' }])
  })
  it('reports the trailing edge gap after the last known range', () => {
    expect(gapsWithinWindow([{ from: '2026-01-01', to: '2026-01-20' }], { from: '2026-01-01', to: '2026-01-31' }))
      .toEqual([{ from: '2026-01-21', to: '2026-01-31' }])
  })
  it('reports the internal gap between two known ranges', () => {
    expect(gapsWithinWindow(
      [{ from: '2026-01-01', to: '2026-01-10' }, { from: '2026-01-20', to: '2026-01-31' }],
      { from: '2026-01-01', to: '2026-01-31' },
    )).toEqual([{ from: '2026-01-11', to: '2026-01-19' }])
  })
  it('clips a range that only partially overlaps the window, so it contributes only actual coverage', () => {
    expect(gapsWithinWindow([{ from: '2025-12-15', to: '2026-01-10' }], { from: '2026-01-01', to: '2026-01-31' }))
      .toEqual([{ from: '2026-01-11', to: '2026-01-31' }])
  })
  it('reports the whole window as a gap when there is no coverage at all', () => {
    expect(gapsWithinWindow([], { from: '2026-01-01', to: '2026-01-31' })).toEqual([{ from: '2026-01-01', to: '2026-01-31' }])
  })
})

describe('internalGaps (Všetko: gaps only between known ranges)', () => {
  it('finds no gap for a single known range', () => {
    expect(internalGaps([{ from: '2026-01-01', to: '2026-01-31' }])).toEqual([])
  })
  it('finds the gap between two known ranges and invents nothing before the first or after the last', () => {
    expect(internalGaps([{ from: '2026-01-01', to: '2026-01-10' }, { from: '2026-03-01', to: '2026-03-31' }]))
      .toEqual([{ from: '2026-01-11', to: '2026-02-28' }])
  })
})

describe('buildAccountCoverage', () => {
  it('labels an account with no statements as having none (Bez výpisov), not complete', () => {
    const coverage = buildAccountCoverage(account(), [], { from: '2026-01-01', to: '2026-01-31' })
    expect(coverage.hasStatements).toBe(false)
    expect(coverage.knownFrom).toBeNull()
    expect(coverage.knownTo).toBeNull()
  })
  it('never lets a reversed range grant completeness: the whole window still shows as a gap, and the range is flagged for review', () => {
    const coverage = buildAccountCoverage(account(), [statement({ period_start: '2026-01-31', period_end: '2026-01-01' })], { from: '2026-01-01', to: '2026-01-31' })
    expect(coverage.hasStatements).toBe(true)
    expect(coverage.gaps).toEqual([{ from: '2026-01-01', to: '2026-01-31' }])
    expect(coverage.invalidRanges).toEqual([{ from: '2026-01-31', to: '2026-01-01' }])
  })
  it('never borrows coverage from a different account sharing the same window', () => {
    const statements = [
      statement({ statement_id: 1, account_id: 1, period_start: '2026-01-01', period_end: '2026-01-31' }),
      statement({ statement_id: 2, account_id: 2, period_start: '2026-01-01', period_end: '2026-01-31' }),
    ]
    const coverage = buildAccountCoverage(account({ id: 3, label: 'Tretí' }), statements, { from: '2026-01-01', to: '2026-01-31' })
    expect(coverage.hasStatements).toBe(false)
    expect(coverage.gaps).toEqual([{ from: '2026-01-01', to: '2026-01-31' }])
  })
  it('reports only the internal known-history gap for Všetko (window null), stating the known bounds', () => {
    const statements = [
      statement({ statement_id: 1, period_start: '2026-01-01', period_end: '2026-01-10' }),
      statement({ statement_id: 2, period_start: '2026-03-01', period_end: '2026-03-31' }),
    ]
    const coverage = buildAccountCoverage(account(), statements, null)
    expect(coverage.knownFrom).toBe('2026-01-01')
    expect(coverage.knownTo).toBe('2026-03-31')
    expect(coverage.gaps).toEqual([{ from: '2026-01-11', to: '2026-02-28' }])
  })
  it('invents no gap before the first or after the last known range for Všetko, even though the window would have edge gaps', () => {
    const coverage = buildAccountCoverage(account(), [statement({ period_start: '2026-01-10', period_end: '2026-01-20' })], null)
    expect(coverage.gaps).toEqual([])
  })
})

describe('buildCoverage', () => {
  it('includes only accounts of the selected kind', () => {
    const accounts = [account({ id: 1, kind: 'personal' }), account({ id: 2, kind: 'business', label: 'Firemný' })]
    const result = buildCoverage(accounts, [], 'personal', { from: '2026-01-01', to: '2026-01-31' })
    expect(result.map((c) => c.accountId)).toEqual([1])
  })
  it('includes every account when no kind filter is applied', () => {
    const accounts = [account({ id: 1, kind: 'personal' }), account({ id: 2, kind: 'business', label: 'Firemný' })]
    const result = buildCoverage(accounts, [], null, { from: '2026-01-01', to: '2026-01-31' })
    expect(result.map((c) => c.accountId)).toEqual([1, 2])
  })
})

describe('anyGaps', () => {
  it('is true when at least one account has a gap', () => {
    const coverage = buildAccountCoverage(account(), [], { from: '2026-01-01', to: '2026-01-31' })
    expect(anyGaps([coverage])).toBe(true)
  })
  it('is false when no account has a gap', () => {
    const coverage = buildAccountCoverage(account(), [statement()], { from: '2026-01-01', to: '2026-01-31' })
    expect(anyGaps([coverage])).toBe(false)
  })
})
