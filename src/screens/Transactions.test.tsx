import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { api, type TxRow } from '../api'
import { files } from '../lib/files'
import { Transactions, TransactionRow } from './Transactions'

function makeTxRow(overrides: Partial<TxRow> & { id: number }): TxRow {
  return {
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
    ...overrides,
  }
}

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
      assign: vi.fn().mockResolvedValue({ updated: 1, rules_created: 0, skipped_transfers: 0 }),
      bulkAssign: vi.fn().mockResolvedValue({ updated: 1, rules_created: 0, skipped_transfers: 2, undo_id: null }),
      undoLastAssignment: vi.fn().mockResolvedValue(null),
      confirm: vi.fn().mockResolvedValue({ updated: 0, rules_created: 0, skipped_transfers: 0 }),
      saveTransactionNote: vi.fn().mockResolvedValue(undefined),
      saveCategory: vi.fn(),
      exportCsv: vi.fn(),
    },
  }
})
vi.mock('../lib/files', () => ({ files: { saveCsv: vi.fn() } }))

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
    expect(onConfirm).toHaveBeenCalledWith(7, false)
  })

  // Point 1: confirming also matching payments. The confirm button never
  // touches other rows unless the user explicitly opts in via this checkbox.
  it('offers confirming matching unconfirmed rows too, and passes that choice on confirm', () => {
    const onConfirm = vi.fn()
    render(
      <table>
        <tbody>
          <TransactionRow row={row} categories={[]} selected={false} matchingCount={2} onSelect={vi.fn()} onAssign={vi.fn()} onConfirm={onConfirm} onNoteSaved={vi.fn()} onCreateCategory={vi.fn()} />
        </tbody>
      </table>,
    )
    const checkbox = screen.getByRole('checkbox', { name: 'Potvrdiť aj podobné (2)' })
    fireEvent.click(checkbox)
    fireEvent.click(screen.getByRole('button', { name: 'Potvrdiť' }))
    expect(onConfirm).toHaveBeenCalledWith(7, true)
  })

  it('shows no matching-rows option when nothing else matches the merchant', () => {
    render(
      <table>
        <tbody>
          <TransactionRow row={row} categories={[]} selected={false} matchingCount={0} onSelect={vi.fn()} onAssign={vi.fn()} onConfirm={vi.fn()} onNoteSaved={vi.fn()} onCreateCategory={vi.fn()} />
        </tbody>
      </table>,
    )
    expect(screen.queryByText(/Potvrdiť aj podobné/)).not.toBeInTheDocument()
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

  it('mounts the recurring action after the note editor in expanded detail', () => {
    render(
      <table>
        <tbody>
          <TransactionRow row={row} categories={[]} selected={false} onSelect={vi.fn()} onAssign={vi.fn()} onConfirm={vi.fn()} onNoteSaved={vi.fn()} onCreateCategory={vi.fn()} />
        </tbody>
      </table>,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Detail transakcie 7' }))
    const note = screen.getByLabelText('Poznámka k transakcii 7')
    const recurring = screen.getByRole('button', { name: 'Pravidelná platba 7' })
    expect(note.compareDocumentPosition(recurring) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })
})

// A17: an empty filter used to show only the table headers with no next-action.
describe('Transactions empty state', () => {
  it('names the period as the likely reason once loading finishes with no rows', async () => {
    vi.mocked(api.listTransactions).mockResolvedValueOnce([])
    render(<Transactions />)
    expect(await screen.findByText('Za toto obdobie nič nie je. Skús iné obdobie hore.')).toBeInTheDocument()
    expect(screen.getByText(/Vytvorenie kategórie nevytvorí pravidlo/)).toBeInTheDocument()
    expect(screen.getByText(/Použiť aj na podobné navyše zaradí už importované nezaradené alebo navrhnuté platby/)).toBeInTheDocument()
  })
})

describe('Transactions bulk assign toast', () => {
  it('shows a toast when bulkAssign skips transfers', async () => {
    render(<Transactions />)
    await waitFor(() => expect(screen.getByText('Obchod')).toBeInTheDocument())

    fireEvent.click(screen.getAllByRole('checkbox')[0])
    const bulkBar = screen.getByText('1 vybraných').closest('div') as HTMLElement
    fireEvent.click(within(bulkBar).getByRole('combobox'))
    fireEvent.click(within(bulkBar).getByRole('option', { name: 'Jedlo' }))
    fireEvent.click(within(bulkBar).getByRole('button', { name: 'Priradiť' }))

    await waitFor(() => expect(vi.mocked(api.bulkAssign)).toHaveBeenCalled())
    expect(await screen.findByText('Prevody sa nepriraďujú, preskočené: 2')).toBeInTheDocument()
  })
})

// Point 11: bulk confirm. One click confirms the current selection through
// the same bulk bar used for bulk assignment; confirmed rows are never
// touched again by the same action.
describe('Transactions bulk confirm', () => {
  afterEach(() => {
    vi.mocked(api.confirm).mockReset().mockResolvedValue({ updated: 0, rules_created: 0, skipped_transfers: 0 })
  })

  it('confirms the whole selection in one click and reports how many were confirmed', async () => {
    vi.mocked(api.confirm).mockResolvedValueOnce({ updated: 1, rules_created: 0, skipped_transfers: 0 })
    render(<Transactions />)
    await waitFor(() => expect(screen.getByText('Obchod')).toBeInTheDocument())

    fireEvent.click(screen.getAllByRole('checkbox')[0])
    const bulkBar = screen.getByText('1 vybraných').closest('div') as HTMLElement
    fireEvent.click(within(bulkBar).getByRole('button', { name: 'Potvrdiť vybrané' }))

    await waitFor(() => expect(vi.mocked(api.confirm)).toHaveBeenCalledWith([5], false))
    expect(await screen.findByText('Potvrdených: 1.')).toBeInTheDocument()
  })
})

// Point 15: undo after bulk assignment. Späť is transient: valid only for
// the most recent bulk assignment, and it disappears once used.
describe('Transactions undo after bulk assignment', () => {
  afterEach(() => {
    vi.mocked(api.bulkAssign).mockReset().mockResolvedValue({ updated: 1, rules_created: 0, skipped_transfers: 0, undo_id: null })
    vi.mocked(api.undoLastAssignment).mockReset().mockResolvedValue(null)
  })

  it('offers Späť after a bulk assignment and restores rows through it', async () => {
    vi.mocked(api.bulkAssign).mockResolvedValueOnce({ updated: 1, rules_created: 0, skipped_transfers: 0, undo_id: 'assignment-undo-1' })
    vi.mocked(api.undoLastAssignment).mockResolvedValueOnce({ restored_rows: 1 })
    render(<Transactions />)
    await waitFor(() => expect(screen.getByText('Obchod')).toBeInTheDocument())

    fireEvent.click(screen.getAllByRole('checkbox')[0])
    const bulkBar = screen.getByText('1 vybraných').closest('div') as HTMLElement
    fireEvent.click(within(bulkBar).getByRole('combobox'))
    fireEvent.click(within(bulkBar).getByRole('option', { name: 'Jedlo' }))
    fireEvent.click(within(bulkBar).getByRole('button', { name: 'Priradiť' }))

    await waitFor(() => expect(vi.mocked(api.bulkAssign)).toHaveBeenCalled())
    const undoButton = await screen.findByRole('button', { name: 'Späť' })

    fireEvent.click(undoButton)
    await waitFor(() => expect(vi.mocked(api.undoLastAssignment)).toHaveBeenCalledWith('assignment-undo-1'))
    expect(await screen.findByText('Vrátených transakcií: 1.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Späť' })).not.toBeInTheDocument()
  })

  it('does not offer Späť when the backend reports no undo id for that assignment', async () => {
    vi.mocked(api.bulkAssign).mockResolvedValueOnce({ updated: 1, rules_created: 0, skipped_transfers: 0, undo_id: null })
    render(<Transactions />)
    await waitFor(() => expect(screen.getByText('Obchod')).toBeInTheDocument())

    fireEvent.click(screen.getAllByRole('checkbox')[0])
    const bulkBar = screen.getByText('1 vybraných').closest('div') as HTMLElement
    fireEvent.click(within(bulkBar).getByRole('combobox'))
    fireEvent.click(within(bulkBar).getByRole('option', { name: 'Jedlo' }))
    fireEvent.click(within(bulkBar).getByRole('button', { name: 'Priradiť' }))

    await waitFor(() => expect(vi.mocked(api.bulkAssign)).toHaveBeenCalled())
    expect(screen.queryByRole('button', { name: 'Späť' })).not.toBeInTheDocument()
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
  expect(filter).toHaveTextContent('Kategória ID 10 (názov nie je dostupný)')
  await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ category_id: 10, account_kind: 'personal' })))

  resolveCategories(categories)
  await waitFor(() => expect(filter).toHaveTextContent('Jedlo'))
  expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ category_id: 10, account_kind: 'personal' }))

  fireEvent.click(filter)
  fireEvent.click(screen.getByRole('option', { name: 'Potraviny' }))
  await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ category_id: 11, account_kind: 'personal' })))
  expect(filter).toHaveTextContent('Potraviny')
})

