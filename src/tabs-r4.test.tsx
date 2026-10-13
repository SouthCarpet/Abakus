import { readFileSync } from 'node:fs'
import { cleanup, render, screen, within } from '@testing-library/react'
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
afterEach(() => { cleanup(); vi.clearAllMocks() })

// Audit 7: "Zelené podčiarknutie aktívnej záložky Transakcie je príliš tenké a málo kontrastné."
// R7 (revision 4): the active tab is a fill step, bold text and a square mark; there is no underline.
function rulesOf(css: string, selector: string): string[] {
  const out: string[] = []
  const re = /([^{}]+)\{([^{}]*)\}/g
  let m: RegExpExecArray | null
  while ((m = re.exec(css.replace(/\/\*[\s\S]*?\*\//g, ''))) !== null) {
    if (m[1].split(',').some((s) => s.trim() === selector)) out.push(m[2])
  }
  return out
}

describe('active tab in r4 (R7)', () => {
  const css = readFileSync('src/kaliber.css', 'utf8')

  it('marks the active tab by a tone step and a mark, with no underline or edge', () => {
    const active = rulesOf(css, '.k-tab.is-active').join(';')
    expect(active).toContain('background: var(--color-surface-selected)')
    expect(active).not.toMatch(/border|box-shadow|text-decoration|outline|var\(--color-accent\)/)
    expect(rulesOf(css, '.k-tab.is-active::before').join(';')).toContain('background: var(--color-accent)')
    expect(rulesOf(css, '.k-tab::before').join(';')).toContain('width: 6px')
  })

  it('never underlines a tab at rest, on hover or in the header', () => {
    const all = [rulesOf(css, '.k-tab'), rulesOf(css, '.k-tab:hover'), rulesOf(css, '.k-header .k-tab')].flat().join(';')
    expect(all).not.toMatch(/border-bottom|text-decoration:\s*underline|box-shadow/)
  })

  it('reserves the bold width of every tab name, so the selection moves no name', () => {
    render(<App />)
    const buttons = within(screen.getByRole('navigation', { name: 'Sekcie' })).getAllByRole('button')
    expect(buttons.map((b) => b.getAttribute('data-k-label'))).toEqual(['Transakcie', 'Prehľad', 'Import', 'Kategórie', 'Nastavenia'])
  })
})
