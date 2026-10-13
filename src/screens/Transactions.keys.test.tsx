import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api, type TxRow } from '../api'
import { Toasts } from '../components/Toasts'
import { INK_HOLD_MS } from '../kaliber/ink'
import { resetToasts } from '../kaliber/toast'
import { Transactions } from './Transactions'

function tx(id: number, overrides: Partial<TxRow> = {}): TxRow {
  return {
    id, account_id: 1, account_kind: 'personal', statement_number: 1, posted_date: '2026-05-01', tx_date: '2026-05-01',
    kind: 'card', amount_cents: -500, orig_amount_cents: null, orig_currency: null, merchant_raw: `Obchod ${id}`, place: null,
    counterparty_name: null, counterparty_iban: null, category_id: null, category_name: null, parent_name: null,
    status: 'unassigned', source: 'pdf', raw_block: '', note: '', ...overrides,
  }
}

vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return {
    ...actual,
    api: {
      listAccounts: vi.fn(),
      listCategories: vi.fn(),
      listTransactions: vi.fn(),
      bulkAssign: vi.fn(),
      undoLastAssignment: vi.fn(),
      confirm: vi.fn(),
      assign: vi.fn(),
    },
  }
})

const ROWS = [tx(1), tx(2), tx(3, { status: 'transfer' }), tx(4, { status: 'suggested', category_id: 1 })]

beforeEach(() => {
  vi.clearAllMocks()
  for (const fn of [api.bulkAssign, api.undoLastAssignment, api.confirm]) vi.mocked(fn).mockReset()
  localStorage.setItem('abakus.period', '{"kind":"all"}')
  vi.mocked(api.listAccounts).mockResolvedValue([])
  vi.mocked(api.listCategories).mockResolvedValue([{ id: 1, parent_id: null, name: 'Jedlo', kind: 'expense', sort: 0, system: false, archived: false }])
  vi.mocked(api.listTransactions).mockResolvedValue(ROWS)
  resetToasts()
  render(<Toasts />)
})
afterEach(() => { cleanup(); vi.useRealTimers(); localStorage.clear() })

const region = () => screen.getByRole('region', { name: 'Transakcie' })
const box = (id: number) => screen.getByRole('checkbox', { name: new RegExp(`^Vybrať transakciu ${id}:`) })
const selectedCount = () => screen.queryByText(/^Vybrané: [0-9]+$/)?.textContent ?? 'Vybrané: 0'

async function ready() {
  render(<Transactions />)
  await screen.findByText('Obchod 1')
}

