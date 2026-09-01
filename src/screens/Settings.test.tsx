import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Account, AuditFailure, NetLogRow, Release } from '../api'
import { api } from '../api'
import { Settings } from './Settings'

afterEach(() => cleanup())
beforeEach(() => vi.clearAllMocks())

vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }))

const release: Release = { tag: '0.2.0', url: 'https://github.com/SouthCarpet/Abakus/releases/tag/0.2.0', notes: '' }
const netLogRow: NetLogRow = { id: 1, started_at: '2026-08-31T10:00:00Z', url: 'https://api.github.com/repos/SouthCarpet/Abakus/releases/latest', status: '200', duration_ms: 120, bytes_in: 512 }
const auditFailure: AuditFailure = { at: '2026-08-31T10:00:05Z', url: 'https://api.github.com/repos/SouthCarpet/Abakus/releases/latest', error: 'store lock poisoned' }

function mockApi(overrides: Partial<typeof api> = {}) {
  vi.mocked(api.listAccounts).mockResolvedValue([])
  vi.mocked(api.dataDir).mockResolvedValue('C:/data')
  vi.mocked(api.recentStatements).mockResolvedValue([])
  vi.mocked(api.netLog).mockResolvedValue([])
  vi.mocked(api.netAuditFailures).mockResolvedValue([])
  vi.mocked(api.getCheckUpdates).mockResolvedValue(false)
  vi.mocked(api.checkUpdateNow).mockResolvedValue(null)
  vi.mocked(api.setCheckUpdates).mockResolvedValue(undefined)
  vi.mocked(api.runNetAudit).mockResolvedValue(0)
  Object.assign(api, overrides)
}

vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return {
    ...actual,
    api: {
      listAccounts: vi.fn(),
      dataDir: vi.fn(),
      recentStatements: vi.fn(),
      netLog: vi.fn(),
      netAuditFailures: vi.fn(),
      getCheckUpdates: vi.fn(),
      checkUpdateNow: vi.fn(),
      setCheckUpdates: vi.fn(),
      runNetAudit: vi.fn(),
      updateAccount: vi.fn(),
    },
  }
})

describe('Settings: opt-in update check', () => {
  it('stays off and never calls checkUpdateNow when the flag is off', async () => {
    mockApi()
    render(<Settings />)
    await waitFor(() => expect(api.getCheckUpdates).toHaveBeenCalled())
    expect(screen.getByRole('checkbox', { name: /Kontrolovať aktualizácie/ })).not.toBeChecked()
    expect(api.checkUpdateNow).not.toHaveBeenCalled()
    expect(screen.queryByText(/Dostupná aktualizácia/)).not.toBeInTheDocument()
  })

  it('checks on mount and shows the quiet release line when the flag is already on', async () => {
    mockApi({ getCheckUpdates: vi.fn().mockResolvedValue(true), checkUpdateNow: vi.fn().mockResolvedValue(release) } as Partial<typeof api>)
    render(<Settings />)
    await waitFor(() => expect(screen.getByText('Dostupná aktualizácia 0.2.0')).toBeInTheDocument())
    expect(screen.getByText(release.url)).toBeInTheDocument()
  })

  it('turning the checkbox on persists the setting and runs the check immediately', async () => {
    mockApi({ checkUpdateNow: vi.fn().mockResolvedValue(release) } as Partial<typeof api>)
    render(<Settings />)
    const box = await screen.findByRole('checkbox', { name: /Kontrolovať aktualizácie/ })
    box.click()
    await waitFor(() => expect(api.setCheckUpdates).toHaveBeenCalledWith(true))
    await waitFor(() => expect(screen.getByText('Dostupná aktualizácia 0.2.0')).toBeInTheDocument())
  })
})

