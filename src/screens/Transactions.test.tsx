import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { api } from '../api'
import { Transactions, TransactionRow } from './Transactions'

afterEach(() => cleanup())

vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return {
    ...actual,
    api: {
      listAccounts: vi.fn().mockResolvedValue([]),
      listCategories: vi.fn().mockResolvedValue([
        { id: 1, parent_id: null, name: 'Jedlo', kind: 'expense', sort: 0, system: false, archived: false },
      ]),
      listTransactions: vi.fn().mockResolvedValue([
        {
          id: 5,
          account_id: 1,
          account_kind: 'personal',
          statement_number: 1,
          posted_date: '2026-05-01',
          tx_date: '2026-05-01',
          kind: 'card',
          amount_cents: -500,
          orig_amount_cents: null,
          orig_currency: null,
          merchant_raw: 'Obchod',
          place: null,
          counterparty_name: null,
          counterparty_iban: null,
          category_id: null,
          category_name: null,
          parent_name: null,
          status: 'unassigned',
          source: 'pdf',
          raw_block: '',
        },
      ]),
      assign: vi.fn().mockResolvedValue({ updated: 1, rules_created: 0, skipped_transfers: 2 }),
    },
  }
})

const row = {
  id: 7,
  account_kind: 'personal',
  tx_date: '2026-05-29',
  merchant_raw: 'ALDI SUED',
  place: 'Neuss',
  amount_cents: -199,
  category_id: 2,
  category_name: 'potraviny',
  parent_name: 'Jedlo',
  status: 'suggested',
  raw_block: 'raw',
  kind: 'card',
} as never

describe('TransactionRow', () => {
  it('shows an Odhad badge with a confirm button and calls confirm', () => {
    const onConfirm = vi.fn()
    render(
      <table>
        <tbody>
          <TransactionRow row={row} categories={[]} selected={false} onSelect={vi.fn()} onAssign={vi.fn()} onConfirm={onConfirm} />
        </tbody>
      </table>,
    )
    expect(screen.getByText('Odhad')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Potvrdiť' }))
    expect(onConfirm).toHaveBeenCalledWith(7)
  })
  it('renders the amount with tabular numerals class and negative sign', () => {
    render(
      <table>
        <tbody>
          <TransactionRow row={row} categories={[]} selected={false} onSelect={vi.fn()} onAssign={vi.fn()} onConfirm={vi.fn()} />
        </tbody>
      </table>,
    )
    expect(screen.getByText('-1,99 €')).toHaveClass('k-num')
  })
  it('disables the row checkbox for a transfer row, mirroring the category picker', () => {
    render(
      <table>
        <tbody>
          <TransactionRow
            row={{ ...(row as Record<string, unknown>), status: 'transfer' } as never}
            categories={[]}
            selected={false}
            onSelect={vi.fn()}
            onAssign={vi.fn()}
            onConfirm={vi.fn()}
          />
        </tbody>
      </table>,
    )
    expect(screen.getByRole('checkbox')).toBeDisabled()
  })
})

describe('Transactions bulk assign toast', () => {
  it('shows a toast when bulkAssign skips transfers', async () => {
    render(<Transactions />)
    await waitFor(() => expect(screen.getByText('Obchod')).toBeInTheDocument())

    fireEvent.click(screen.getAllByRole('checkbox')[0])
    const bulkBar = screen.getByText('1 vybraných').closest('div') as HTMLElement
    fireEvent.change(within(bulkBar).getByRole('combobox'), { target: { value: '1' } })
    fireEvent.click(within(bulkBar).getByRole('button', { name: 'Priradiť' }))

    await waitFor(() => expect(vi.mocked(api.assign)).toHaveBeenCalled())
    expect(await screen.findByText('Prevody sa nepriraďujú, preskočené: 2')).toBeInTheDocument()
  })
})
