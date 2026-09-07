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
          note: '',
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
  note: '',
} as never

describe('TransactionRow', () => {
  it('shows an Odhad badge with a confirm button and calls confirm', () => {
    const onConfirm = vi.fn()
    render(
      <table>
        <tbody>
          <TransactionRow row={row} categories={[]} selected={false} onSelect={vi.fn()} onAssign={vi.fn()} onConfirm={onConfirm} onNoteSaved={vi.fn()} />
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
          <TransactionRow row={row} categories={[]} selected={false} onSelect={vi.fn()} onAssign={vi.fn()} onConfirm={vi.fn()} onNoteSaved={vi.fn()} />
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
            onNoteSaved={vi.fn()}
          />
        </tbody>
      </table>,
    )
    expect(screen.getByRole('checkbox')).toBeDisabled()
  })
})

// A17: an empty filter used to show only the table headers with no next-action.
describe('Transactions empty state', () => {
  it('names the period as the likely reason once loading finishes with no rows', async () => {
    vi.mocked(api.listTransactions).mockResolvedValueOnce([])
    render(<Transactions />)
    expect(await screen.findByText('Za toto obdobie nič nie je. Skús iné obdobie hore.')).toBeInTheDocument()
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

// Oracle: parent-filter brief and independent native finding; metadata loading must not mislabel a retained parent constraint.
it('retains parent drilldown 10 through asynchronous metadata and queries parent then child with the personal account constraint', async () => {
  const categories = [
    { id: 10, parent_id: null, name: 'Jedlo', kind: 'expense' as const, sort: 0, system: false, archived: false },
    { id: 11, parent_id: 10, name: 'Potraviny', kind: 'expense' as const, sort: 1, system: false, archived: false },
  ]
  let resolveCategories!: (value: typeof categories) => void
  vi.mocked(api.listCategories).mockReturnValueOnce(new Promise((resolve) => { resolveCategories = resolve }))
  render(<Transactions initialCategoryId={10} initialAccountKind="personal" />)
  const filter = screen.getByRole('combobox', { name: 'Filter kategórie' })
  expect(filter).toHaveValue('10')
  expect(filter).toHaveDisplayValue('Kategória ID 10 (názov nie je dostupný)')
  await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ category_id: 10, account_kind: 'personal' })))

  resolveCategories(categories)
  await waitFor(() => expect(filter).toHaveDisplayValue('Jedlo'))
  expect(filter).toHaveValue('10')
  expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ category_id: 10, account_kind: 'personal' }))

  fireEvent.change(filter, { target: { value: '11' } })
  await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ category_id: 11, account_kind: 'personal' })))
  expect(filter).toHaveDisplayValue('Potraviny')
})