// Point 17: a merchant click in Overview lands here with the merchant name
// already in the search box and already applied as the text filter, with no
// debounce wait needed for the initial value.
it('applies initialText as the search filter immediately, without waiting for the debounce', async () => {
  render(<Transactions initialText="Kaufland" />)
  expect(screen.getByRole('textbox', { name: 'Hľadať obchodníka alebo poznámku' })).toHaveValue('Kaufland')
  await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ text: 'Kaufland' })))
})

// Point 3: creating a subcategory from an open statement's transactions must
// work without leaving the screen: pick "Nová kategória..." from the row's
// picker, name it, choose the parent, and the created category is assigned
// to that row immediately (already implemented; this pins the behavior).
it('creates a subcategory from the row picker, asks for the parent, and assigns it to that row', async () => {
  vi.mocked(api.saveCategory).mockResolvedValueOnce({ id: 9, parent_id: 1, name: 'Potraviny', kind: 'expense', sort: 1, system: false, archived: false })
  render(<Transactions />)
  await screen.findByText('Obchod')

  fireEvent.click(screen.getByRole('combobox', { name: 'Kategória transakcie 5' }))
  fireEvent.click(screen.getByRole('option', { name: 'Nová kategória...' }))
  const dialog = screen.getByRole('dialog', { name: 'Nová kategória' })
  await within(dialog).findByRole('option', { name: 'Jedlo' })
  fireEvent.change(within(dialog).getByLabelText('Nadradená kategória'), { target: { value: '1' } })
  fireEvent.change(within(dialog).getByLabelText('Názov kategórie'), { target: { value: 'Potraviny' } })
  fireEvent.click(within(dialog).getByRole('button', { name: 'Uložiť' }))

  await waitFor(() => expect(api.saveCategory).toHaveBeenCalledWith(null, 1, 'Potraviny', 'expense'))
  await waitFor(() => expect(api.assign).toHaveBeenCalledWith([5], 9, false))
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
})

