import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Summary } from '../../api'
import { api } from '../../api'
import { YearComparisonCard } from './YearComparisonCard'

afterEach(() => cleanup())

beforeEach(() => {
  vi.mocked(api.summary).mockReset()
})

vi.mock('../../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../api')>()
  return {
    ...actual,
    api: { summary: vi.fn() },
  }
})

const summary = (overrides: Partial<Summary> = {}): Summary => ({
  income_cents: 0,
  expense_cents: 0,
  transfer_cents: 0,
  fee_cents: 0,
  net_cents: 0,
  unassigned_count: 0,
  suggested_count: 0,
  by_category: [],
  by_month: [],
  by_month_category: [],
  top_merchants: [],
  ...overrides,
})

describe('YearComparisonCard', () => {
  it('defaults to a one-month comparison and shows both compared ranges next to the numbers', async () => {
    vi.mocked(api.summary)
      .mockResolvedValueOnce(summary({ income_cents: 100000, expense_cents: 60000 }))
      .mockResolvedValueOnce(summary({ income_cents: 90000, expense_cents: 50000 }))

    render(<YearComparisonCard accountKind={null} today="2026-06-15" />)

    await waitFor(() => expect(api.summary).toHaveBeenCalledWith('2026-06-01', '2026-06-30', null, null))
    expect(api.summary).toHaveBeenCalledWith('2025-06-01', '2025-06-30', null, null)
    expect(await screen.findByText('1. 6. 2026–30. 6. 2026')).toBeInTheDocument()
    expect(screen.getByText('1. 6. 2025–30. 6. 2025')).toBeInTheDocument()
    expect(screen.getByText('1 000,00 €')).toBeInTheDocument()
    expect(screen.getByText('900,00 €')).toBeInTheDocument()
  })

  it('switches to a three-month window ending in the selected month on choice, refetching both ranges', async () => {
    vi.mocked(api.summary).mockResolvedValue(summary())
    render(<YearComparisonCard accountKind={null} today="2026-06-15" />)
    await waitFor(() => expect(api.summary).toHaveBeenCalledWith('2026-06-01', '2026-06-30', null, null))

    fireEvent.click(screen.getByRole('button', { name: 'Tri mesiace' }))

    await waitFor(() => expect(api.summary).toHaveBeenCalledWith('2026-04-01', '2026-06-30', null, null))
    expect(api.summary).toHaveBeenCalledWith('2025-04-01', '2025-06-30', null, null)
    expect(await screen.findByText('1. 4. 2026–30. 6. 2026')).toBeInTheDocument()
    expect(screen.getByText('1. 4. 2025–30. 6. 2025')).toBeInTheDocument()
  })

  it('passes the selected account kind through to both summary fetches', async () => {
    vi.mocked(api.summary).mockResolvedValue(summary())
    render(<YearComparisonCard accountKind="business" today="2026-06-15" />)
    await waitFor(() => expect(api.summary).toHaveBeenCalledWith('2026-06-01', '2026-06-30', null, 'business'))
    expect(api.summary).toHaveBeenCalledWith('2025-06-01', '2025-06-30', null, 'business')
  })
})
