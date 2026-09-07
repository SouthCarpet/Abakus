import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api, type Account, type AccountDeletePreview, type TxRow } from '../api'
import { files } from '../lib/files'
import { Settings } from './Settings'
import { Transactions } from './Transactions'
import { Categories } from './Categories'
import { Overview } from './Overview'
import { App } from '../App'

// Owned IPC and file-picker boundaries. All records are synthetic.
vi.mock('../api', async (original) => ({
  ...await original<typeof import('../api')>(),
  api: {
    listAccounts: vi.fn(), listCategories: vi.fn(), listTransactions: vi.fn(),
    dataDir: vi.fn(), recentStatements: vi.fn(), netLog: vi.fn(), netAuditFailures: vi.fn(),
    getCheckUpdates: vi.fn(), accountDeletePreview: vi.fn(), deleteAccount: vi.fn(),
    exportCsv: vi.fn(), confirm: vi.fn(), assign: vi.fn(),
    listRules: vi.fn(), saveCategory: vi.fn(), archiveCategory: vi.fn(),
    summary: vi.fn(), badChecksums: vi.fn(), setCheckUpdates: vi.fn(), checkUpdateNow: vi.fn(),
  },
}))
vi.mock('../lib/files', () => ({ files: { saveCsv: vi.fn() } }))
const account: Account = { id: 7, label: 'Test účet', iban: 'SK3112000000198742637541', kind: 'personal', has_password: false }
const preview: AccountDeletePreview = { account_id: 7, label: 'Test účet', statement_count: 2, transaction_count: 3, confirmed_count: 1, rules_deleted: 1, has_password: false }
function row(overrides: Partial<TxRow> = {}): TxRow {
  return { id: 1, account_id: 7, account_kind: 'personal', statement_number: 1, posted_date: '2026-09-01', tx_date: '2026-09-01', kind: 'card', amount_cents: 12345, orig_amount_cents: null, orig_currency: null, merchant_raw: 'Mzda', place: null, counterparty_name: null, counterparty_iban: null, category_id: null, category_name: null, parent_name: null, status: 'unassigned', source: 'pdf', raw_block: '', note: '', ...overrides }
}
function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}
beforeEach(() => {
  vi.resetAllMocks(); localStorage.clear()
  vi.mocked(api.listAccounts).mockResolvedValue([account])
  vi.mocked(api.listCategories).mockResolvedValue([{ id: 8, parent_id: null, name: 'Jedlo', kind: 'expense', sort: 0, system: false, archived: false }])
  vi.mocked(api.listTransactions).mockResolvedValue([row()])
  vi.mocked(api.dataDir).mockResolvedValue('C:/synthetic')
  vi.mocked(api.recentStatements).mockResolvedValue([])
  vi.mocked(api.netLog).mockResolvedValue([]); vi.mocked(api.netAuditFailures).mockResolvedValue([])
  vi.mocked(api.getCheckUpdates).mockResolvedValue(false)
  vi.mocked(api.accountDeletePreview).mockResolvedValue(preview)
  vi.mocked(api.deleteAccount).mockResolvedValue({ account_id: 7, statements_deleted: 2, transactions_deleted: 3, rules_deleted: 1, open_rows_reclassified: 0 })
  vi.mocked(files.saveCsv).mockResolvedValue('C:/synthetic/export.csv')
  vi.mocked(api.exportCsv).mockResolvedValue(1)
  vi.mocked(api.listRules).mockResolvedValue([])
  vi.mocked(api.badChecksums).mockResolvedValue([])
})
afterEach(() => { cleanup(); localStorage.clear() })

