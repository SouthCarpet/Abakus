import type { Account, AccountKind, StatementHistoryRow } from '../../api'
import { gapsWithinWindow, mergeRanges } from './coverage'
import { addDaysIso, isValidIsoDate, type DateRange } from './date'

export interface StatementReminder {
  accountId: number
  accountLabel: string
  accountKind: AccountKind
  missingRanges: DateRange[]
}

export interface StatementRangeReview {
  accountId: number
  accountLabel: string
  accountKind: AccountKind
  invalidRanges: DateRange[]
}

export interface StatementReminderResult {
  targetMonth: DateRange | null
  reminders: StatementReminder[]
  review: StatementRangeReview[]
}

function previousMonth(today: string): DateRange | null {
  if (!isValidIsoDate(today)) throw new RangeError(`Not a valid YYYY-MM-DD date: ${today}`)
  if (today.startsWith('0000-01-')) return null

  const previousMonthEnd = addDaysIso(`${today.slice(0, 7)}-01`, -1)
  return { from: `${previousMonthEnd.slice(0, 7)}-01`, to: previousMonthEnd }
}

function reminderForAccount(
  account: Pick<Account, 'id' | 'label' | 'kind'>,
  merged: DateRange[],
  invalid: DateRange[],
  targetMonth: DateRange,
  remindersAreDue: boolean,
): StatementReminder | null {
  if (!remindersAreDue || invalid.length > 0 || merged.length === 0) return null

  const checkedFrom = merged[0].from > targetMonth.from ? merged[0].from : targetMonth.from
  if (checkedFrom > targetMonth.to) return null

  const missingRanges = gapsWithinWindow(merged, { from: checkedFrom, to: targetMonth.to })
  if (missingRanges.length === 0) return null

  return {
    accountId: account.id,
    accountLabel: account.label,
    accountKind: account.kind,
    missingRanges,
  }
}

export function buildStatementReminders(
  accounts: readonly Pick<Account, 'id' | 'label' | 'kind'>[],
  statements: readonly StatementHistoryRow[],
  accountKind: AccountKind | null,
  today: string,
): StatementReminderResult {
  const targetMonth = previousMonth(today)
  const reminders: StatementReminder[] = []
  const review: StatementRangeReview[] = []

  for (const account of accounts) {
    if (accountKind !== null && account.kind !== accountKind) continue
    const rows = statements.filter(
      (row) => row.account_id === account.id && row.account_kind === account.kind,
    )
    const { merged, invalid } = mergeRanges(
      rows.map((row) => ({ from: row.period_start, to: row.period_end })),
    )
    if (invalid.length > 0) {
      review.push({
        accountId: account.id,
        accountLabel: account.label,
        accountKind: account.kind,
        invalidRanges: invalid,
      })
    }
    if (targetMonth === null) continue
    const reminder = reminderForAccount(
      account,
      merged,
      invalid,
      targetMonth,
      Number(today.slice(8, 10)) >= 8,
    )
    if (reminder) reminders.push(reminder)
  }

  return { targetMonth, reminders, review }
}
