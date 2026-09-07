import { cleanup, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Account, StatementHistoryRow, Summary } from '../../api'
import { api } from '../../api'
import { InsightsPanel } from './InsightsPanel'

afterEach(() => cleanup())

// mockResolvedValue/mockRejectedValue persist across tests in this file (no
// clearMocks configured); pin every fetch back to a harmless default before
// each test, so one test's failure or override never leaks into the next.
beforeEach(() => {
  vi.mocked(api.listAccounts).mockReset().mockResolvedValue([])
  vi.mocked(api.statementHistory).mockReset().mockResolvedValue([])
  vi.mocked(api.summary).mockReset()
})

vi.mock('../../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../api')>()
  return {
    ...actual,
    api: {
      listAccounts: vi.fn().mockResolvedValue([]),
      statementHistory: vi.fn().mockResolvedValue([]),
      summary: vi.fn(),
    },
  }
})

const account = (overrides: Partial<Account> = {}): Account => ({
  id: 1, iban: 'SK4411000000000012345678', kind: 'personal', label: 'Osobný', has_password: false, ...overrides,
})

const statement = (overrides: Partial<StatementHistoryRow> = {}): StatementHistoryRow => ({
  statement_id: 1, account_id: 1, account_label: 'Osobný', account_kind: 'personal', number: 3,
  period_start: '2026-06-01', period_end: '2026-06-30', opening_cents: 10000, closing_cents: 12345,
  checksum: { status: 'ok' }, ...overrides,
})

const emptySummary: Summary = {
  income_cents: 0, expense_cents: 0, transfer_cents: 0, net_cents: 0, unassigned_count: 0, suggested_count: 0,
  by_category: [], by_month: [], by_month_category: [], top_merchants: [],
}

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: unknown) => void
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej })
  return { promise, resolve, reject }
}

describe('InsightsPanel stays visible when the transaction summary is empty', () => {
  it('renders coverage and balance content even though summary.by_month is empty', async () => {
    vi.mocked(api.listAccounts).mockResolvedValue([account()])
    vi.mocked(api.statementHistory).mockResolvedValue([statement()])
    render(<InsightsPanel period={{ kind: 'all' }} accountKind="all" summary={{ ...emptySummary, by_month: [] }} />)
    expect(await screen.findByText('Pokrytie výpismi')).toBeInTheDocument()
    expect(await screen.findByText('Zostatky z výpisov')).toBeInTheDocument()
  })

  it('renders coverage and balance content even before the summary has loaded (summary is still null)', async () => {
    vi.mocked(api.listAccounts).mockResolvedValue([account()])
    vi.mocked(api.statementHistory).mockResolvedValue([statement()])
    render(<InsightsPanel period={{ kind: 'all' }} accountKind="all" summary={null} />)
    expect(await screen.findByText('Pokrytie výpismi')).toBeInTheDocument()
  })
})

describe('InsightsPanel statement-history failure', () => {
  it('shows a distinct error instead of a false-empty coverage table', async () => {
    vi.mocked(api.listAccounts).mockResolvedValue([account()])
    vi.mocked(api.statementHistory).mockRejectedValue(new Error('offline'))
    render(<InsightsPanel period={{ kind: 'all' }} accountKind="all" summary={emptySummary} />)
    expect(await screen.findByRole('alert')).toHaveTextContent('offline')
    expect(screen.queryByText('Pokrytie výpismi')).not.toBeInTheDocument()
    expect(screen.queryByText('Zostatky z výpisov')).not.toBeInTheDocument()
  })
})

describe('InsightsPanel stale statement-history responses', () => {
  it('never lets a slow response for an old account-kind selection overwrite a newer selection', async () => {
    const first = deferred<StatementHistoryRow[]>()
    const second = deferred<StatementHistoryRow[]>()
    vi.mocked(api.listAccounts).mockResolvedValue([])
    vi.mocked(api.statementHistory).mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise)

    const { rerender } = render(<InsightsPanel period={{ kind: 'all' }} accountKind="all" summary={emptySummary} />)
    rerender(<InsightsPanel period={{ kind: 'all' }} accountKind="personal" summary={emptySummary} />)

    second.resolve([statement({ statement_id: 10, account_label: 'DRUHA-OK', number: 5 })])
    await screen.findByText('DRUHA-OK')

    first.resolve([statement({ statement_id: 20, account_label: 'PRVA-STALE', number: 99 })])
    await waitFor(() => expect(screen.queryByText('PRVA-STALE')).not.toBeInTheDocument())
  })
})

describe('InsightsPanel category comparison', () => {
  it('has no comparison for Všetko, and never fetches a previous-range summary for it', async () => {
    render(<InsightsPanel period={{ kind: 'all' }} accountKind="all" summary={emptySummary} />)
    await waitFor(() => expect(api.statementHistory).toHaveBeenCalled())
    expect(screen.queryByText('Porovnanie výdavkov podľa kategórií')).not.toBeInTheDocument()
    expect(api.summary).not.toHaveBeenCalled()
  })

  it('shows a distinct error instead of a false zero when the previous-range summary fetch fails', async () => {
    vi.mocked(api.summary).mockRejectedValue(new Error('previous range failed'))
    render(
      <InsightsPanel
        period={{ kind: 'custom', custom: { from: '2026-06-01', to: '2026-06-30' } }}
        accountKind="all"
        summary={{ ...emptySummary, by_category: [{ category_id: 1, name: 'Jedlo', parent_name: null, cents: -1000 }], expense_cents: -1000 }}
      />,
    )
    expect(await screen.findByRole('alert')).toHaveTextContent('previous range failed')
    expect(screen.queryByText('Porovnanie výdavkov podľa kategórií')).not.toBeInTheDocument()
  })

  it('renders the current-vs-previous category table and marks a zero previous total as an unavailable percent, never 0% or Infinity', async () => {
    vi.mocked(api.summary).mockResolvedValue({ ...emptySummary, by_category: [], expense_cents: 0 })
    render(
      <InsightsPanel
        period={{ kind: 'custom', custom: { from: '2026-06-01', to: '2026-06-30' } }}
        accountKind="all"
        summary={{ ...emptySummary, by_category: [{ category_id: 1, name: 'Jedlo', parent_name: null, cents: -1000 }], expense_cents: -1000 }}
      />,
    )
    expect(await screen.findByText('Jedlo')).toBeInTheDocument()
    expect(screen.getAllByText('nedostupné').length).toBeGreaterThan(0)
  })

  it('fetches the previous-range summary with the same account-kind scope as the current selection', async () => {
    vi.mocked(api.summary).mockResolvedValue({ ...emptySummary, by_category: [], expense_cents: 0 })
    render(
      <InsightsPanel
        period={{ kind: 'custom', custom: { from: '2026-06-01', to: '2026-06-30' } }}
        accountKind="business"
        summary={{ ...emptySummary, by_category: [], expense_cents: 0 }}
      />,
    )
    await waitFor(() => expect(api.summary).toHaveBeenCalledWith('2026-05-02', '2026-05-31', null, 'business'))
  })
})
