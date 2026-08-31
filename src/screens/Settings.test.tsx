import { cleanup, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { NetLogRow, Release } from '../api'
import { api } from '../api'
import { Settings } from './Settings'

afterEach(() => cleanup())
beforeEach(() => vi.clearAllMocks())

vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }))

const release: Release = { tag: '0.2.0', url: 'https://github.com/SouthCarpet/Abakus/releases/tag/0.2.0', notes: '' }
const netLogRow: NetLogRow = { id: 1, started_at: '2026-08-31T10:00:00Z', url: 'https://api.github.com/repos/SouthCarpet/Abakus/releases/latest', status: '200', duration_ms: 120, bytes_in: 512 }

function mockApi(overrides: Partial<typeof api> = {}) {
  vi.mocked(api.listAccounts).mockResolvedValue([])
  vi.mocked(api.dataDir).mockResolvedValue('C:/data')
  vi.mocked(api.recentStatements).mockResolvedValue([])
  vi.mocked(api.netLog).mockResolvedValue([])
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
      getCheckUpdates: vi.fn(),
      checkUpdateNow: vi.fn(),
      setCheckUpdates: vi.fn(),
      runNetAudit: vi.fn(),
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
})
