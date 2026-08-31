import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { BadChecksum } from '../api'
import { ChecksumBanner, Overview } from './Overview'

afterEach(() => cleanup())

vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return {
    ...actual,
    api: {
      listAccounts: vi.fn().mockResolvedValue([]),
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
