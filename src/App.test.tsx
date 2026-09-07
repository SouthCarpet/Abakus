import { act, cleanup, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { App } from './App'

afterEach(() => cleanup())

vi.mock('./api', () => ({
  api: {
    listAccounts: vi.fn().mockResolvedValue([]),
    statementHistory: vi.fn().mockResolvedValue([]),
    badChecksums: vi.fn().mockResolvedValue([]),
    summary: vi.fn().mockResolvedValue({
      income_cents: 0,
      expense_cents: 0,
      transfer_cents: 0,
      net_cents: 0,
      unassigned_count: 0,
      suggested_count: 0,
      by_category: [],
      by_month: [],
      by_month_category: [],
      top_merchants: [],
    }),
    recentStatements: vi.fn().mockResolvedValue([]),
    listCategories: vi.fn().mockResolvedValue([]),
  },
  formatEur: (cents: number) => `${cents}`,
}))

vi.mock('./lib/recurring-api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./lib/recurring-api')>()
  return {
    ...actual,
    recurringApi: {
      overview: vi.fn().mockResolvedValue({
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
        excluded: { missing: 0, ended: 0, unknown: 0 },
        expense_share_basis_points: null,
        average_expense_cents: null,
        average_months: [],
      }),
      detail: vi.fn(),
      transactionContext: vi.fn(),
      save: vi.fn(),
      reset: vi.fn(),
    },
  }
})

// A17: this is the wiring the dark theme block in tokens.css depends on. If
// nothing sets `data-theme`, the dark block is dead code even though it is
// syntactically correct CSS.
describe('App system theme wiring', () => {
  let listeners: Record<string, (event: MediaQueryListEvent) => void>
  let systemPrefersDark: boolean

  beforeEach(() => {
    listeners = {}
    systemPrefersDark = false
    vi.stubGlobal(
      'matchMedia',
      (query: string) =>
        ({
          get matches() {
            return systemPrefersDark
          },
          media: query,
          onchange: null,
          addEventListener: (_: string, cb: (event: MediaQueryListEvent) => void) => {
            listeners[query] = cb
          },
          removeEventListener: (_: string, cb: (event: MediaQueryListEvent) => void) => {
            if (listeners[query] === cb) delete listeners[query]
          },
          addListener: () => {},
          removeListener: () => {},
          dispatchEvent: () => false,
        }) as MediaQueryList,
    )
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    delete document.documentElement.dataset.theme
  })

  it('sets data-theme from the system preference on mount', () => {
    systemPrefersDark = true
    render(<App />)
    expect(document.documentElement.dataset.theme).toBe('dark')
  })

  it('flips data-theme when the system preference changes while the app runs', () => {
    systemPrefersDark = false
    render(<App />)
    expect(document.documentElement.dataset.theme).toBe('light')

    act(() => {
      listeners['(prefers-color-scheme: dark)']?.({ matches: true } as MediaQueryListEvent)
    })

    expect(document.documentElement.dataset.theme).toBe('dark')
  })
})