// Oracle: contracts.md Account deletion IPC and UI acceptance.
describe('account deletion from its Settings row', () => {
  it('shows actual preview counts and preserves the account when cancelled', async () => {
    render(<Settings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Zmazať účet' }))
    const dialog = screen.getByRole('dialog', { name: 'Zmazať účet Test účet?' })
    expect(await within(dialog).findByText(/Výpisy: 2. Transakcie: 3/)).toBeVisible()
    expect(within(dialog).getByText(/Pôvodné bankové PDF/)).toBeVisible()
    expect(within(dialog).getByRole('button', { name: 'Natrvalo zmazať účet' })).toBeDisabled()
    fireEvent.click(within(dialog).getByRole('button', { name: 'Zrušiť' }))
    expect(api.deleteAccount).not.toHaveBeenCalled()
    expect(screen.getByText('Test účet')).toBeVisible()
  })
  it('requires the exact name and prevents a second delete while pending, then refreshes counts', async () => {
    const pending = deferred<Awaited<ReturnType<typeof api.deleteAccount>>>()
    vi.mocked(api.deleteAccount).mockReturnValue(pending.promise)
    render(<Settings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Zmazať účet' }))
    const name = await screen.findByLabelText('Na potvrdenie napíšte názov účtu: Test účet')
    fireEvent.change(name, { target: { value: 'Test' } })
    expect(screen.getByRole('button', { name: 'Natrvalo zmazať účet' })).toBeDisabled()
    fireEvent.change(name, { target: { value: 'Test účet' } })
    const remove = screen.getByRole('button', { name: 'Natrvalo zmazať účet' })
    fireEvent.click(remove); fireEvent.click(remove)
    expect(api.deleteAccount).toHaveBeenCalledTimes(1)
    expect(api.deleteAccount).toHaveBeenCalledWith(7)
    expect(remove).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Zrušiť' })).toBeDisabled()
    vi.mocked(api.listAccounts).mockResolvedValue([])
    await act(async () => pending.resolve({ account_id: 7, statements_deleted: 2, transactions_deleted: 3, rules_deleted: 1, open_rows_reclassified: 0 }))
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    expect(screen.queryByText('Test účet')).not.toBeInTheDocument()
    expect(screen.getByText(/Účty:/)).toHaveTextContent('Účty: 0')
    expect(screen.getByText(/Výpisy:/)).toHaveTextContent('Výpisy: 0')
  })
  it('shows preview failure and permits retry without ever deleting', async () => {
    vi.mocked(api.accountDeletePreview).mockRejectedValueOnce(new Error('Náhľad zlyhal'))
    render(<Settings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Zmazať účet' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Náhľad zlyhal')
    fireEvent.click(screen.getByRole('button', { name: 'Skúsiť znova' }))
    expect(await screen.findByLabelText(/Na potvrdenie/)).toBeEnabled()
    expect(api.deleteAccount).not.toHaveBeenCalled()
  })
  it('shows credential or database failure inside the confirmation and allows retry', async () => {
    vi.mocked(api.deleteAccount).mockRejectedValueOnce(new Error('Heslo sa nepodarilo odstrániť'))
    render(<Settings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Zmazať účet' }))
    fireEvent.change(await screen.findByLabelText(/Na potvrdenie/), { target: { value: 'Test účet' } })
    fireEvent.click(screen.getByRole('button', { name: 'Natrvalo zmazať účet' }))
    expect(await within(screen.getByRole('dialog')).findByRole('alert')).toHaveTextContent('Heslo sa nepodarilo odstrániť')
    expect(screen.getByRole('button', { name: 'Natrvalo zmazať účet' })).toBeEnabled()
  })
})

// Oracle: contracts.md three additions, period validation and async audit.
describe('transaction filters, export and sums', () => {
  it('exports all current constraints including fresh pre-debounce text and statement drill-down', async () => {
    render(<Transactions statementId={42} initialAccountKind="personal" initialCategoryId={8} initialStatus="unassigned" />)
    await screen.findByText('Mzda')
    fireEvent.change(screen.getByRole('combobox', { name: 'Účet' }), { target: { value: '7' } })
    fireEvent.change(screen.getByRole('textbox', { name: 'Hľadať obchodníka alebo poznámku' }), { target: { value: 'čerstvý text' } })
    fireEvent.click(screen.getByRole('button', { name: 'Exportovať filtrované CSV' }))
    await waitFor(() => expect(api.exportCsv).toHaveBeenCalledWith({ from: null, to: null, account_id: 7, account_kind: 'personal', category_id: 8, status: 'unassigned', text: 'čerstvý text', statement_id: 42 }, 'C:/synthetic/export.csv'))
    expect(await screen.findByText('Exportovaných transakcií: 1.')).toBeVisible()
  })
  it('clear removes all filters and selection including statement and account kind', async () => {
    render(<Transactions statementId={42} initialAccountKind="personal" initialCategoryId={8} initialStatus="unassigned" />)
    fireEvent.click(await screen.findByRole('checkbox', { name: /Vybrať transakciu/ }))
    expect(screen.getByText('1 vybraných')).toBeVisible()
    fireEvent.click(screen.getByRole('button', { name: 'Vymazať všetky filtre' }))
    await waitFor(() => expect(api.listTransactions).toHaveBeenLastCalledWith({ from: null, to: null, account_id: null, account_kind: null, category_id: null, status: null, text: null, statement_id: null }))
    expect(screen.queryByText('1 vybraných')).not.toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Hľadať obchodníka alebo poznámku' })).toHaveValue('')
  })
  it('counts three rows but excludes a 900 euro transfer from 123.45 income and 23.45 expense', async () => {
    vi.mocked(api.listTransactions).mockResolvedValue([row(), row({ id: 2, amount_cents: -2345 }), row({ id: 3, amount_cents: 90000, status: 'transfer' })])
    render(<Transactions />)
    const sums = await screen.findByRole('region', { name: 'Súčty zobrazených transakcií' })
    expect(sums).toHaveTextContent('Zobrazené transakcie: 3')
    expect(sums).toHaveTextContent('Príjem: 123,45 €')
    expect(sums).toHaveTextContent('Výdavky: 23,45 €')
    expect(sums).toHaveTextContent('Čisté: 100,00 €')
    expect(sums).toHaveTextContent('Prevody mimo súčtov: 1')
  })
  it('rejects an old response after a newer filter and hides previous sums during loading', async () => {
    const old = deferred<TxRow[]>()
    vi.mocked(api.listTransactions).mockReturnValueOnce(old.promise).mockResolvedValue([row({ merchant_raw: 'Nové' })])
    render(<Transactions />)
    fireEvent.change(screen.getByRole('combobox', { name: 'Stav' }), { target: { value: 'confirmed' } })
    expect(await screen.findByText('Nové')).toBeVisible()
    await act(async () => old.resolve([row({ merchant_raw: 'Staré' })]))
    expect(screen.queryByText('Staré')).not.toBeInTheDocument()
    const next = deferred<TxRow[]>()
    vi.mocked(api.listTransactions).mockReturnValueOnce(next.promise)
    fireEvent.change(screen.getByRole('combobox', { name: 'Stav' }), { target: { value: 'suggested' } })
    expect(screen.queryByRole('region', { name: 'Súčty zobrazených transakcií' })).not.toBeInTheDocument()
    await act(async () => next.reject(new Error('Čítanie zlyhalo')))
    expect(await screen.findByRole('alert')).toHaveTextContent('Čítanie zlyhalo')
    expect(screen.queryByText('Nové')).not.toBeInTheDocument()
  })
  it('cancels CSV without invoking export', async () => {
    vi.mocked(files.saveCsv).mockResolvedValue(null)
    render(<Transactions />); await screen.findByText('Mzda')
    fireEvent.click(screen.getByRole('button', { name: 'Exportovať filtrované CSV' }))
    expect(await screen.findByText('Export zrušený.')).toBeVisible()
    expect(api.exportCsv).not.toHaveBeenCalled()
  })
  it('shows CSV failure and permits retry', async () => {
    vi.mocked(api.exportCsv).mockRejectedValueOnce(new Error('Zápis zlyhal'))
    render(<Transactions />); await screen.findByText('Mzda')
    fireEvent.click(screen.getByRole('button', { name: 'Exportovať filtrované CSV' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Zápis zlyhal')
    expect(screen.getByRole('button', { name: 'Exportovať filtrované CSV' })).toBeEnabled()
  })
  it('blocks empty and reversed custom periods but accepts a same-day range', async () => {
    render(<Transactions />); await screen.findByText('Mzda')
    vi.mocked(api.listTransactions).mockClear()
    fireEvent.click(screen.getByRole('button', { name: 'Vlastné' }))
    expect(screen.getByRole('alert')).toHaveTextContent('Zadajte platný')
    expect(screen.getByRole('button', { name: 'Exportovať filtrované CSV' })).toBeDisabled()
    fireEvent.change(screen.getByLabelText('Od dátumu'), { target: { value: '2026-09-05' } })
    fireEvent.change(screen.getByLabelText('Do dátumu'), { target: { value: '2026-09-04' } })
    expect(api.listTransactions).not.toHaveBeenCalled()
    fireEvent.change(screen.getByLabelText('Do dátumu'), { target: { value: '2026-09-05' } })
    await waitFor(() => expect(api.listTransactions).toHaveBeenCalledWith(expect.objectContaining({ from: '2026-09-05', to: '2026-09-05' })))
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })
  it('shows confirm failure and leaves the suggested transaction actionable', async () => {
    vi.mocked(api.listTransactions).mockResolvedValue([row({ status: 'suggested', category_id: 8 })])
    vi.mocked(api.confirm).mockRejectedValue(new Error('Potvrdenie zlyhalo'))
    render(<Transactions />)
    fireEvent.click(await screen.findByRole('button', { name: 'Potvrdiť' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Potvrdenie zlyhalo')
    expect(screen.getByRole('button', { name: 'Potvrdiť' })).toBeEnabled()
    expect(within(screen.getByRole('table')).getByText('Odhad')).toBeVisible()
  })
})

// Oracle: audit brief, failures must remain visible in the control that initiated them.
describe('other audited screen boundaries', () => {
  it('shows duplicate category rejection inside the add dialog and leaves the name editable', async () => {
    vi.mocked(api.saveCategory).mockRejectedValue(new Error('Kategória už existuje'))
    render(<Categories />)
    await screen.findByLabelText('Názov kategórie 8')
    fireEvent.click(screen.getByRole('button', { name: '+ kategória' }))
    const dialog = screen.getByRole('dialog', { name: 'Pridať kategóriu' })
    fireEvent.change(within(dialog).getByLabelText('Názov'), { target: { value: 'jedlo' } })
    fireEvent.click(within(dialog).getByRole('button', { name: 'Uložiť' }))
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Kategória už existuje')
    expect(within(dialog).getByLabelText('Názov')).toHaveValue('jedlo')
    expect(within(dialog).getByRole('button', { name: 'Uložiť' })).toBeEnabled()
  })
  it('keeps failed category archive confirmation open and names the affected category', async () => {
    vi.mocked(api.archiveCategory).mockRejectedValue(new Error('Archív zlyhal'))
    render(<Categories />)
    fireEvent.click(await screen.findByRole('button', { name: 'Archivovať' }))
    const dialog = screen.getByRole('dialog', { name: 'Archivovať kategóriu' })
    expect(within(dialog).getByText('Jedlo')).toBeVisible()
    fireEvent.click(within(dialog).getByRole('button', { name: 'Archivovať' }))
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Archív zlyhal')
    expect(within(dialog).getByRole('button', { name: 'Archivovať' })).toBeEnabled()
  })
  it('shows overview query failure and recovers after a period click', async () => {
    vi.mocked(api.summary).mockRejectedValueOnce(new Error('Súhrn zlyhal')).mockResolvedValue({ income_cents: 0, expense_cents: 0, transfer_cents: 0, net_cents: 0, unassigned_count: 0, suggested_count: 0, by_category: [], by_month: [], by_month_category: [], top_merchants: [] })
    render(<Overview onNavigateToImport={vi.fn()} onNavigateToTransactions={vi.fn()} />)
    expect(await screen.findByRole('alert')).toHaveTextContent('Súhrn zlyhal')
    fireEvent.click(screen.getByRole('button', { name: 'Minulý mesiac' }))
    expect(await screen.findByText('Zatiaľ nič. Importuj prvý výpis.')).toBeVisible()
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })
  it('does not restore an old boot update preference after the user has chosen a new value', async () => {
    const boot = deferred<boolean>()
    vi.mocked(api.getCheckUpdates).mockReturnValue(boot.promise)
    vi.mocked(api.setCheckUpdates).mockResolvedValue(undefined)
    vi.mocked(api.checkUpdateNow).mockResolvedValue(null)
    render(<Settings />)
    await screen.findByText('Test účet')
    const checkbox = screen.getByRole('checkbox', { name: /Kontrolovať aktualizácie/ })
    fireEvent.click(checkbox)
    await waitFor(() => expect(checkbox).toBeChecked())
    await act(async () => boot.resolve(false))
    expect(checkbox).toBeChecked()
    expect(api.checkUpdateNow).toHaveBeenCalledTimes(1)
  })
})

// Oracle: audit brief drill-down reset. Real rail navigation must clear the prior statement.
it('returns from a checksum statement drill-down to ordinary transactions through the rail', async () => {
  localStorage.setItem('abakus.period', '{"kind":"all"}')
  vi.mocked(api.badChecksums).mockResolvedValue([{ statement_id: 42, number: 3, account_label: 'Test', off_by_cents: 100 }])
  vi.mocked(api.summary).mockResolvedValue({ income_cents: 0, expense_cents: 0, transfer_cents: 0, net_cents: 0, unassigned_count: 0, suggested_count: 0, by_category: [], by_month: [], by_month_category: [], top_merchants: [] })
  render(<App />)
  fireEvent.click(await screen.findByRole('button', { name: 'Zobraziť transakcie' }))
  await waitFor(() => expect(api.listTransactions).toHaveBeenLastCalledWith(expect.objectContaining({ statement_id: 42, from: null, to: null })))
  fireEvent.click(within(screen.getByRole('navigation')).getByRole('button', { name: 'Transakcie' }))
  await waitFor(() => expect(api.listTransactions).toHaveBeenLastCalledWith(expect.objectContaining({ statement_id: null })))
  expect(screen.queryByText('Vybraný výpis, všetky jeho dátumy.')).not.toBeInTheDocument()
})

// Oracle: PARENT-NOTE.md supplied synthetic June personal totals, independent of this implementation.
it('reduces June expenses by the refund and excludes the internal transfer', async () => {
  vi.mocked(api.listTransactions).mockResolvedValue([
    row({ id: 1, amount_cents: -199 }), row({ id: 2, amount_cents: 1000 }),
    row({ id: 3, amount_cents: 4300, kind: 'refund' }), row({ id: 4, amount_cents: -4000 }),
    row({ id: 5, amount_cents: -18000 }), row({ id: 6, amount_cents: -8000, status: 'transfer' }),
    row({ id: 7, amount_cents: -1369 }), row({ id: 8, amount_cents: -700 }),
  ])
  render(<Transactions />)
  const sums = await screen.findByRole('region', { name: 'Súčty zobrazených transakcií' })
  expect(sums).toHaveTextContent('Zobrazené transakcie: 8')
  expect(sums).toHaveTextContent('Príjem: 10,00 €')
  expect(sums).toHaveTextContent('Výdavky: 199,68 €')
  expect(sums).toHaveTextContent('Čisté: -189,68 €')
  expect(sums).toHaveTextContent('Prevody mimo súčtov: 1')
})

// Oracle: existing money contract, expense credits reduce expenses; income-category debits are not expenses.
it('respects assigned category kinds including expense credits and income-category debits', async () => {
  vi.mocked(api.listCategories).mockResolvedValue([
    { id: 8, parent_id: null, name: 'Jedlo', kind: 'expense', sort: 0, system: false, archived: false },
    { id: 9, parent_id: null, name: 'Mzda', kind: 'income', sort: 1, system: false, archived: false },
  ])
  vi.mocked(api.listTransactions).mockResolvedValue([
    row({ id: 1, amount_cents: 900, category_id: 8 }),
    row({ id: 2, amount_cents: -500, category_id: 9 }),
    row({ id: 3, amount_cents: 700, category_id: 9 }),
    row({ id: 4, amount_cents: 300, kind: 'refund' }),
    row({ id: 5, amount_cents: -2000, category_id: 8 }),
  ])
  render(<Transactions />)
  const sums = await screen.findByRole('region', { name: 'Súčty zobrazených transakcií' })
  expect(sums).toHaveTextContent('Príjem: 7,00 €')
  expect(sums).toHaveTextContent('Výdavky: 8,00 €')
  expect(sums).toHaveTextContent('Čisté: -1,00 €')
})

// Oracle: parent review retry requirement, unavailable metadata must not silently change financial classification.
it('reloads account and category sources through Obnoviť after initial metadata failure', async () => {
  vi.mocked(api.listCategories).mockRejectedValueOnce(new Error('Kategórie sa nedajú načítať'))
  render(<Transactions />)
  expect(await screen.findByRole('alert')).toHaveTextContent('Kategórie sa nedajú načítať')
  expect(screen.queryByRole('region', { name: 'Súčty zobrazených transakcií' })).not.toBeInTheDocument()
  fireEvent.click(screen.getByRole('button', { name: 'Obnoviť' }))
  expect(await screen.findByRole('region', { name: 'Súčty zobrazených transakcií' })).toBeVisible()
  expect(within(screen.getByRole('combobox', { name: 'Účet' })).getByRole('option', { name: 'Osobný - Test účet' })).toBeInTheDocument()
  expect(within(screen.getByRole('combobox', { name: 'Filter kategórie' })).getByRole('option', { name: 'Jedlo' })).toBeInTheDocument()
  expect(screen.queryByRole('alert')).not.toBeInTheDocument()
})
