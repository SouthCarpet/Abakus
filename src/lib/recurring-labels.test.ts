import { describe, expect, it } from 'vitest'
import type { TxRow } from '../api'
import {
  actualChargeLabel,
  apportionmentLabel,
  cadenceLabel,
  cashDirectionCaption,
  categoryStatusLabel,
  emptyPanelExplanation,
  exclusionLabel,
  expenseShareLabel,
  formatBasisPoints,
  formatMoney,
  isRecurringEligible,
  localDateFromIso,
  localTodayIso,
  partitionVisibleRows,
  priceChangeLabel,
  remainingMonthLabel,
  snapshotCaption,
  stateDetail,
  stateLabel,
} from './recurring-labels'
import type { RecurringOverview, RecurringRow } from './recurring-api'

function row(overrides: Partial<RecurringRow> = {}): RecurringRow {
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

function tx(overrides: Partial<TxRow> = {}): TxRow {
  return {
    id: 1,
    account_id: 1,
    account_kind: 'personal',
    statement_number: 1,
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
    category_id: null,
    category_name: null,
    parent_name: null,
    status: 'unassigned',
    source: 'pdf',
    raw_block: '',
    note: '',
    ...overrides,
  }
}

describe('formatMoney uses integer cents, never float division', () => {
  it('formats EUR through the existing Slovak helper', () => {
    expect(formatMoney(1200, 'EUR')).toBe('12,00 €')
  })

  it('formats a foreign original amount with its ISO code', () => {
    // Oracle: contract R13, 1200 original USD cents.
    expect(formatMoney(1200, 'USD')).toBe('12,00 USD')
  })

  it('keeps a negative sign on the integer whole part', () => {
    expect(formatMoney(-1050, 'GBP')).toBe('-10,50 GBP')
  })
})

describe('formatBasisPoints', () => {
  it('renders the R15 share of 488 basis points as 4,88 %', () => {
    // Oracle: recurring-acceptance R15, 5367/110000 rounded half up to 488 bps.
    expect(formatBasisPoints(488)).toBe('4,88 %')
  })
})

describe('state labels are not color-only and keep honest unknown reasons', () => {
  it('labels upcoming, missing, ended and grace without calling ended a cancellation', () => {
    expect(stateLabel(row({ state: 'upcoming' }))).toBe('Príde do konca mesiaca')
    expect(stateLabel(row({ state: 'missing' }))).toBe('Chýba platba')
    expect(stateLabel(row({ state: 'ended' }))).toBe('Odhad ukončenia')
    expect(stateDetail(row({ state: 'ended' }))).toBe('Dve očakávané platby chýbajú v úplných výpisoch.')
    expect(stateLabel(row({ state: 'active', grace_until: '2026-07-25' }))).toBe('V tolerancii do 2026-07-25')
    expect(stateLabel(row({ state: 'active' }))).toBe('Aktívna')
  })

  it('names each unknown reason instead of a generic unknown', () => {
    expect(stateLabel(row({ state: 'unknown', unknown_reason: 'missing_coverage' }))).toBe('Neznáme, chýba výpis')
    expect(stateLabel(row({ state: 'unknown', unknown_reason: 'ambiguous_membership' }))).toBe('Neznáme, nejednoznačné členstvo')
    expect(stateLabel(row({ state: 'unknown', unknown_reason: 'no_evidence' }))).toBe('Neznáme, chýbajú doklady')
    expect(stateLabel(row({ state: 'unknown', unknown_reason: 'unmatched_history' }))).toBe('Neznáme, história nesedí')
  })
})

describe('remainingMonthLabel names a historical month and never calls it this month', () => {
  it('uses the contract example for August 2026 when today is September', () => {
    // Oracle: recurring-contract §2, "Zostávalo v auguste 2026".
    expect(remainingMonthLabel('2026-08-31', '2026-09-07')).toBe('Zostávalo v auguste 2026')
  })

  it('uses this-month wording only when as_of is in the same calendar month as today', () => {
    expect(remainingMonthLabel('2026-09-30', '2026-09-07')).toBe('Ešte príde tento mesiac')
  })

  it('names March 2026 for the R09 historical window ending 2026-03-31', () => {
    expect(remainingMonthLabel('2026-03-31', '2026-09-07')).toBe('Zostávalo v marci 2026')
    expect(remainingMonthLabel('2026-03-31', '2026-09-07')).not.toMatch(/tento mesiac/)
  })
})

describe('priceChangeLabel', () => {
  it('labels an expense increase with original currency and effective month', () => {
    const badge = priceChangeLabel(row({
      price_change: { currency: 'EUR', previous_cents: 1000, current_cents: 1200, delta_cents: 200, effective_from: '2026-04-15' },
    }))
    expect(badge).toBe('Zdraželo o 2,00 € od 2026-04')
  })

  it('uses a neutral income verb and never assumes salary', () => {
    const badge = priceChangeLabel(row({
      direction: 'income',
      price_change: { currency: 'EUR', previous_cents: 250000, current_cents: 260000, delta_cents: 10000, effective_from: '2026-06-01' },
    }))
    expect(badge).toBe('Príjem sa zvýšil o 100,00 € od 2026-06')
  })

  it('labels a USD original-currency increase, never an EUR FX move', () => {
    const badge = priceChangeLabel(row({
      currency: 'USD',
      price_change: { currency: 'USD', previous_cents: 1000, current_cents: 1200, delta_cents: 200, effective_from: '2026-05-01' },
    }))
    expect(badge).toBe('Zdraželo o 2,00 USD od 2026-05')
  })
})

describe('actual charge versus monthly apportionment', () => {
  it('shows original currency beside the booked EUR amount', () => {
    expect(actualChargeLabel(row({
      currency: 'USD',
      original_amount_cents: 1000,
      amount_cents: 950,
    }))).toBe('10,00 USD (9,50 €)')
  })

  it('labels a yearly row with the backend monthly cents, not a float 10000/12', () => {
    // Oracle: contract §2, 10000 annual cents = 833 monthly cents.
    expect(apportionmentLabel(row({
      cadence: 'yearly',
      amount_cents: 10000,
      monthly_cents: 833,
      annual_cents: 10000,
    }))).toBe('8,33 € mesačne (ročne)')
  })

  it('does not add an apportionment badge on a monthly row', () => {
    expect(apportionmentLabel(row({ cadence: 'monthly', monthly_cents: 1200 }))).toBeNull()
  })
})

describe('categoryStatusLabel', () => {
  it('uses Viac kategórií when mixed evidence leaves category_id null', () => {
    expect(categoryStatusLabel(row({ category_id: null }), null)).toBe('Viac kategórií')
  })
})

describe('expenseShareLabel never fabricates 0% when the backend sent null', () => {
  const emptyAmounts = {
    monthly_income_cents: 0,
    monthly_expense_cents: 0,
    monthly_net_cents: 0,
    annual_income_cents: 0,
    annual_expense_cents: 0,
    annual_net_cents: 0,
    remaining_income_cents: 0,
    remaining_expense_cents: 0,
  }
  const base: RecurringOverview = {
    as_of: '2026-09-07',
    history_from: '2026-01-31',
    future_period: false,
    unfinished_period: true,
    rows: [],
    confirmed: emptyAmounts,
    estimates: emptyAmounts,
    excluded: { missing: 0, ended: 0, unknown: 0 },
    expense_share_basis_points: null,
    average_expense_cents: null,
    average_months: [],
  }

  it('says unavailable rather than 0 % when R15 has no eligible months', () => {
    expect(expenseShareLabel(base)).toBe('Podiel na výdavkoch nie je dostupný.')
    expect(expenseShareLabel(base)).not.toMatch(/0,00 %/)
  })

  it('states the R15 months and 488 bps when the backend provided them', () => {
    expect(expenseShareLabel({
      ...base,
      expense_share_basis_points: 488,
      average_expense_cents: 110000,
      average_months: ['2026-07', '2026-08'],
    })).toBe('4,88 % priemerných mesačných výdavkov 1 100,00 € (2026-07, 2026-08).')
  })
})

describe('snapshot, empty, exclusion and cash-direction copy', () => {
  it('states that the panel is an as-of snapshot, not a filter sum', () => {
    const caption = snapshotCaption({
      as_of: '2026-03-31',
      history_from: '2026-01-31',
      future_period: false,
      unfinished_period: false,
      rows: [],
      confirmed: {
        monthly_income_cents: 0, monthly_expense_cents: 0, monthly_net_cents: 0,
        annual_income_cents: 0, annual_expense_cents: 0, annual_net_cents: 0,
        remaining_income_cents: 0, remaining_expense_cents: 0,
      },
      estimates: {
        monthly_income_cents: 0, monthly_expense_cents: 0, monthly_net_cents: 0,
        annual_income_cents: 0, annual_expense_cents: 0, annual_net_cents: 0,
        remaining_income_cents: 0, remaining_expense_cents: 0,
      },
      excluded: { missing: 0, ended: 0, unknown: 0 },
      expense_share_basis_points: null,
      average_expense_cents: null,
      average_months: [],
    })
    expect(caption).toContain('snímka plánu')
    expect(caption).toContain('2026-03-31')
    expect(caption).toContain('2026-01-31')
  })

  it('explains the observation minimum and that manual setup starts in a transaction detail', () => {
    expect(emptyPanelExplanation()).toMatch(/tri mesačné/)
    expect(emptyPanelExplanation()).toMatch(/detaile transakcie/)
  })

  it('lists exclusion counts so a zero total is not read as complete knowledge', () => {
    const overview = {
      as_of: '2026-09-07',
      history_from: null,
      future_period: false,
      unfinished_period: true,
      rows: [],
      confirmed: {
        monthly_income_cents: 0, monthly_expense_cents: 0, monthly_net_cents: 0,
        annual_income_cents: 0, annual_expense_cents: 0, annual_net_cents: 0,
        remaining_income_cents: 0, remaining_expense_cents: 0,
      },
      estimates: {
        monthly_income_cents: 0, monthly_expense_cents: 0, monthly_net_cents: 0,
        annual_income_cents: 0, annual_expense_cents: 0, annual_net_cents: 0,
        remaining_income_cents: 0, remaining_expense_cents: 0,
      },
      excluded: { missing: 1, ended: 2, unknown: 3 },
      expense_share_basis_points: null,
      average_expense_cents: null,
      average_months: [],
    }
    expect(exclusionLabel(overview)).toBe('Mimo súčtov: chýbajúce 1, odhad ukončenia 2, neznáme 3.')
  })

  it('explains cash-direction versus category-kind semantics', () => {
    expect(cashDirectionCaption()).toMatch(/skutočný smer peňazí/)
  })
})

describe('cadenceLabel and partitionVisibleRows', () => {
  it('uses the Slovak interval words from the proposal', () => {
    expect(cadenceLabel('monthly')).toBe('mesačne')
    expect(cadenceLabel('quarterly')).toBe('štvrťročne')
    expect(cadenceLabel('yearly')).toBe('ročne')
  })

  it('keeps estimates, confirmed expenses/incomes and ignored in separate lists', () => {
    const estimate = row({ series_key: 'g:est', decision: 'estimate', decision_id: null })
    const expense = row({ series_key: 'g:exp', decision: 'confirmed', direction: 'expense' })
    const income = row({ series_key: 'g:inc', decision: 'confirmed', direction: 'income', name: 'Mzda' })
    const ignored = row({ series_key: 'g:ign', decision: 'ignored', name: 'Obchod' })
    const parts = partitionVisibleRows([estimate, expense, income, ignored])
    expect(parts.estimates.map((r) => r.series_key)).toEqual(['g:est'])
    expect(parts.expenses.map((r) => r.series_key)).toEqual(['g:exp'])
    expect(parts.incomes.map((r) => r.series_key)).toEqual(['g:inc'])
    expect(parts.ignored.map((r) => r.series_key)).toEqual(['g:ign'])
  })
})

describe('isRecurringEligible', () => {
  it('rejects transfer, refund and zero rows and keeps a normal debit', () => {
    expect(isRecurringEligible(tx())).toBe(true)
    expect(isRecurringEligible(tx({ status: 'transfer' }))).toBe(false)
    expect(isRecurringEligible(tx({ kind: 'refund' }))).toBe(false)
    expect(isRecurringEligible(tx({ amount_cents: 0 }))).toBe(false)
  })
})

describe('localTodayIso uses calendar fields, not elapsed milliseconds', () => {
  it('formats a local Date without UTC conversion', () => {
    expect(localTodayIso(new Date(2026, 8, 7, 23, 30, 0))).toBe('2026-09-07')
    expect(localDateFromIso('2026-09-07').getFullYear()).toBe(2026)
    expect(localDateFromIso('2026-09-07').getMonth()).toBe(8)
    expect(localDateFromIso('2026-09-07').getDate()).toBe(7)
  })
})
