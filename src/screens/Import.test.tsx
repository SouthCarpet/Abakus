import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ImportReport, RecentStatement, StatementDeletePreview } from '../api'
import { api } from '../api'
import { Import, ImportResultCard } from './Import'

afterEach(() => cleanup())

vi.mock('../api', () => ({
  api: {
    importStatements: vi.fn(),
    importWithPassword: vi.fn(),
    saveAccount: vi.fn(),
    recentStatements: vi.fn().mockResolvedValue([]),
    statementDeletePreview: vi.fn(),
    deleteStatement: vi.fn(),
  },
}))

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({ onDragDropEvent: () => Promise.resolve(() => {}) }),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

const base: ImportReport = { path: 'C:/x/vypis.pdf', status: 'imported', accountLabel: 'Osobný', accountKind: 'personal', ibanMasked: 'SK44...5678', iban: 'SK44', statementNumber: 6, periodStart: '2026-05-30', periodEnd: '2026-06-30', inserted: 8, duplicates: 0, checksum: { status: 'ok' }, warnings: [], message: null, statementId: 9 }
describe('ImportResultCard', () => {
  it('shows account, period, counts and checksum', () => {
    render(<ImportResultCard report={base} onPassword={vi.fn()} onAddAccount={vi.fn()} />)
    expect(screen.getByText('Osobný')).toBeInTheDocument(); expect(screen.getByText(/8 nových/)).toBeInTheDocument(); expect(screen.getByText('Kontrolný súčet sedí')).toBeInTheDocument()
  })
  it('offers a password button for a locked file', () => {
    render(<ImportResultCard report={{ ...base, status: 'locked', checksum: null }} onPassword={vi.fn()} onAddAccount={vi.fn()} />)
    expect(screen.getByRole('button', { name: 'Zadať heslo' })).toBeInTheDocument()
  })
  it('offers to add an unknown account with its kind prefilled', () => {
    const onAdd = vi.fn()
    render(<ImportResultCard report={{ ...base, status: 'unknown_account', accountLabel: null }} onPassword={vi.fn()} onAddAccount={onAdd} />)
    screen.getByRole('button', { name: 'Pridať účet' }).click()
    expect(onAdd).toHaveBeenCalledWith('SK44', 'personal')
  })
  // A17/F4: the backend's own failure text must reach the screen, not a generic label.
  it('shows the backend message for a failed import', () => {
    render(<ImportResultCard report={{ ...base, status: 'error', message: 'súbor nie je výpis Tatra banky' }} onPassword={vi.fn()} onAddAccount={vi.fn()} />)
    expect(screen.getByText('súbor nie je výpis Tatra banky')).toBeInTheDocument()
  })
  // A17/F5: continuing to Transakcie must carry the statement id the import wrote.
  it('passes the statement id to onContinue', () => {
    const onContinue = vi.fn()
    render(<ImportResultCard report={base} onPassword={vi.fn()} onAddAccount={vi.fn()} onContinue={onContinue} />)
    screen.getByRole('button', { name: 'Pokračovať na Transakcie' }).click()
    expect(onContinue).toHaveBeenCalledWith(9)
  })
})

