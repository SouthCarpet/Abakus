import { describe, expect, it } from 'vitest'
import type { StatementHistoryRow } from '../../api'
import { statementsOverlappingWindow } from './balances'

const statement = (overrides: Partial<StatementHistoryRow> = {}): StatementHistoryRow => ({
  statement_id: 1,
  account_id: 1,
  account_label: 'Osobný',
  account_kind: 'personal',
  number: 1,
  period_start: '2026-01-01',
  period_end: '2026-01-31',
  opening_cents: 10000,
  closing_cents: 12000,
  checksum: { status: 'ok' },
  ...overrides,
})

describe('statementsOverlappingWindow', () => {
  it('keeps a statement whose period is fully inside the window', () => {
    expect(statementsOverlappingWindow([statement()], { from: '2026-01-01', to: '2026-01-31' })).toHaveLength(1)
  })
  it('drops a statement whose period ends the day before the window starts', () => {
    expect(statementsOverlappingWindow([statement({ period_end: '2025-12-31' })], { from: '2026-01-01', to: '2026-01-31' })).toHaveLength(0)
  })
  it('keeps a statement that only partially overlaps the trailing edge of the window', () => {
    expect(statementsOverlappingWindow([statement({ period_start: '2026-01-25', period_end: '2026-02-10' })], { from: '2026-01-01', to: '2026-01-31' })).toHaveLength(1)
  })
  it('keeps every statement when the window is null (Všetko)', () => {
    expect(statementsOverlappingWindow([statement({ period_start: '2020-01-01', period_end: '2020-01-31' })], null)).toHaveLength(1)
  })
  it('sorts by account, then by period start, so two accounts do not interleave', () => {
    const rows = statementsOverlappingWindow([
      statement({ statement_id: 2, account_id: 2, period_start: '2026-01-05', period_end: '2026-01-31' }),
      statement({ statement_id: 1, account_id: 1, period_start: '2026-01-10', period_end: '2026-01-31' }),
    ], null)
    expect(rows.map((r) => r.statement_id)).toEqual([1, 2])
  })
  it('preserves a null closing balance as unavailable rather than substituting a value', () => {
    const rows = statementsOverlappingWindow([statement({ closing_cents: null })], { from: '2026-01-01', to: '2026-01-31' })
    expect(rows[0].closing_cents).toBeNull()
  })
})