// Point 13: fees need to be a separately filterable kind, so the kind filter
// must offer every TxKind including 'fee', not just the account/status/
// category/text filters that existed before.
it('offers Poplatok in the kind filter and sends kind: fee to listTransactions', async () => {
  render(<Transactions />)
  await screen.findByText('Obchod')
  const kindFilter = screen.getByRole('combobox', { name: 'Druh' })
  expect(within(kindFilter).getByRole('option', { name: 'Poplatok' })).toBeInTheDocument()

  fireEvent.change(kindFilter, { target: { value: 'fee' } })

  await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ kind: 'fee' })))
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

// Point 2: unassigned rows sharing a merchant are frequent and hard to tell
// apart one row at a time. The group strip surfaces them at the top of the
// table and hands the exact ids straight to the existing bulk selection.
describe('Transactions unassigned merchant groups (point 2)', () => {
  afterEach(() => {
    vi.mocked(api.listTransactions).mockReset().mockResolvedValue(DEFAULT_ROWS)
  })

  const TWITCH_ROWS = Array.from({ length: 15 }, (_, i) => makeTxRow({ id: 100 + i, merchant_raw: 'Twitch' }))

  it('shows a group strip for repeated unassigned merchants and selects the group rows on click', async () => {
    vi.mocked(api.listTransactions).mockResolvedValueOnce(TWITCH_ROWS)
    render(<Transactions />)

    const groupButton = await screen.findByRole('button', { name: 'Twitch, 15 platieb, nepriradené' })
    fireEvent.click(groupButton)

    expect(await screen.findByText('15 vybraných')).toBeInTheDocument()
  })

  it('hides the group strip once there are no unassigned rows left', async () => {
    vi.mocked(api.listTransactions).mockResolvedValueOnce([makeTxRow({ id: 5, status: 'confirmed' })])
    render(<Transactions />)
    await screen.findByText('Obchod')
    expect(screen.queryByRole('region', { name: 'Skupiny nezaradených platieb' })).not.toBeInTheDocument()
  })
})

