import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { BadChecksum, Summary } from '../api'
import { api } from '../api'
import { ChecksumBanner, Overview } from './Overview'

afterEach(() => cleanup())

vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return {
    ...actual,
    api: {
      badChecksums: vi.fn().mockResolvedValue([{ statement_id: 9, number: 4, account_label: 'Osobný', off_by_cents: -150 }]),
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
      // InsightsPanel (080-insights) fetches these independently of summary;
      // an empty resolve keeps its cards from rendering in tests that do not
      // exercise them directly.
      listAccounts: vi.fn().mockResolvedValue([]),
      statementHistory: vi.fn().mockResolvedValue([]),
      listCategories: vi.fn().mockResolvedValue([]),
    },
  }
})

vi.mock('../lib/recurring-api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/recurring-api')>()
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

describe('ChecksumBanner', () => {
  it('renders the mismatch message and navigates to the statement on click', () => {
    const row: BadChecksum = { statement_id: 9, number: 4, account_label: 'Osobný', off_by_cents: -150 }
    const onNavigate = vi.fn()
    render(<ChecksumBanner row={row} onNavigate={onNavigate} />)
    expect(screen.getByText('Výpis č. 4 (účet Osobný) nesedí o 1,50 €')).toBeInTheDocument()
    screen.getByRole('button', { name: 'Zobraziť transakcie' }).click()
    expect(onNavigate).toHaveBeenCalledWith(9)
  })
})

describe('Overview', () => {
  it('renders a danger banner from the mounted screen once badChecksums resolves', async () => {
    render(<Overview onNavigateToImport={() => {}} onNavigateToTransactions={() => {}} />)
    const banner = await screen.findByText('Výpis č. 4 (účet Osobný) nesedí o 1,50 €')
    expect(banner).toHaveClass('k-text-danger')
  })
})

describe('Overview account kind filter (A17/F3)', () => {
  it('sends the whole account kind, not one account id, so a second account of that kind is never dropped', async () => {
    render(<Overview onNavigateToImport={() => {}} onNavigateToTransactions={() => {}} />)
    await waitFor(() => expect(api.summary).toHaveBeenCalled())
    expect(vi.mocked(api.summary).mock.calls[0].slice(2)).toEqual([null, null])

    fireEvent.click(screen.getByRole('button', { name: 'Osobný' }))
    await waitFor(() => expect(vi.mocked(api.summary).mock.calls.at(-1)?.slice(2)).toEqual([null, 'personal']))
  })
})

const nonEmptySummary: Summary = {
  income_cents: 0,
  expense_cents: 0,
  transfer_cents: 0,
  net_cents: 0,
  unassigned_count: 3,
  suggested_count: 1,
  by_category: [],
  by_month: [{ month: '2026-06', income_cents: 0, expense_cents: 0 }],
  by_month_category: [],
  top_merchants: [],
}

// A17: the unassigned drill-down must carry the account kind the user
// already filtered by, or it can show rows from an account they excluded.
describe('Overview unassigned drill-down carries the account kind filter', () => {
  it('adds the selected account kind to the navigation entry', async () => {
    vi.mocked(api.summary).mockResolvedValue(nonEmptySummary)
    const onNavigateToTransactions = vi.fn()
    render(<Overview onNavigateToImport={() => {}} onNavigateToTransactions={onNavigateToTransactions} />)
    fireEvent.click(await screen.findByRole('button', { name: 'Osobný' }))
    await waitFor(() => expect(vi.mocked(api.summary).mock.calls.at(-1)?.slice(2)).toEqual([null, 'personal']))

    fireEvent.click(await screen.findByRole('button', { name: /Nezaradené/ }))

    expect(onNavigateToTransactions).toHaveBeenCalledWith({ status: 'unassigned', accountKind: 'personal' })
  })

  it('leaves the entry untouched while the filter is Všetko', async () => {
    vi.mocked(api.summary).mockResolvedValue(nonEmptySummary)
    const onNavigateToTransactions = vi.fn()
    render(<Overview onNavigateToImport={() => {}} onNavigateToTransactions={onNavigateToTransactions} />)

    fireEvent.click(await screen.findByRole('button', { name: /Nezaradené/ }))

    expect(onNavigateToTransactions).toHaveBeenCalledWith({ status: 'unassigned' })
  })
})

const emptySummary: Summary = { ...nonEmptySummary, unassigned_count: 0, suggested_count: 0, by_month: [] }