// Oracle: Kaliber components.md "Selection model" and "Escape layering"; docs/plan-091-kaliber-r3.md
// key set (Ctrl+A, Space, Shift+arrows, Esc, Ctrl+Z, Ctrl+F); R14 toast with Undo.
describe('register keys', () => {
  it('selects all visible rows except the transfer with Ctrl+A, clears on the second press', async () => {
    await ready()
    fireEvent.keyDown(region(), { key: 'a', code: 'KeyA', ctrlKey: true })
    expect(selectedCount()).toBe('Vybrané: 3')
    expect(box(3)).not.toBeChecked()
    fireEvent.keyDown(region(), { key: 'a', code: 'KeyA', ctrlKey: true })
    expect(selectedCount()).toBe('Vybrané: 0')
  })

  it('toggles the focused row with Space and extends with Shift+ArrowDown', async () => {
    await ready()
    fireEvent.keyDown(region(), { key: 'ArrowDown' })
    fireEvent.keyDown(region(), { key: ' ', code: 'Space' })
    expect(box(1)).toBeChecked()
    fireEvent.keyDown(region(), { key: 'ArrowDown', shiftKey: true })
    expect(box(2)).toBeChecked()
    fireEvent.keyDown(region(), { key: 'End' })
    expect(within(region()).getByText('Obchod 4').closest('tr')).toHaveClass('is-active')
  })

  it('clears the selection with Escape and keeps the rows', async () => {
    await ready()
    fireEvent.click(box(1))
    fireEvent.click(box(2))
    expect(selectedCount()).toBe('Vybrané: 2')
    fireEvent.keyDown(region(), { key: 'Escape' })
    expect(box(1)).not.toBeChecked()
    expect(screen.getByText('Obchod 1')).toBeInTheDocument()
  })

  it('selects a range with Shift+click from the anchor', async () => {
    await ready()
    fireEvent.click(box(1))
    fireEvent.click(box(4), { shiftKey: true })
    expect(box(2)).toBeChecked()
    expect(box(3)).not.toBeChecked()
    expect(box(4)).toBeChecked()
  })

  it('focuses the search field with Ctrl+F, and Escape clears its text first', async () => {
    await ready()
    fireEvent.keyDown(window, { key: 'f', code: 'KeyF', ctrlKey: true })
    const search = screen.getByRole('searchbox', { name: 'Hľadať obchodníka alebo poznámku' })
    expect(document.activeElement).toBe(search)
    fireEvent.change(search, { target: { value: 'Penny' } })
    fireEvent.keyDown(search, { key: 'Escape' })
    expect(search).toHaveValue('')
    expect(document.activeElement).toBe(search)
  })

  it('shows the bulk result as a toast with Späť, and Ctrl+Z undoes it', async () => {
    vi.mocked(api.bulkAssign).mockResolvedValue({ updated: 1, rules_created: 0, skipped_transfers: 0, undo_id: 'u-1' })
    vi.mocked(api.undoLastAssignment).mockResolvedValue({ restored_rows: 1 } as Awaited<ReturnType<typeof api.undoLastAssignment>>)
    await ready()
    fireEvent.click(box(1))
    const bulkBar = screen.getByText('Vybrané: 1').closest('div') as HTMLElement
    fireEvent.click(within(bulkBar).getByRole('combobox'))
    fireEvent.click(within(bulkBar).getByRole('option', { name: 'Jedlo' }))
    fireEvent.click(within(bulkBar).getByRole('button', { name: 'Priradiť' }))
    const toast = (await screen.findByText('Priradenie dokončené.')).closest('.k-toast') as HTMLElement
    expect(within(toast).getByRole('button', { name: 'Späť' })).toBeInTheDocument()
    await waitFor(() => expect(box(1)).not.toBeChecked())
    fireEvent.keyDown(document.body, { key: 'z', code: 'KeyY', ctrlKey: true })
    await waitFor(() => expect(api.undoLastAssignment).toHaveBeenCalledWith('u-1'))
    expect(await screen.findByText('Vrátených transakcií: 1.')).toBeInTheDocument()
  })

  it('redoes the undone bulk assignment with Ctrl+Y and Ctrl+Shift+Z, and only once per undo', async () => {
    vi.mocked(api.bulkAssign).mockResolvedValueOnce({ updated: 1, rules_created: 0, skipped_transfers: 0, undo_id: 'u-1' })
      .mockResolvedValueOnce({ updated: 1, rules_created: 0, skipped_transfers: 0, undo_id: 'u-2' })
      .mockResolvedValueOnce({ updated: 1, rules_created: 0, skipped_transfers: 0, undo_id: 'u-3' })
    vi.mocked(api.undoLastAssignment).mockResolvedValue({ restored_rows: 1 } as Awaited<ReturnType<typeof api.undoLastAssignment>>)
    await ready()
    fireEvent.click(box(1))
    const bulkBar = screen.getByText('Vybrané: 1').closest('div') as HTMLElement
    fireEvent.click(within(bulkBar).getByRole('combobox'))
    fireEvent.click(within(bulkBar).getByRole('option', { name: 'Jedlo' }))
    fireEvent.click(within(bulkBar).getByRole('button', { name: 'Priradiť' }))
    await screen.findByText('Priradenie dokončené.')
    expect(api.bulkAssign).toHaveBeenLastCalledWith([1], 1, false)
    fireEvent.keyDown(document.body, { key: 'z', code: 'KeyY', ctrlKey: true })
    await waitFor(() => expect(api.undoLastAssignment).toHaveBeenCalledWith('u-1'))
    await screen.findByText('Vrátených transakcií: 1.')
    fireEvent.keyDown(document.body, { key: 'y', code: 'KeyZ', ctrlKey: true })
    await waitFor(() => expect(api.bulkAssign).toHaveBeenCalledTimes(2))
    expect(api.bulkAssign).toHaveBeenLastCalledWith([1], 1, false)
    fireEvent.keyDown(document.body, { key: 'y', code: 'KeyZ', ctrlKey: true })
    await act(async () => {})
    expect(api.bulkAssign).toHaveBeenCalledTimes(2)
    fireEvent.keyDown(document.body, { key: 'z', code: 'KeyY', ctrlKey: true })
    await waitFor(() => expect(api.undoLastAssignment).toHaveBeenLastCalledWith('u-2'))
    await waitFor(() => expect(screen.getAllByText('Vrátených transakcií: 1.').length).toBeGreaterThan(0))
    fireEvent.keyDown(document.body, { key: 'Z', code: 'KeyY', ctrlKey: true, shiftKey: true })
    await waitFor(() => expect(api.bulkAssign).toHaveBeenCalledTimes(3))
  })

  it('inks a row whose state a write changed, for ink_hold, and never at page load', async () => {
    vi.mocked(api.confirm).mockResolvedValue({ updated: 1, rules_created: 0 } as Awaited<ReturnType<typeof api.confirm>>)
    await ready()
    const row4 = () => screen.getByText('Obchod 4').closest('tr') as HTMLElement
    expect(row4()).not.toHaveClass('is-inked')
    vi.mocked(api.listTransactions).mockResolvedValue([tx(1), tx(2), tx(3, { status: 'transfer' }), tx(4, { status: 'confirmed', category_id: 1 })])
    fireEvent.keyDown(region(), { key: 'End' })
    fireEvent.keyDown(region(), { key: 'Enter' })
    await waitFor(() => expect(row4()).toHaveClass('is-inked'))
    expect(screen.getByText('Obchod 1').closest('tr')).not.toHaveClass('is-inked')
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, INK_HOLD_MS + 50)) })
    expect(row4()).not.toHaveClass('is-inked')
  })
})

// Plan 091 p3 follow-up (ledger seq 5433): the preview of "confirm similar" on the synthetic
// Penny Neuss rows (fixtures/synthetic/penny-neuss-2026-07.txt; the store side is
// crates/store/tests/penny_neuss.rs). Oracle: docs/plan-091-bulk-confirmation.md, same
// merchant and same place.
describe('Penny Neuss preview', () => {
  it('offers the two other PENNY Neuss payments, not Berlin, PENNY MARKT or REWE', async () => {
    const penny = (id: number, place: string, merchant = 'PENNY', status: TxRow['status'] = 'unassigned') =>
      tx(id, { merchant_raw: merchant, place, status, category_id: status === 'suggested' ? 1 : null })
    vi.mocked(api.listTransactions).mockResolvedValue([
      penny(11, 'Neuss', 'PENNY', 'suggested'), penny(12, 'Neuss'), penny(13, 'Neuss'),
      penny(14, 'Berlin'), penny(15, 'Neuss', 'PENNY MARKT'), penny(16, 'Neuss', 'REWE'),
    ])
    render(<Transactions />)
    const row = (await screen.findAllByText('PENNY'))[0].closest('tr') as HTMLElement
    expect(within(row).getByRole('checkbox', { name: 'Potvrdiť aj podobné (2)' })).toBeInTheDocument()
  })
})