// Point 12: keyboard control of the table. Arrow keys move a roving row
// highlight and Enter confirms that row's suggestion, but never when focus
// sits in the search field, a note, or the CategoryPicker's own search.
describe('Transactions keyboard row navigation (point 12)', () => {
  afterEach(() => {
    vi.mocked(api.listTransactions).mockReset().mockResolvedValue(DEFAULT_ROWS)
    vi.mocked(api.confirm).mockReset().mockResolvedValue({ updated: 0, rules_created: 0, skipped_transfers: 0 })
  })

  const SUGGESTED_ROWS = [
    makeTxRow({ id: 5, merchant_raw: 'Prvy', status: 'suggested' }),
    makeTxRow({ id: 6, merchant_raw: 'Druhy', status: 'suggested' }),
  ]

  it('moves the row highlight with arrow keys and confirms the focused suggestion on Enter', async () => {
    vi.mocked(api.listTransactions).mockResolvedValueOnce(SUGGESTED_ROWS)
    vi.mocked(api.confirm).mockResolvedValueOnce({ updated: 1, rules_created: 0, skipped_transfers: 0 })
    render(<Transactions />)
    await screen.findByText('Prvy')

    const table = screen.getByRole('region', { name: 'Transakcie' })
    fireEvent.keyDown(table, { key: 'ArrowDown' })
    fireEvent.keyDown(table, { key: 'ArrowDown' })
    fireEvent.keyDown(table, { key: 'Enter' })

    await waitFor(() => expect(vi.mocked(api.confirm)).toHaveBeenCalledWith([6], false))
  })

  it('ignores arrow keys and Enter typed in the search field', async () => {
    vi.mocked(api.listTransactions).mockResolvedValueOnce(SUGGESTED_ROWS)
    render(<Transactions />)
    await screen.findByText('Prvy')

    const search = screen.getByRole('textbox', { name: 'Hľadať obchodníka alebo poznámku' })
    fireEvent.keyDown(search, { key: 'ArrowDown' })
    fireEvent.keyDown(search, { key: 'ArrowDown' })
    fireEvent.keyDown(search, { key: 'Enter' })

    expect(vi.mocked(api.confirm)).not.toHaveBeenCalled()
  })

  it('ignores arrow keys and Enter fired from a category picker search inside the table', async () => {
    vi.mocked(api.listTransactions).mockResolvedValueOnce(SUGGESTED_ROWS)
    render(<Transactions />)
    await screen.findByText('Prvy')

    fireEvent.click(screen.getByRole('combobox', { name: 'Kategória transakcie 5' }))
    const search = screen.getByRole('searchbox', { name: 'Hľadať kategóriu' })
    fireEvent.keyDown(search, { key: 'ArrowDown' })
    fireEvent.keyDown(search, { key: 'Enter' })

    expect(vi.mocked(api.confirm)).not.toHaveBeenCalled()
  })
})