describe('Settings: network audit', () => {
  it('shows the empty state when net_log has no rows', async () => {
    mockApi()
    render(<Settings />)
    await waitFor(() => expect(screen.getByText('Žiadna sieťová aktivita')).toBeInTheDocument())
  })

  it('lists net_log rows and refreshes them after a manual scan', async () => {
    mockApi({ netLog: vi.fn().mockResolvedValue([netLogRow]) } as Partial<typeof api>)
    render(<Settings />)
    await waitFor(() => expect(screen.getByText(netLogRow.url)).toBeInTheDocument())

    screen.getByRole('button', { name: 'Skenovať teraz' }).click()
    await waitFor(() => expect(api.runNetAudit).toHaveBeenCalled())
    await waitFor(() => expect(vi.mocked(api.netLog).mock.calls.length).toBeGreaterThan(1))
  })

  // A17/F7: a write into net_log can itself fail. The failure is process-local
  // (lost on restart), but while the process runs, Nastavenia must show it.
  it('shows a failed audit write and hides the section once none remain', async () => {
    mockApi({ netAuditFailures: vi.fn().mockResolvedValue([auditFailure]) } as Partial<typeof api>)
    render(<Settings />)
    await waitFor(() => expect(screen.getByText(auditFailure.error)).toBeInTheDocument())
    expect(screen.getByText('Neúspešné zápisy auditu')).toBeInTheDocument()
  })

  it('shows nothing extra when every audit write succeeded', async () => {
    mockApi()
    render(<Settings />)
    await waitFor(() => expect(screen.getByText('Žiadna sieťová aktivita')).toBeInTheDocument())
    expect(screen.queryByText('Neúspešné zápisy auditu')).not.toBeInTheDocument()
  })
})

// A17/F2: label is a free edit, IBAN stays read-only, and a kind change on an
// account with imports needs the user's deliberate confirmation.
describe('Settings: edit account', () => {
  const account: Account = { id: 1, iban: 'SK4411000000000012345678', kind: 'personal', label: 'Osobný', has_password: false }

  it('edits the label and keeps the IBAN read-only', async () => {
    const updateAccount = vi.fn().mockResolvedValue({ ...account, label: 'Nový názov' })
    mockApi({ listAccounts: vi.fn().mockResolvedValue([account]), updateAccount } as Partial<typeof api>)
    render(<Settings />)

    fireEvent.click(await screen.findByRole('button', { name: 'Upraviť' }))
    const ibanField = screen.getByLabelText('IBAN (nedá sa zmeniť)') as HTMLInputElement
    expect(ibanField).toBeDisabled()
    expect(ibanField.value).toBe('SK44...5678')

    fireEvent.change(screen.getByLabelText('Názov účtu'), { target: { value: 'Nový názov' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await waitFor(() => expect(updateAccount).toHaveBeenCalledWith(1, 'Nový názov', 'personal', false))
  })

  it('shows the backend refusal and lets the user confirm the kind change deliberately', async () => {
    const refusal = 'Počet výpisov: 3. Počet transakcií: 12. Zmena typu účtu zmení ich zaradenie v Prehľade aj v exporte. Ak chcete pokračovať, potvrďte zmenu.'
    const updateAccount = vi.fn().mockRejectedValueOnce(refusal).mockResolvedValueOnce({ ...account, kind: 'business' })
    mockApi({ listAccounts: vi.fn().mockResolvedValue([account]), updateAccount } as Partial<typeof api>)
    render(<Settings />)

    fireEvent.click(await screen.findByRole('button', { name: 'Upraviť' }))
    fireEvent.change(screen.getByLabelText('Typ účtu'), { target: { value: 'business' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await screen.findByText(refusal)
    expect(updateAccount).toHaveBeenNthCalledWith(1, 1, 'Osobný', 'business', false)

    fireEvent.click(screen.getByRole('checkbox', { name: /Potvrdzujem zmenu typu účtu/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await waitFor(() => expect(updateAccount).toHaveBeenNthCalledWith(2, 1, 'Osobný', 'business', true))
  })
})
