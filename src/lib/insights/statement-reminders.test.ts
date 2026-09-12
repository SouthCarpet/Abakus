import { describe, expect, it } from 'vitest'
import type { Account, AccountKind, StatementHistoryRow } from '../../api'
import { buildStatementReminders } from './statement-reminders'

const account = (
  id = 1,
  kind: AccountKind = 'personal',
  label = 'Osobný',
): Pick<Account, 'id' | 'label' | 'kind'> => ({ id, kind, label })

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
  transaction_count: 0,
  total_cents: 0,
  checksum: { status: 'ok' },
  ...overrides,
})

describe('buildStatementReminders', () => {
  it('starts previous-month reminders on day 8 after seven full grace days', () => {
    const statements = [statement({ period_end: '2026-02-20' })]

    const daySeven = buildStatementReminders([account()], statements, 'personal', '2026-03-07')
    const dayEight = buildStatementReminders([account()], statements, 'personal', '2026-03-08')

    expect(daySeven.targetMonth).toEqual({ from: '2026-02-01', to: '2026-02-28' })
    expect(daySeven.reminders).toEqual([])
    expect(dayEight.reminders).toEqual([
      {
        accountId: 1,
        accountLabel: 'Osobný',
        accountKind: 'personal',
        missingRanges: [{ from: '2026-02-21', to: '2026-02-28' }],
      },
    ])
  })

  it('targets December of the previous year when today is in January', () => {
    const result = buildStatementReminders(
      [account()],
      [statement({ period_start: '2025-11-01', period_end: '2025-11-30' })],
      'personal',
      '2026-01-08',
    )

    expect(result.targetMonth).toEqual({ from: '2025-12-01', to: '2025-12-31' })
    expect(result.reminders[0]?.missingRanges).toEqual([{ from: '2025-12-01', to: '2025-12-31' }])
  })

  it('uses February 28 as the previous-month end in an ordinary year', () => {
    const result = buildStatementReminders(
      [account()],
      [statement({ period_end: '2026-02-27' })],
      'personal',
      '2026-03-08',
    )

    expect(result.targetMonth).toEqual({ from: '2026-02-01', to: '2026-02-28' })
    expect(result.reminders[0]?.missingRanges).toEqual([{ from: '2026-02-28', to: '2026-02-28' }])
  })

  it('uses February 29 as the previous-month end in a leap year', () => {
    const result = buildStatementReminders(
      [account()],
      [statement({ period_start: '2028-01-01', period_end: '2028-02-28' })],
      'personal',
      '2028-03-08',
    )

    expect(result.targetMonth).toEqual({ from: '2028-02-01', to: '2028-02-29' })
    expect(result.reminders[0]?.missingRanges).toEqual([{ from: '2028-02-29', to: '2028-02-29' }])
  })

  it('preserves a low four-digit year when it computes the previous month', () => {
    const result = buildStatementReminders(
      [account()],
      [statement({ period_start: '0026-01-01', period_end: '0026-02-20' })],
      'personal',
      '0026-03-08',
    )

    expect(result.targetMonth).toEqual({ from: '0026-02-01', to: '0026-02-28' })
    expect(result.reminders[0]?.missingRanges).toEqual([{ from: '0026-02-21', to: '0026-02-28' }])
  })

  it('returns no target or reminder before the supported calendar at year 0000 January', () => {
    const result = buildStatementReminders([account()], [statement()], 'personal', '0000-01-08')

    expect(result.targetMonth).toBeNull()
    expect(result.reminders).toEqual([])
  })

  it('throws RangeError for malformed or nonexistent today dates', () => {
    expect(() => buildStatementReminders([], [], null, '2026-3-08')).toThrow(RangeError)
    expect(() => buildStatementReminders([], [], null, '2026-02-30')).toThrow(RangeError)
  })

  it('emits no reminder or review for an account with empty history', () => {
    const result = buildStatementReminders([account()], [], 'personal', '2026-03-08')

    expect(result.reminders).toEqual([])
    expect(result.review).toEqual([])
  })

  it('reports invalid-only history for review without treating it as missing coverage', () => {
    const result = buildStatementReminders(
      [account()],
      [statement({ period_start: '2026-02-28', period_end: '2026-02-01' })],
      'personal',
      '2026-03-08',
    )

    expect(result.reminders).toEqual([])
    expect(result.review).toEqual([
      {
        accountId: 1,
        accountLabel: 'Osobný',
        accountKind: 'personal',
        invalidRanges: [{ from: '2026-02-28', to: '2026-02-01' }],
      },
    ])
  })

  it('emits no past reminder when all valid history starts after the target month', () => {
    const result = buildStatementReminders(
      [account()],
      [statement({ period_start: '2026-03-01', period_end: '2026-03-31' })],
      'personal',
      '2026-03-08',
    )

    expect(result.reminders).toEqual([])
  })

  it('clips the checked interval to a first valid statement that starts within the target month', () => {
    const result = buildStatementReminders(
      [account()],
      [statement({ period_start: '2026-02-10', period_end: '2026-02-20' })],
      'personal',
      '2026-03-08',
    )

    expect(result.reminders[0]?.missingRanges).toEqual([{ from: '2026-02-21', to: '2026-02-28' }])
  })

  it('returns no reminder for full target coverage regardless of transaction count or checksum', () => {
    const result = buildStatementReminders(
      [account()],
      [
        statement({
          period_start: '2026-02-01',
          period_end: '2026-02-28',
          transaction_count: 0,
          checksum: { status: 'off_by', off_by: 500 },
        }),
      ],
      'personal',
      '2026-03-08',
    )

    expect(result.reminders).toEqual([])
  })

  it('returns exact partial gaps after merging adjacent, nested and duplicate ranges', () => {
    const result = buildStatementReminders(
      [account()],
      [
        statement({ statement_id: 1, period_start: '2026-01-01', period_end: '2026-02-05' }),
        statement({ statement_id: 2, period_start: '2026-02-01', period_end: '2026-02-05' }),
        statement({ statement_id: 3, period_start: '2026-02-02', period_end: '2026-02-04' }),
        statement({ statement_id: 4, period_start: '2026-02-06', period_end: '2026-02-10' }),
        statement({ statement_id: 5, period_start: '2026-02-15', period_end: '2026-02-20' }),
        statement({ statement_id: 6, period_start: '2026-02-21', period_end: '2026-02-25' }),
      ],
      'personal',
      '2026-03-08',
    )

    expect(result.reminders[0]?.missingRanges).toEqual([
      { from: '2026-02-11', to: '2026-02-14' },
      { from: '2026-02-26', to: '2026-02-28' },
    ])
  })

  it('isolates account IDs and row kinds for personal, business and unfiltered selection', () => {
    const accounts = [account(2, 'business', 'Firma'), account(1, 'personal', 'Osobný')]
    const statements = [
      statement({ statement_id: 1, account_id: 2, account_kind: 'business' }),
      statement({ statement_id: 2, account_id: 1, account_kind: 'personal' }),
      statement({ statement_id: 3, account_id: 1, account_kind: 'business', period_start: '2026-02-01', period_end: '2026-02-28' }),
      statement({ statement_id: 4, account_id: 99, account_kind: 'personal', period_start: '2026-02-01', period_end: '2026-02-28' }),
      statement({ statement_id: 5, account_id: 1, account_kind: 'business', period_start: '2026-02-30', period_end: '2026-03-01' }),
      statement({ statement_id: 6, account_id: 99, account_kind: 'personal', period_start: '2026-02-30', period_end: '2026-03-01' }),
    ]

    const personal = buildStatementReminders(accounts, statements, 'personal', '2026-03-08')
    const business = buildStatementReminders(accounts, statements, 'business', '2026-03-08')
    const all = buildStatementReminders(accounts, statements, null, '2026-03-08')

    expect(personal.reminders.map((item) => item.accountId)).toEqual([1])
    expect(business.reminders.map((item) => item.accountId)).toEqual([2])
    expect(all.reminders.map((item) => item.accountId)).toEqual([2, 1])
    expect(all.reminders.map((item) => item.missingRanges)).toEqual([
      [{ from: '2026-02-01', to: '2026-02-28' }],
      [{ from: '2026-02-01', to: '2026-02-28' }],
    ])
    expect(all.review).toEqual([])
  })

  it('keeps mixed invalid history in review and suppresses only that account reminder', () => {
    const accounts = [account(1, 'personal', 'Prvý'), account(2, 'personal', 'Druhý')]
    const statements = [
      statement({ statement_id: 1, account_id: 1, account_label: 'Prvý' }),
      statement({ statement_id: 2, account_id: 1, account_label: 'Prvý', period_start: '2026-02-30', period_end: '2026-03-01' }),
      statement({ statement_id: 3, account_id: 2, account_label: 'Druhý' }),
    ]

    const duringGrace = buildStatementReminders(accounts, statements, null, '2026-03-07')
    const afterGrace = buildStatementReminders(accounts, statements, null, '2026-03-08')

    expect(duringGrace.review.map((item) => item.accountId)).toEqual([1])
    expect(afterGrace.review[0]?.invalidRanges).toEqual([{ from: '2026-02-30', to: '2026-03-01' }])
    expect(afterGrace.reminders.map((item) => item.accountId)).toEqual([2])
  })

  it('does not mutate readonly account or statement inputs', () => {
    const accounts = [account(1, 'personal', 'Osobný'), account(2, 'business', 'Firma')] as const
    const statements = [
      statement({ statement_id: 1, period_start: '2026-02-15', period_end: '2026-02-20' }),
      statement({ statement_id: 2, account_id: 2, account_kind: 'business', period_start: '2026-01-01', period_end: '2026-01-31' }),
    ] as const
    const accountsBefore = structuredClone(accounts)
    const statementsBefore = structuredClone(statements)

    buildStatementReminders(accounts, statements, null, '2026-03-08')

    expect(accounts).toEqual(accountsBefore)
    expect(statements).toEqual(statementsBefore)
  })

  it('does not emit an older-month backlog when the previous month is fully covered', () => {
    const result = buildStatementReminders(
      [account()],
      [
        statement({ statement_id: 1, period_start: '2026-01-01', period_end: '2026-01-10' }),
        statement({ statement_id: 2, period_start: '2026-02-01', period_end: '2026-02-28' }),
      ],
      'personal',
      '2026-03-08',
    )

    expect(result.reminders).toEqual([])
  })
})