describe('Import retry after unknown account', () => {
  it('threads the original remember=true choice into the post-add-account retry', async () => {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const lockedReport: ImportReport = { path: 'C:/x/locked.pdf', status: 'locked', accountLabel: null, accountKind: null, ibanMasked: null, iban: null, statementNumber: null, periodStart: null, periodEnd: null, inserted: 0, duplicates: 0, checksum: null, warnings: [], message: null, statementId: null }
    const unknownAccountReport: ImportReport = { ...lockedReport, status: 'unknown_account', iban: 'SK4411000000000012345678', accountKind: 'personal' }
    const importedReport: ImportReport = { ...lockedReport, status: 'imported', accountLabel: 'Osobný', accountKind: 'personal', inserted: 3 }

    vi.mocked(open).mockResolvedValue(lockedReport.path)
    vi.mocked(api.importStatements).mockResolvedValueOnce([lockedReport])
    vi.mocked(api.importWithPassword).mockResolvedValueOnce(unknownAccountReport).mockResolvedValueOnce(importedReport)
    vi.mocked(api.saveAccount).mockResolvedValue({ id: 1, iban: unknownAccountReport.iban ?? '', kind: 'personal', label: 'Test účet', has_password: false })

    render(<Import />)

    fireEvent.click(screen.getByRole('button', { name: 'Vybrať PDF' }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Zadať heslo' })).toBeInTheDocument())

    fireEvent.click(screen.getByRole('button', { name: 'Zadať heslo' }))
    fireEvent.change(screen.getByLabelText('Heslo'), { target: { value: 'secret' } })
    fireEvent.click(screen.getByLabelText('Zapamätať pre tento účet'))
    fireEvent.click(screen.getByRole('button', { name: 'Potvrdiť' }))
    await waitFor(() => expect(screen.getByRole('button', { name: 'Pridať účet' })).toBeInTheDocument())

    fireEvent.click(screen.getByRole('button', { name: 'Pridať účet' }))
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await waitFor(() => expect(vi.mocked(api.importWithPassword).mock.calls).toHaveLength(2))

    expect(vi.mocked(api.importWithPassword).mock.calls[1]).toEqual([lockedReport.path, 'secret', true])
  })
})

// A17/F1: delete asks a real question (the backend's own counts) and then
// removes the row, refreshing the list so the numbers change immediately.
describe('Recent imports: delete a statement', () => {
  beforeEach(() => vi.clearAllMocks())
  const statement: RecentStatement = { statement_id: 5, number: 6, period_end: '2026-06-30', account_label: 'Osobný', transaction_count: 8, checksum: { status: 'ok' } }
  const preview: StatementDeletePreview = { statement_id: 5, number: 6, account_label: 'Osobný', period_start: '2026-05-30', period_end: '2026-06-30', transaction_count: 8, confirmed_count: 2, rules_deleted: 1 }

  it('shows the preview counts before deleting and refreshes the list after', async () => {
    vi.mocked(api.recentStatements).mockResolvedValueOnce([statement]).mockResolvedValueOnce([])
    vi.mocked(api.statementDeletePreview).mockResolvedValueOnce(preview)
    vi.mocked(api.deleteStatement).mockResolvedValueOnce({ statement_id: 5, number: 6, transactions_deleted: 8, rules_deleted: 1, open_rows_reclassified: 0 })

    render(<Import />)
    await waitFor(() => expect(screen.getByText('č. 6')).toBeInTheDocument())

    fireEvent.click(screen.getByRole('button', { name: 'Zmazať' }))
    await waitFor(() => expect(api.statementDeletePreview).toHaveBeenCalledWith(5))
    await screen.findByText(/Odstráni sa 8 transakcií, z nich 2 potvrdených. Pravidlá, ktoré nepoužíva iný výpis: 1/)

    fireEvent.click(screen.getByRole('button', { name: 'Natrvalo zmazať' }))
    await waitFor(() => expect(api.deleteStatement).toHaveBeenCalledWith(5))
    await waitFor(() => expect(vi.mocked(api.recentStatements).mock.calls.length).toBeGreaterThan(1))
    await waitFor(() => expect(screen.queryByText('č. 6')).not.toBeInTheDocument())
  })

  it('cancelling the confirmation never calls deleteStatement', async () => {
    vi.mocked(api.recentStatements).mockResolvedValueOnce([statement])
    vi.mocked(api.statementDeletePreview).mockResolvedValueOnce(preview)

    render(<Import />)
    await waitFor(() => expect(screen.getByText('č. 6')).toBeInTheDocument())

    fireEvent.click(screen.getByRole('button', { name: 'Zmazať' }))
    await screen.findByText(/Odstráni sa 8 transakcií/)
    fireEvent.click(screen.getByRole('button', { name: 'Späť' }))

    expect(api.deleteStatement).not.toHaveBeenCalled()
  })
})

// A17: the row cells (date, account, number, count, checksum) had no names.
describe('Recent imports: column headers', () => {
  it('names every cell the rows already render', async () => {
    vi.mocked(api.recentStatements).mockResolvedValueOnce([
      { statement_id: 5, number: 6, period_end: '2026-06-30', account_label: 'Osobný', transaction_count: 8, checksum: { status: 'ok' } },
    ])
    render(<Import />)
    await waitFor(() => expect(screen.getByText('č. 6')).toBeInTheDocument())

    for (const label of ['Dátum', 'Účet', 'Číslo', 'Transakcie', 'Kontrolný súčet', 'Akcie']) {
      expect(screen.getByText(label)).toBeInTheDocument()
    }
  })
})

// A17: Zmazať used to outweigh Zobraziť transakcie on every row (filled
// danger next to a plain text control). It stays findable through the
// danger text tone, but no longer the loudest thing on the row.
describe('Recent imports: row-level Zmazať weight', () => {
  it('is a ghost button with the danger text tone, not the filled danger button', async () => {
    vi.mocked(api.recentStatements).mockResolvedValueOnce([
      { statement_id: 5, number: 6, period_end: '2026-06-30', account_label: 'Osobný', transaction_count: 8, checksum: { status: 'ok' } },
    ])
    render(<Import />)
    const button = await screen.findByRole('button', { name: 'Zmazať' })
    expect(button).toHaveClass('k-btn-ghost')
    expect(button).toHaveClass('k-text-danger')
    expect(button).not.toHaveClass('k-btn-danger')
  })
})

// Oracle: audit brief requires visible import/preview/delete failure and no duplicate pending deletion.
describe('Import audit failure boundaries', () => {
  it('shows a rejected file import instead of leaving an unhandled promise', async () => {
    const { open } = await import('@tauri-apps/plugin-dialog')
    vi.mocked(api.recentStatements).mockResolvedValue([])
    vi.mocked(open).mockResolvedValue('C:/synthetic/failure.pdf')
    vi.mocked(api.importStatements).mockRejectedValueOnce(new Error('PDF sa nedá načítať'))
    render(<Import />)
    fireEvent.click(screen.getByRole('button', { name: 'Vybrať PDF' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('PDF sa nedá načítať')
    expect(screen.getByRole('button', { name: 'Vybrať PDF' })).toBeEnabled()
  })

  it('shows statement preview failure without enabling destructive confirmation', async () => {
    vi.mocked(api.recentStatements).mockResolvedValue([{ statement_id: 91, number: 3, account_label: 'Test', period_end: '2026-09-01', transaction_count: 2, checksum: { status: 'ok' } }])
    vi.mocked(api.statementDeletePreview).mockRejectedValueOnce(new Error('Náhľad výpisu zlyhal'))
    render(<Import />)
    fireEvent.click(await screen.findByRole('button', { name: 'Zmazať' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Náhľad výpisu zlyhal')
    expect(screen.getByRole('button', { name: 'Natrvalo zmazať' })).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Späť' }))
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('keeps statement delete failure in its dialog with a retryable confirmation', async () => {
    vi.mocked(api.recentStatements).mockResolvedValue([{ statement_id: 91, number: 3, account_label: 'Test', period_end: '2026-09-01', transaction_count: 2, checksum: { status: 'ok' } }])
    vi.mocked(api.statementDeletePreview).mockResolvedValue({ statement_id: 91, number: 3, account_label: 'Test', period_start: '2026-08-01', period_end: '2026-09-01', transaction_count: 2, confirmed_count: 1, rules_deleted: 0 })
    vi.mocked(api.deleteStatement).mockRejectedValueOnce(new Error('Odstránenie výpisu zlyhalo'))
    render(<Import />)
    fireEvent.click(await screen.findByRole('button', { name: 'Zmazať' }))
    const remove = screen.getByRole('button', { name: 'Natrvalo zmazať' })
    await waitFor(() => expect(remove).toBeEnabled())
    fireEvent.click(remove)
    expect(await screen.findByRole('alert')).toHaveTextContent('Odstránenie výpisu zlyhalo')
    expect(screen.getByRole('dialog')).toBeVisible()
    expect(remove).toBeEnabled()
  })
})
