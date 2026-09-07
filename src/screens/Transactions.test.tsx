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
      saveTransactionNote: vi.fn().mockResolvedValue(undefined),
    },
  }
})

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: unknown) => void
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej })
  return { promise, resolve, reject }
}

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
          <TransactionRow row={row} categories={[]} selected={false} onSelect={vi.fn()} onAssign={vi.fn()} onConfirm={onConfirm} onNoteSaved={vi.fn()} onCreateCategory={vi.fn()} />
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
          <TransactionRow row={row} categories={[]} selected={false} onSelect={vi.fn()} onAssign={vi.fn()} onConfirm={vi.fn()} onNoteSaved={vi.fn()} onCreateCategory={vi.fn()} />
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
            onCreateCategory={vi.fn()}
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

const DEFAULT_ROWS = [
  {
    id: 5, account_id: 1, account_kind: 'personal', statement_number: 1, posted_date: '2026-05-01', tx_date: '2026-05-01',
    kind: 'card', amount_cents: -500, orig_amount_cents: null, orig_currency: null, merchant_raw: 'Obchod', place: null,
    counterparty_name: null, counterparty_iban: null, category_id: null, category_name: null, parent_name: null,
    status: 'unassigned', source: 'pdf', raw_block: '', note: '',
  },
] as never

describe('Transactions integration: a note save settling after the filter changed', () => {
  afterEach(() => {
    vi.mocked(api.listAccounts).mockReset().mockResolvedValue([])
    vi.mocked(api.listTransactions).mockReset().mockResolvedValue(DEFAULT_ROWS)
    vi.mocked(api.saveTransactionNote).mockClear()
  })

  // Oracle/controller finding: onNoteSaved used to be the plain `fetchRows`
  // closure, captured by NoteEditor at click time. If the account/date/text
  // filter changes while that save's write is still settling, the stale
  // closure's refetch (querying the OLD filter) can resolve last and
  // overwrite the current, correctly-filtered rows.
  it('never lets a save that settles after an account filter change refetch with the old filter', async () => {
    vi.mocked(api.listAccounts).mockResolvedValueOnce([
      { id: 1, iban: 'SK4411000000000012345678', kind: 'personal', label: 'Osobný', has_password: false },
      { id: 2, iban: 'SK3711000000000098765432', kind: 'business', label: 'Firemný', has_password: false },
    ])
    vi.mocked(api.listTransactions).mockImplementation(async (filter) =>
      (filter as { account_id: number | null }).account_id === 2 ? [] : DEFAULT_ROWS,
    )
    const save = deferred<void>()
    vi.mocked(api.saveTransactionNote).mockReturnValueOnce(save.promise)

    render(<Transactions />)
    await screen.findByText('Obchod')
    fireEvent.click(screen.getByRole('button', { name: 'Detail transakcie 5' }))
    fireEvent.change(screen.getByLabelText('Poznámka k transakcii 5'), { target: { value: 'nova poznamka' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    await waitFor(() => expect(api.saveTransactionNote).toHaveBeenCalledTimes(1))

    fireEvent.change(screen.getByRole('combobox', { name: 'Účet' }), { target: { value: '2' } })
    await waitFor(() => expect(screen.queryByText('Obchod')).not.toBeInTheDocument())
    const callsBeforeResolve = vi.mocked(api.listTransactions).mock.calls.length

    save.resolve()
    await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.length).toBeGreaterThan(callsBeforeResolve))

    // The settling save's own refetch (triggered just now, after resolve)
    // must still have queried the current filter, not the stale one it was
    // originally captured with.
    expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ account_id: 2 }))
    expect(screen.queryByText('Obchod')).not.toBeInTheDocument()
  })

  // Oracle/controller finding: a note save can make the row it belongs to
  // stop matching the active text filter; the row then disappears from the
  // table but must also disappear from the bulk selection, or a hidden row
  // could still receive a bulk category assignment.
  it('drops a row from the bulk selection once a note save makes it stop matching the text filter', async () => {
    let call = 0
    vi.mocked(api.listTransactions).mockImplementation(async () => {
      call += 1
      return call === 1 ? DEFAULT_ROWS : []
    })
    render(<Transactions />)
    await screen.findByText('Obchod')
    fireEvent.click(screen.getByRole('checkbox', { name: /Vybrať transakciu 5/ }))
    expect(screen.getByText('1 vybraných')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Detail transakcie 5' }))
    fireEvent.change(screen.getByLabelText('Poznámka k transakcii 5'), { target: { value: 'uz nesedi filtru' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    await waitFor(() => expect(screen.queryByText('Obchod')).not.toBeInTheDocument())

    expect(screen.queryByText('1 vybraných')).not.toBeInTheDocument()
  })
})

describe('Transactions integration: a successful note write with a failed refresh', () => {
  afterEach(() => {
    vi.mocked(api.listTransactions).mockReset().mockResolvedValue(DEFAULT_ROWS)
    vi.mocked(api.saveTransactionNote).mockClear()
  })

  // Controller finding: verify the REAL integrated tree, not NoteEditor alone
  // with a manually rejected callback. The row (and the message inside it)
  // must stay visible: a failed refresh must never read as if the row, or
  // the fact the note itself did save, had disappeared.
  it('keeps the row and the saved-but-refresh-failed message visible instead of hiding them', async () => {
    vi.mocked(api.listTransactions).mockResolvedValueOnce(DEFAULT_ROWS).mockRejectedValueOnce(new Error('siet nedostupna'))
    render(<Transactions />)
    await screen.findByText('Obchod')
    fireEvent.click(screen.getByRole('button', { name: 'Detail transakcie 5' }))
    fireEvent.change(screen.getByLabelText('Poznámka k transakcii 5'), { target: { value: 'ulozena poznamka' } })

    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))

    expect(await screen.findByText(/obnovenie zoznamu zlyhalo/)).toBeInTheDocument()
    expect(screen.getByText('Obchod')).toBeInTheDocument()
    expect(screen.getByLabelText('Poznámka k transakcii 5')).toBeInTheDocument()
  })

  // The note write itself already succeeded; retrying must not repeat it.
  it('does not repeat the already-successful write when the user retries after a refresh failure', async () => {
    vi.mocked(api.listTransactions)
      .mockResolvedValueOnce(DEFAULT_ROWS)
      .mockRejectedValueOnce(new Error('siet nedostupna'))
      .mockResolvedValueOnce(DEFAULT_ROWS)
    render(<Transactions />)
    await screen.findByText('Obchod')
    fireEvent.click(screen.getByRole('button', { name: 'Detail transakcie 5' }))
    fireEvent.change(screen.getByLabelText('Poznámka k transakcii 5'), { target: { value: 'ulozena poznamka' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    await screen.findByText(/obnovenie zoznamu zlyhalo/)
    expect(api.saveTransactionNote).toHaveBeenCalledTimes(1)

    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.length).toBeGreaterThanOrEqual(3))

    expect(api.saveTransactionNote).toHaveBeenCalledTimes(1)
  })
})