// Point 9: precise merchant+place substring search must work together with
// every other Transactions control, not just on its own.
describe('Transactions common search (point 9)', () => {
  afterEach(() => {
    vi.mocked(api.listAccounts).mockReset().mockResolvedValue([])
    vi.mocked(api.listTransactions).mockReset().mockResolvedValue(DEFAULT_ROWS)
    vi.mocked(files.saveCsv).mockReset()
    vi.mocked(api.exportCsv).mockReset()
  })

  it('combines search text with period, account, kind, category and status filters into one listTransactions call', async () => {
    vi.mocked(api.listAccounts).mockResolvedValueOnce([
      { id: 1, iban: 'SK4411000000000012345678', kind: 'personal', label: 'Osobný', has_password: false },
    ])
    render(<Transactions />)
    await screen.findByText('Obchod')
    vi.mocked(api.listTransactions).mockClear()

    // Period: click "Všetko" to get a deterministic from/to (null, null)
    // instead of depending on today's date.
    fireEvent.click(screen.getByRole('button', { name: 'Všetko' }))
    fireEvent.change(screen.getByRole('combobox', { name: 'Účet' }), { target: { value: '1' } })
    fireEvent.change(screen.getByRole('combobox', { name: 'Druh' }), { target: { value: 'fee' } })
    fireEvent.click(screen.getByRole('combobox', { name: 'Filter kategórie' }))
    fireEvent.click(screen.getByRole('option', { name: 'Jedlo' }))
    fireEvent.change(screen.getByRole('combobox', { name: 'Stav' }), { target: { value: 'unassigned' } })
    fireEvent.change(screen.getByRole('textbox', { name: 'Hľadať obchodníka alebo poznámku' }), { target: { value: 'Penny Neuss' } })

    const expectedFilter = {
      from: null, to: null,
      account_id: 1, account_kind: null,
      category_id: 1, status: 'unassigned', kind: 'fee',
      text: 'Penny Neuss', statement_id: null,
    }
    await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expectedFilter))
    // The full combination reaches listTransactions exactly once: earlier
    // calls (fired before the debounce settled) still carried text: null.
    const fullCombinationCalls = vi.mocked(api.listTransactions).mock.calls.filter(
      (call) => JSON.stringify(call[0]) === JSON.stringify(expectedFilter),
    )
    expect(fullCombinationCalls).toHaveLength(1)
  })

  it('keeps the group strip clickable and the search text in the input while a search is active', async () => {
    const twitchRows = Array.from({ length: 15 }, (_, i) => makeTxRow({ id: 300 + i, merchant_raw: 'Twitch' }))
    vi.mocked(api.listTransactions).mockResolvedValue(twitchRows)
    render(<Transactions />)
    await screen.findByRole('button', { name: 'Twitch, 15 platieb, nepriradené' })

    fireEvent.change(screen.getByRole('textbox', { name: 'Hľadať obchodníka alebo poznámku' }), { target: { value: 'twi' } })
    await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ text: 'twi' })))

    const groupButton = await screen.findByRole('button', { name: 'Twitch, 15 platieb, nepriradené' })
    fireEvent.click(groupButton)

    expect(await screen.findByText('15 vybraných')).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Hľadať obchodníka alebo poznámku' })).toHaveValue('twi')
  })

  it('sends the currently visible search text in the CSV export before the debounce settles', async () => {
    vi.mocked(files.saveCsv).mockResolvedValueOnce('C:/synthetic/export.csv')
    vi.mocked(api.exportCsv).mockResolvedValueOnce(3)
    render(<Transactions />)
    await screen.findByText('Obchod')
    vi.mocked(api.listTransactions).mockClear()

    fireEvent.change(screen.getByRole('textbox', { name: 'Hľadať obchodníka alebo poznámku' }), { target: { value: 'Penny Neuss' } })
    fireEvent.click(screen.getByRole('button', { name: 'Exportovať filtrované CSV' }))

    // The 300 ms debounce has not elapsed: no listTransactions refetch has
    // carried the new text yet, so this proves the export used the live
    // input value, not the (still-old) debounced filter.
    expect(vi.mocked(api.listTransactions).mock.calls.some(
      (call) => (call[0] as { text: string | null }).text === 'Penny Neuss',
    )).toBe(false)

    await waitFor(() => expect(vi.mocked(api.exportCsv)).toHaveBeenCalledTimes(1))
    // The export payload is the contract: every unset filter is null and the
    // typed text travels as-is (README, Exportovať filtrované CSV).
    expect(vi.mocked(api.exportCsv).mock.calls[0][0]).toEqual({
      from: null, to: null,
      account_id: null, account_kind: null,
      category_id: null, status: null, kind: null,
      text: 'Penny Neuss', statement_id: null,
    })
  })

  it('clears the search text together with every other filter, so the next list call carries text: null', async () => {
    render(<Transactions />)
    await screen.findByText('Obchod')

    fireEvent.change(screen.getByRole('textbox', { name: 'Hľadať obchodníka alebo poznámku' }), { target: { value: 'Penny Neuss' } })
    await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ text: 'Penny Neuss' })))

    fireEvent.click(screen.getByRole('button', { name: 'Vymazať všetky filtre' }))

    expect(screen.getByRole('textbox', { name: 'Hľadať obchodníka alebo poznámku' })).toHaveValue('')
    // Clear-all resets the whole filter: the next list call carries only nulls.
    await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.at(-1)?.[0]).toEqual({
      from: null, to: null,
      account_id: null, account_kind: null,
      category_id: null, status: null, kind: null,
      text: null, statement_id: null,
    }))
  })
})

