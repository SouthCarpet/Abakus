import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ImportReport } from '../api'
import { api } from '../api'
import { Import, ImportResultCard } from './Import'

afterEach(() => cleanup())

vi.mock('../api', () => ({
  api: {
    importStatements: vi.fn(),
    importWithPassword: vi.fn(),
    saveAccount: vi.fn(),
    recentStatements: vi.fn().mockResolvedValue([]),
  },
}))

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({ onDragDropEvent: () => Promise.resolve(() => {}) }),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

const base: ImportReport = { path: 'C:/x/vypis.pdf', status: 'imported', accountLabel: 'Osobný', accountKind: 'personal', ibanMasked: 'SK44...5678', iban: 'SK44', statementNumber: 6, periodStart: '2026-05-30', periodEnd: '2026-06-30', inserted: 8, duplicates: 0, checksum: { status: 'ok' }, warnings: [], message: null }
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
})

describe('Import retry after unknown account', () => {
  it('threads the original remember=true choice into the post-add-account retry', async () => {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const lockedReport: ImportReport = { path: 'C:/x/locked.pdf', status: 'locked', accountLabel: null, accountKind: null, ibanMasked: null, iban: null, statementNumber: null, periodStart: null, periodEnd: null, inserted: 0, duplicates: 0, checksum: null, warnings: [], message: null }
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
