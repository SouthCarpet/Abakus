import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from './api'
import { App } from './App'
import { resetEscapeLayers } from './kaliber/escape'
import { resetThemeState } from './kaliber/theme'

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

const widthBefore = window.innerWidth
function setWidth(width: number) {
  Object.defineProperty(window, 'innerWidth', { configurable: true, value: width })
}

beforeEach(() => {
  localStorage.clear()
  resetThemeState()
  resetEscapeLayers()
  for (const fn of [api.listAccounts, api.listCategories, api.listTransactions, api.statementHistory, api.badChecksums, api.recentStatements, api.netLog, api.netAuditFailures]) {
    vi.mocked(fn).mockResolvedValue([] as never)
  }
  vi.mocked(api.dataDir).mockResolvedValue('C:/data')
  vi.mocked(api.getCheckUpdates).mockResolvedValue(false)
})
afterEach(() => { cleanup(); vi.clearAllMocks(); setWidth(widthBefore) })

// Oracle: Kaliber R31 (a phone window of 599 px or less uses bottom navigation with icon and
// text for 3 to 5 targets; the current target is marked; every target stays a real button).
describe('Phone navigation (R31)', () => {
  it('shows the five sections in the bottom navigation and no header tabs at 390 px', () => {
    setWidth(390)
    const { container } = render(<App />)
    const nav = screen.getByRole('navigation', { name: 'Sekcie' })
    expect(nav).toHaveClass('k-bottom-nav')
    expect(within(nav).getAllByRole('button').map((button) => button.textContent)).toEqual(['Transakcie', 'Prehľad', 'Import', 'Kategórie', 'Nastavenia'])
    expect(container.querySelectorAll('.k-tabs')).toHaveLength(0)
    expect(container.querySelectorAll('.k-bottom-nav-item svg')).toHaveLength(5)
  })

  it('marks the current target and moves it on a click', () => {
    setWidth(390)
    render(<App />)
    const nav = screen.getByRole('navigation', { name: 'Sekcie' })
    expect(within(nav).getByRole('button', { name: 'Transakcie' })).toHaveAttribute('aria-current', 'page')
    fireEvent.click(within(nav).getByRole('button', { name: 'Kategórie' }))
    expect(within(nav).getByRole('button', { name: 'Kategórie' })).toHaveAttribute('aria-current', 'page')
    expect(within(nav).getByRole('button', { name: 'Transakcie' })).not.toHaveAttribute('aria-current')
  })

  it('switches to the header tabs when the window grows past the phone width', () => {
    setWidth(390)
    const { container } = render(<App />)
    expect(container.querySelectorAll('.k-bottom-nav')).toHaveLength(1)
    act(() => { setWidth(1024); window.dispatchEvent(new Event('resize')) })
    expect(container.querySelectorAll('.k-bottom-nav')).toHaveLength(0)
    expect(container.querySelectorAll('.k-tabs')).toHaveLength(1)
  })
})