// 091/B10 p3 fix: `dbGeneration` is bumped by App once a database restore
// lands. A Transactions instance that is still mounted at that moment (not
// just freshly mounted through App's own `key` remount) must drop old-DB
// transient state and refetch, or it could offer an undo against a
// database that restore already replaced, or bulk-act on a selection of
// row ids that no longer mean anything.
describe('Transactions: dbGeneration bump resets transient state and refetches', () => {
  afterEach(() => {
    vi.mocked(api.bulkAssign).mockReset().mockResolvedValue({ updated: 1, rules_created: 0, skipped_transfers: 0, undo_id: null })
  })

  it('drops the undo offer and the bulk selection, and refetches accounts/categories/rows', async () => {
    vi.mocked(api.bulkAssign).mockResolvedValueOnce({ updated: 1, rules_created: 0, skipped_transfers: 0, undo_id: 'assignment-undo-1' })
    const { rerender } = render(<Transactions dbGeneration={1} />)
    await waitFor(() => expect(screen.getByText('Obchod')).toBeInTheDocument())

    fireEvent.click(screen.getAllByRole('checkbox')[0])
    const bulkBar = screen.getByText('1 vybraných').closest('div') as HTMLElement
    fireEvent.click(within(bulkBar).getByRole('combobox'))
    fireEvent.click(within(bulkBar).getByRole('option', { name: 'Jedlo' }))
    fireEvent.click(within(bulkBar).getByRole('button', { name: 'Priradiť' }))

    await screen.findByRole('button', { name: 'Späť' })
    // The bulk assign itself already clears the selection it just acted on
    // (existing behaviour), and reload()'s revision bump briefly empties
    // and refetches `rows` in the same beat; wait for the row to be back
    // before re-selecting it, or the checkbox is not there yet to click.
    await waitFor(() => expect(screen.getByText('Obchod')).toBeInTheDocument())
    fireEvent.click(screen.getAllByRole('checkbox')[0])
    // The generation bump below must clear a selection made AFTER that
    // assignment too, not just whatever the assignment itself already cleared.
    expect(screen.getByText('1 vybraných')).toBeInTheDocument()
    const listAccountsCallsBefore = vi.mocked(api.listAccounts).mock.calls.length
    const listTransactionsCallsBefore = vi.mocked(api.listTransactions).mock.calls.length

    rerender(<Transactions dbGeneration={2} />)

    await waitFor(() => expect(vi.mocked(api.listAccounts).mock.calls.length).toBeGreaterThan(listAccountsCallsBefore))
    await waitFor(() => expect(vi.mocked(api.listTransactions).mock.calls.length).toBeGreaterThan(listTransactionsCallsBefore))
    expect(screen.queryByRole('button', { name: 'Späť' })).not.toBeInTheDocument()
    expect(screen.queryByText('1 vybraných')).not.toBeInTheDocument()
  })

  it('does not refetch on the very first render, only a real generation change after mount', async () => {
    const callsBefore = vi.mocked(api.listTransactions).mock.calls.length
    render(<Transactions dbGeneration={3} />)
    await waitFor(() => expect(screen.getByText('Obchod')).toBeInTheDocument())
    expect(vi.mocked(api.listTransactions).mock.calls.length).toBe(callsBefore + 1)
  })
})
