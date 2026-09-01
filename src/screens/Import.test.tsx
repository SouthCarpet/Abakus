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
    await screen.findByText(/zmaže 8 transakcií, z toho 2 potvrdených ručne, a 1 naučených pravidiel/)

    fireEvent.click(screen.getAllByRole('button', { name: 'Zmazať' })[1])
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
    await screen.findByText(/zmaže 8 transakcií/)
    fireEvent.click(screen.getByRole('button', { name: 'Zrušiť' }))

    expect(api.deleteStatement).not.toHaveBeenCalled()
  })
})
