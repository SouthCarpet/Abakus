import type { Category, TxRow } from '../../api'
import type { RecurringAmounts, RecurringOverview, RecurringRow } from '../../lib/recurring-api'

export const ZERO_AMOUNTS: RecurringAmounts = {
  monthly_income_cents: 0,
  monthly_expense_cents: 0,
  monthly_net_cents: 0,
  annual_income_cents: 0,
  annual_expense_cents: 0,
  annual_net_cents: 0,
  remaining_income_cents: 0,
  remaining_expense_cents: 0,
}

export function recurringRow(overrides: Partial<RecurringRow> = {}): RecurringRow {
  return {
    series_key: 'g:netflix',
    group_key: 'netflix-key',
    decision_id: 1,
    scope: 'group',
    decision: 'confirmed',
    account_id: 1,
    account_label: 'Osobný',
    account_kind: 'personal',
    direction: 'expense',
    name: 'Netflix',
    cadence: 'monthly',
    anchor_date: '2026-01-15',
    subscription_category: true,
    category_id: 10,
    evidence_count: 8,
    last_paid: '2026-08-15',
    next_due: '2026-09-15',
    state: 'upcoming',
    unknown_reason: null,
    grace_until: null,
    currency: 'EUR',
    original_amount_cents: null,
    amount_cents: 1200,
    amount_basis: 'stable_pair',
    monthly_cents: 1200,
    annual_cents: 14400,
    price_change: null,
    manual_membership: false,
    foreign_eur_estimate: false,
    ...overrides,
  }
}

export function txRow(overrides: Partial<TxRow> = {}): TxRow {
  return {
    id: 11,
    account_id: 1,
    account_kind: 'personal',
    statement_number: 3,
    posted_date: '2026-01-15',
    tx_date: '2026-01-15',
    kind: 'Card',
    amount_cents: -1200,
    orig_amount_cents: null,
    orig_currency: null,
    merchant_raw: 'Netflix',
    place: null,
    counterparty_name: null,
    counterparty_iban: null,
    category_id: 10,
    category_name: 'Netflix',
    parent_name: 'Predplatné',
    status: 'confirmed',
    source: 'pdf',
    raw_block: 'NETFLIX',
    note: '',
    ...overrides,
  }
}

export const SAMPLE_CATEGORY: Category = {
  id: 10,
  parent_id: 9,
  name: 'Netflix',
  kind: 'expense',
  sort: 1,
  system: false,
  archived: false,
}

// Oracle: recurring-acceptance R14, today 2026-09-01 schedule.
export const R14_MONTHLY_EXPENSE = recurringRow({
  series_key: 'g:monthly-exp',
  group_key: 'monthly-exp',
  name: 'Nájom',
  cadence: 'monthly',
  amount_cents: 1200,
  monthly_cents: 1200,
  annual_cents: 14400,
  next_due: '2026-09-15',
  state: 'upcoming',
})

export const R14_ANNUAL_EXPENSE = recurringRow({
  series_key: 'g:annual-exp',
  group_key: 'annual-exp',
  decision_id: 2,
  name: 'Poistenie',
  cadence: 'yearly',
  amount_cents: 10000,
  monthly_cents: 833,
  annual_cents: 10000,
  next_due: '2026-12-01',
  state: 'active',
  last_paid: '2025-12-01',
})

export const R14_QUARTERLY_EXPENSE = recurringRow({
  series_key: 'g:quarter-exp',
  group_key: 'quarter-exp',
  decision_id: 3,
  name: 'Daň',
  cadence: 'quarterly',
  amount_cents: 10001,
  monthly_cents: 3334,
  annual_cents: 40004,
  next_due: '2026-10-01',
  state: 'active',
  last_paid: '2026-07-01',
})

export const R14_MONTHLY_INCOME = recurringRow({
  series_key: 'g:monthly-inc',
  group_key: 'monthly-inc',
  decision_id: 4,
  name: 'Faktúra',
  direction: 'income',
  cadence: 'monthly',
  amount_cents: 250000,
  monthly_cents: 250000,
  annual_cents: 3000000,
  next_due: '2026-09-20',
  state: 'upcoming',
  subscription_category: false,
  category_id: 20,
})

export const R14_ESTIMATE = recurringRow({
  series_key: 'g:estimate',
  group_key: 'estimate',
  decision_id: null,
  decision: 'estimate',
  name: 'Hosting',
  cadence: 'monthly',
  amount_cents: 500,
  monthly_cents: 500,
  annual_cents: 6000,
  next_due: '2026-09-20',
  state: 'upcoming',
  subscription_category: false,
  category_id: null,
})

export const R14_CONFIRMED: RecurringAmounts = {
  monthly_income_cents: 250000,
  monthly_expense_cents: 5367,
  monthly_net_cents: 244633,
  annual_income_cents: 3000000,
  annual_expense_cents: 64404,
  annual_net_cents: 2935596,
  remaining_income_cents: 250000,
  remaining_expense_cents: 1200,
}

export const R14_ESTIMATES: RecurringAmounts = {
  monthly_income_cents: 0,
  monthly_expense_cents: 500,
  monthly_net_cents: -500,
  annual_income_cents: 0,
  annual_expense_cents: 6000,
  annual_net_cents: -6000,
  remaining_income_cents: 0,
  remaining_expense_cents: 500,
}

export const R14_OVERVIEW: RecurringOverview = {
  as_of: '2026-09-01',
  history_from: '2025-12-01',
  future_period: false,
  unfinished_period: true,
  rows: [R14_MONTHLY_EXPENSE, R14_ANNUAL_EXPENSE, R14_QUARTERLY_EXPENSE, R14_MONTHLY_INCOME, R14_ESTIMATE],
  confirmed: R14_CONFIRMED,
  estimates: R14_ESTIMATES,
  excluded: { missing: 0, ended: 0, unknown: 0 },
  expense_share_basis_points: 488,
  average_expense_cents: 110000,
  average_months: ['2026-07', '2026-08'],
}

export const EMPTY_OVERVIEW: RecurringOverview = {
  as_of: '2026-09-07',
  history_from: null,
  future_period: false,
  unfinished_period: true,
  rows: [],
  confirmed: ZERO_AMOUNTS,
  estimates: ZERO_AMOUNTS,
  excluded: { missing: 0, ended: 0, unknown: 0 },
  expense_share_basis_points: null,
  average_expense_cents: null,
  average_months: [],
}

export function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}