// A17: "Importuj prvý výpis" told a user with two imports on Import to
// import a first statement, because the empty card never checked whether
// any import existed at all, only the current period.
describe('Overview empty state names the real cause', () => {
  // An earlier describe block leaves api.summary resolving nonEmptySummary
  // (mockResolvedValue with no "Once" persists); pin it back to the empty
  // shape these tests need, since mocks are not reset between test files.
  beforeEach(() => {
    vi.mocked(api.summary).mockResolvedValue(emptySummary)
  })

  it('tells the user to import a first statement when no import exists yet', async () => {
    vi.mocked(api.recentStatements).mockResolvedValueOnce([])
    render(<Overview onNavigateToImport={() => {}} onNavigateToTransactions={() => {}} />)
    expect(await screen.findByText('Zatiaľ nič. Importuj prvý výpis.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Importovať' })).toBeInTheDocument()
  })

  it('points at the period chips when imports exist but the period is empty', async () => {
    vi.mocked(api.recentStatements).mockResolvedValueOnce([
      { statement_id: 5, number: 6, period_end: '2026-06-30', account_label: 'Osobný', transaction_count: 8, checksum: { status: 'ok' } },
    ])
    render(<Overview onNavigateToImport={() => {}} onNavigateToTransactions={() => {}} />)
    expect(await screen.findByText(/Za vybrané obdobie nič nie je/)).toBeInTheDocument()
    expect(screen.queryByText('Zatiaľ nič. Importuj prvý výpis.')).not.toBeInTheDocument()
  })
})

// Oracle: visual-corrections defect 3 preserves category drilldown and the existing account-kind filter.
it('navigates from a negative category to transactions with the selected personal account kind', async () => {
  vi.mocked(api.summary).mockResolvedValue({
    ...nonEmptySummary,
    by_month_category: [{ month: '2026-06', category_id: 42, name: 'Jedlo', cents: -5001 }],
  })
  const onNavigateToTransactions = vi.fn()
  render(<Overview onNavigateToImport={() => {}} onNavigateToTransactions={onNavigateToTransactions} />)
  fireEvent.click(screen.getByRole('button', { name: 'Osobný' }))
  fireEvent.click(await screen.findByRole('button', { name: 'Jedlo · -50,01 €' }))
  expect(onNavigateToTransactions).toHaveBeenCalledWith({ categoryId: 42, accountKind: 'personal' })
})

// 080-insights: coverage and balance history must stay visible under the
// EmptyOverview card, not disappear along with the rest of the transaction
// summary. Deep insight behaviors (gaps, comparison, async independence) are
// covered in InsightsPanel.test.tsx; this only checks Overview wires it in
// and does not gate it behind the summary/hasAnyImports state.
describe('Overview keeps insights visible when the transaction summary is empty', () => {
  it('renders statement coverage below the empty-overview card', async () => {
    vi.mocked(api.summary).mockResolvedValue(emptySummary)
    vi.mocked(api.recentStatements).mockResolvedValueOnce([])
    vi.mocked(api.listAccounts).mockResolvedValueOnce([{ id: 1, iban: 'SK00', kind: 'personal', label: 'Osobný', has_password: false }])
    vi.mocked(api.statementHistory).mockResolvedValueOnce([
      { statement_id: 1, account_id: 1, account_label: 'Osobný', account_kind: 'personal', number: 3, period_start: '2026-06-01', period_end: '2026-06-30', opening_cents: 1000, closing_cents: 2000, checksum: { status: 'ok' } },
    ])
    render(<Overview onNavigateToImport={() => {}} onNavigateToTransactions={() => {}} />)
    expect(await screen.findByText('Zatiaľ nič. Importuj prvý výpis.')).toBeInTheDocument()
    expect(await screen.findByText('Pokrytie výpismi')).toBeInTheDocument()
  })
})

describe('Overview keeps the recurring panel visible when the transaction summary is empty', () => {
  it('still mounts Pravidelné platby under the empty-overview card', async () => {
    vi.mocked(api.summary).mockResolvedValue(emptySummary)
    vi.mocked(api.recentStatements).mockResolvedValueOnce([])
    render(<Overview onNavigateToImport={() => {}} onNavigateToTransactions={() => {}} />)
    expect(await screen.findByText('Zatiaľ nič. Importuj prvý výpis.')).toBeInTheDocument()
    expect(await screen.findByText('Pravidelné platby')).toBeInTheDocument()
    expect(await screen.findByText(/tri mesačné/)).toBeInTheDocument()
  })
})
