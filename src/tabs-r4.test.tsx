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

// Oracle: Kaliber 5.0.0, Tabs (owner variant A): active tabs have a 2 px accent
// line, a selected tone and bold text. Only unsaved changes have a static dot.
function rulesOf(css: string, selector: string): string[] {
  const out: string[] = []
  const re = /([^{}]+)\{([^{}]*)\}/g
  let m: RegExpExecArray | null
  while ((m = re.exec(css.replace(/\/\*[\s\S]*?\*\//g, ''))) !== null) {
    if (m[1].split(',').some((s) => s.trim() === selector)) out.push(m[2])
  }
  return out
}

describe('active tab in Kaliber 5', () => {
  const css = readFileSync('src/kaliber.css', 'utf8')

  it('marks the active tab with a selected tone, bold text and the accent line', () => {
    const active = rulesOf(css, '.k-tab.is-active').join(';')
    expect(active).toContain('background: var(--color-surface-selected)')
    expect(active).toContain('font-weight: 700')
    expect(active).toContain('border-bottom-color: var(--color-accent)')
    expect(active).not.toMatch(/box-shadow|text-decoration|outline/)
    expect(rulesOf(css, '.k-tab.is-active::before')).toEqual([])
    expect(rulesOf(css, '.k-tab::before')).toEqual([])
  })

  it('reserves the bottom border at rest and keeps hover and header free of extra lines', () => {
    expect(rulesOf(css, '.k-tab').join(';')).toContain('border-bottom: 2px solid transparent')
    const all = [rulesOf(css, '.k-tab:hover'), rulesOf(css, '.k-header .k-tab')].flat().join(';')
    expect(all).not.toMatch(/border-bottom|text-decoration:\s*underline|box-shadow/)
  })

  it('shows a static round dot only for unsaved changes', () => {
    const dirty = rulesOf(css, '.k-tab.is-dirty::before').join(';')
    expect(dirty).toContain('width: 6px')
    expect(dirty).toContain('height: 6px')
    expect(dirty).toContain('border-radius: var(--radius-pill)')
    expect(dirty).toContain('background: var(--color-accent)')
    expect(dirty).not.toMatch(/animation|transition/)
  })

  it('reserves the bold width of every tab name, so the selection moves no name', () => {
    render(<App />)
    const buttons = within(screen.getByRole('navigation', { name: 'Sekcie' })).getAllByRole('button')
    expect(buttons.map((b) => b.getAttribute('data-k-label'))).toEqual(['Transakcie', 'Prehľad', 'Import', 'Kategórie', 'Nastavenia'])
  })
})
