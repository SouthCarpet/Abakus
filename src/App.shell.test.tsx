import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from './api'
import { App } from './App'
import { resetEscapeLayers } from './kaliber/escape'
import { resetThemeState } from './kaliber/theme'
import { setLatestRelease } from './lib/updateStatus'

vi.mock('./api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('./api')>()
  return {
    ...actual,
    api: {
      listAccounts: vi.fn(), listCategories: vi.fn(), listTransactions: vi.fn(), statementHistory: vi.fn(),
      badChecksums: vi.fn(), summary: vi.fn(), recentStatements: vi.fn(), dataDir: vi.fn(), netLog: vi.fn(),
      netAuditFailures: vi.fn(), getCheckUpdates: vi.fn(), checkUpdateNow: vi.fn(), setCheckUpdates: vi.fn(),
    },
  }
})

beforeEach(() => {
  localStorage.clear()
  resetThemeState()
  resetEscapeLayers()
  for (const fn of [api.listAccounts, api.listCategories, api.listTransactions, api.statementHistory, api.badChecksums, api.recentStatements, api.netLog, api.netAuditFailures]) {
    vi.mocked(fn).mockResolvedValue([] as never)
  }
  vi.mocked(api.dataDir).mockResolvedValue('C:/data')
  vi.mocked(api.getCheckUpdates).mockResolvedValue(false)
  vi.mocked(api.checkUpdateNow).mockResolvedValue(null)
})
afterEach(() => { cleanup(); vi.clearAllMocks(); setLatestRelease(null) })

const tabs = () => within(screen.getByRole('navigation', { name: 'Sekcie' })).getAllByRole('button')

// Oracle: Kaliber R12 (5 sections as header tabs, Settings last, Ctrl+comma; Help in the footer
// and F1, a panel with search; Escape one layer per press), R16 (the register opens first),
// R20 (theme switch in the header tools; update check at start only with consent, quiet).
describe('App shell', () => {
  it('opens on the register with five header tabs, Settings last', async () => {
    render(<App />)
    expect(tabs().map((tab) => tab.textContent)).toEqual(['Transakcie', 'Prehľad', 'Import', 'Kategórie', 'Nastavenia'])
    expect(tabs()[0]).toHaveAttribute('aria-current', 'page')
    expect(screen.getByRole('heading', { name: 'Transakcie' })).toBeInTheDocument()
    expect(screen.getByRole('switch', { name: 'Tmavý vzhľad' })).toBeInTheDocument()
    await waitFor(() => expect(api.getCheckUpdates).toHaveBeenCalled())
  })

  it('opens Settings with Ctrl+comma', async () => {
    render(<App />)
    fireEvent.keyDown(window, { key: ',', code: 'Comma', ctrlKey: true })
    expect(tabs()[4]).toHaveAttribute('aria-current', 'page')
    expect(await screen.findByRole('heading', { name: 'Nastavenia' })).toBeInTheDocument()
  })

  it('opens Help with F1; Escape clears the search, then closes and returns the focus', async () => {
    render(<App />)
    const link = screen.getByRole('button', { name: 'Pomocník' })
    link.focus()
    fireEvent.keyDown(window, { key: 'F1' })
    const panel = screen.getByRole('dialog', { name: 'Pomocník' })
    const search = within(panel).getByRole('searchbox', { name: 'Hľadať v pomocníkovi' })
    expect(document.activeElement).toBe(search)
    fireEvent.change(search, { target: { value: 'zálohu nič také' } })
    expect(within(panel).getByText('Žiadna téma nezodpovedá hľadaniu.')).toBeInTheDocument()
    fireEvent.change(search, { target: { value: 'záloha' } })
    expect(within(panel).getByText('Nastavenia, záloha a aktualizácie')).toBeInTheDocument()
    fireEvent.keyDown(search, { key: 'Escape' })
    expect(search).toHaveValue('')
    expect(screen.getByRole('dialog', { name: 'Pomocník' })).toBeInTheDocument()
    fireEvent.keyDown(search, { key: 'Escape' })
    expect(screen.queryByRole('dialog', { name: 'Pomocník' })).not.toBeInTheDocument()
    expect(document.activeElement).toBe(link)
  })

  it('makes no update request at start while checking is off', async () => {
    render(<App />)
    await waitFor(() => expect(api.getCheckUpdates).toHaveBeenCalledTimes(1))
    await act(async () => {})
    expect(api.checkUpdateNow).not.toHaveBeenCalled()
  })

  it('checks once at start while checking is on and marks a found update quietly', async () => {
    vi.mocked(api.getCheckUpdates).mockResolvedValue(true)
    vi.mocked(api.checkUpdateNow).mockResolvedValue({
      tag: 'v0.2.1', url: 'https://github.com/SouthCarpet/Abakus/releases/tag/v0.2.1', notes: '',
      installer_url: null, installer_name: null, installer_size: null, checksums_url: null,
    })
    render(<App />)
    const line = await screen.findByRole('button', { name: 'Nová verzia v0.2.1' })
    expect(line).toHaveClass('k-update-line')
    expect(api.checkUpdateNow).toHaveBeenCalledTimes(1)
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('stays silent when the start check is offline', async () => {
    vi.mocked(api.getCheckUpdates).mockResolvedValue(true)
    vi.mocked(api.checkUpdateNow).mockRejectedValue('update check request failed: offline')
    render(<App />)
    await waitFor(() => expect(api.checkUpdateNow).toHaveBeenCalledTimes(1))
    await act(async () => {})
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(document.querySelector('.k-toast')).toBeNull()
  })
})
