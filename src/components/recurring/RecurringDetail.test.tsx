import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { recurringApi } from '../../lib/recurring-api'
import { RecurringDetailDialog } from './RecurringDetail'
import { deferred, recurringRow, txRow } from './oracles'

afterEach(() => cleanup())

vi.mock('../../lib/recurring-api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../lib/recurring-api')>()
  return {
    ...actual,
    recurringApi: {
      overview: vi.fn(),
      detail: vi.fn(),
      transactionContext: vi.fn(),
      save: vi.fn(),
      reset: vi.fn(),
    },
  }
})

const query = { from: '2026-03-01', to: '2026-03-31', account_kind: null, today: '2026-09-07' }
const row = recurringRow({ name: 'Netflix' })
const periodMember = txRow({ id: 21, tx_date: '2026-03-15', amount_cents: -1200, category_name: 'Netflix' })
const historyMember = txRow({ id: 20, tx_date: '2026-01-15', amount_cents: -1200, category_name: 'Netflix' })
const otherAccount = txRow({ id: 99, account_id: 2, account_kind: 'business', merchant_raw: 'Netflix iné', tx_date: '2026-02-01' })

beforeEach(() => {
  vi.mocked(recurringApi.detail).mockReset().mockImplementation(async (request) => {
    const full = request.query.from === null
    return {
      row,
      transactions: full ? [historyMember, periodMember] : [periodMember],
      matching_transaction_ids: [20, 21],
      compatible_transactions: full ? [otherAccount] : [],
    }
  })
})

describe('RecurringDetailDialog exact membership and period switch', () => {
  it('lists only the selected-period members until the user asks for full history', async () => {
    render(<RecurringDetailDialog seriesKey="g:netflix" query={query} onClose={() => {}} onEdit={() => {}} />)
    expect(await screen.findByText('15. 3. 2026')).toBeInTheDocument()
    expect(screen.queryByText('15. 1. 2026')).not.toBeInTheDocument()
    expect(screen.getByText(/Vybrané obdobie 2026-03-01 až 2026-03-31/)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Celá známa história' }))
    expect(await screen.findByText('15. 1. 2026')).toBeInTheDocument()
    expect(screen.getByText('15. 3. 2026')).toBeInTheDocument()
    expect(screen.getByText(/Celá známa história do 2026-09-07/)).toBeInTheDocument()
    expect(vi.mocked(recurringApi.detail).mock.calls.at(-1)?.[0]).toEqual({
      series_key: 'g:netflix',
      query: { from: null, to: null, account_kind: null, today: '2026-09-07' },
    })
  })

  it('does not treat a same-name other-account row as a period member', async () => {
    render(<RecurringDetailDialog seriesKey="g:netflix" query={query} onClose={() => {}} onEdit={() => {}} />)
    await screen.findByText('15. 3. 2026')
    expect(screen.queryByText('Netflix iné')).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Celá známa história' }))
    expect(await screen.findByText('Netflix iné')).toBeInTheDocument()
    expect(screen.getByText('nie')).toBeInTheDocument()
  })

  it('drops a stale full-history response after the user switches back to the selected period', async () => {
    const period = deferred<{ row: typeof row; transactions: typeof periodMember[]; matching_transaction_ids: number[]; compatible_transactions: typeof otherAccount[] }>()
    const history = deferred<{ row: typeof row; transactions: typeof periodMember[]; matching_transaction_ids: number[]; compatible_transactions: typeof otherAccount[] }>()
    vi.mocked(recurringApi.detail).mockReset()
      .mockReturnValueOnce(period.promise)
      .mockReturnValueOnce(history.promise)
      .mockResolvedValue({
        row,
        transactions: [periodMember],
        matching_transaction_ids: [21],
        compatible_transactions: [],
      })
    render(<RecurringDetailDialog seriesKey="g:netflix" query={query} onClose={() => {}} onEdit={() => {}} />)
    fireEvent.click(await screen.findByRole('button', { name: 'Celá známa história' }))
    fireEvent.click(screen.getByRole('button', { name: 'Vybrané obdobie' }))
    history.resolve({
      row,
      transactions: [historyMember],
      matching_transaction_ids: [20, 21],
      compatible_transactions: [],
    })
    period.resolve({
      row,
      transactions: [periodMember],
      matching_transaction_ids: [21],
      compatible_transactions: [],
    })
    await waitFor(() => expect(screen.getByText('15. 3. 2026')).toBeInTheDocument())
    expect(screen.queryByText('15. 1. 2026')).not.toBeInTheDocument()
  })
})
