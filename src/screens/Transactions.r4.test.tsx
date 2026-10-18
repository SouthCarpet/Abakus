import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api, type TxRow } from '../api'
import { Transactions } from './Transactions'
import { HELP_TOPICS } from '../components/HelpPanel'
import { resetToasts } from '../kaliber/toast'

// Round 091-r4-a2: the Senate audit findings on Transakcie (screen S6), one test each.

function makeRow(overrides: Partial<TxRow> & { id: number }): TxRow {
  return {
    account_id: 1, account_kind: 'personal', statement_number: 1, posted_date: '2026-07-02', tx_date: '2026-07-02',
    kind: 'card', amount_cents: -1240, orig_amount_cents: null, orig_currency: null, merchant_raw: 'PENNY', place: 'Neuss',
    counterparty_name: null, counterparty_iban: null, category_id: null, category_name: null, parent_name: null,
    status: 'unassigned', source: 'pdf', raw_block: '', note: '', ...overrides,
  }
}

vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return {
    ...actual,
    api: {
      listAccounts: vi.fn().mockResolvedValue([]),
      listCategories: vi.fn().mockResolvedValue([
        { id: 1, parent_id: null, name: 'Potraviny', kind: 'expense', sort: 0, system: false, archived: false },
      ]),
      listTransactions: vi.fn(),
      assign: vi.fn().mockResolvedValue({ updated: 1, rules_created: 0, skipped_transfers: 0 }),
      bulkAssign: vi.fn(),
      undoLastAssignment: vi.fn(),
      confirm: vi.fn().mockResolvedValue({ updated: 1, rules_created: 0, skipped_transfers: 0 }),
      saveTransactionNote: vi.fn(),
      saveCategory: vi.fn(),
      exportCsv: vi.fn(),
    },
  }
})
vi.mock('../lib/files', () => ({ files: { saveCsv: vi.fn() } }))

const ROWS: TxRow[] = [
  makeRow({ id: 1, status: 'suggested', category_id: 1, category_name: 'Potraviny' }),
  makeRow({ id: 2, status: 'unassigned', amount_cents: -2310 }),
  makeRow({ id: 3, status: 'unassigned', amount_cents: -875 }),
  makeRow({ id: 4, status: 'unassigned', merchant_raw: 'REWE', place: null }),
  makeRow({ id: 5, status: 'confirmed', category_id: 1, category_name: 'Potraviny', merchant_raw: 'Výplata', amount_cents: 185000 }),
  makeRow({ id: 6, status: 'transfer', amount_cents: -5000, merchant_raw: 'Prevod' }),
]

beforeEach(() => {
  resetToasts()
  localStorage.setItem('abakus.period', JSON.stringify({ kind: 'all' }))
  vi.mocked(api.listTransactions).mockResolvedValue(ROWS)
})
afterEach(() => cleanup())

async function loaded() {
  render(<Transactions onNavigateToImport={vi.fn()} />)
  await screen.findByText('Výplata')
}

describe('Transakcie in Kaliber r4', () => {
  // Audit 2: "Výber Všetko nemá viditeľný názov poľa".
  it('gives every filter a visible label, the period included', async () => {
    await loaded()
    expect(screen.getByRole('group', { name: 'Obdobie' })).toBeInTheDocument()
    for (const name of ['Obdobie', 'Účet', 'Stav', 'Kategória', 'Druh', 'Hľadať']) {
      const visible = screen.getAllByText(name).filter((el) => el.tagName === 'LABEL' || el.classList.contains('ab-field-name'))
      expect(visible.length, `visible label ${name}`).toBeGreaterThan(0)
    }
  })

  // Audit 3: the state text and the Potvrdiť button sit in separate cells (R28: separate columns).
  it('keeps the state and its action in different cells', async () => {
    await loaded()
    const row = document.querySelector('tr[data-row-id="1"]') as HTMLElement
    const stateCell = within(row).getByText('Odhad').closest('td') as HTMLElement
    const confirm = within(row).getByRole('button', { name: 'Potvrdiť' })
    expect(stateCell).not.toContainElement(confirm)
    expect(confirm.closest('td')).not.toBe(stateCell)
  })

  // Audit 6: one distinct mark per state, never colour alone.
  it('draws a different mark for each of the four states', async () => {
    await loaded()
    const markOf = (id: number) => {
      const tr = document.querySelector(`tr[data-row-id="${id}"]`) as HTMLElement
      return (tr.querySelector('.k-state-label .k-mark') as HTMLElement).className
    }
    const marks = [markOf(2), markOf(1), markOf(5), markOf(6)]
    expect(new Set(marks).size).toBe(4)
    expect(markOf(1)).toContain('k-mark--half')
    expect(markOf(5)).toContain('k-mark--full')
    expect(markOf(2)).not.toMatch(/k-mark--/)
    expect(markOf(6)).toContain('ab-mark--transfer')
    const label = document.querySelector('tr[data-row-id="2"] .k-state-label') as HTMLElement
    expect(label).toHaveTextContent('Nezaradené')
  })

  // Audit 5: correct Slovak plurals, and "nezaradené" everywhere (not "nepriradené").
  it('names the waiting payments with the right plural form and the state word Nezaradené', async () => {
    await loaded()
    const status = screen.getByRole('status', { name: 'Čakajú na zaradenie' })
    expect(status).toHaveTextContent('3 platby čakajú na zaradenie')
    expect(status).toHaveTextContent('PENNY, Neuss: 2 platby.')
    expect(status).toHaveTextContent('REWE: 1 platba.')
    expect(document.body.textContent).not.toMatch(/nepriraden/i)
  })

  it('counts the states of the list with the right forms', async () => {
    await loaded()
    const cap = screen.getByRole('status', { name: 'Počty podľa stavu' })
    expect(cap).toHaveTextContent('6 platieb')
    expect(cap).toHaveTextContent('3 nezaradené')
    expect(cap).toHaveTextContent('1 odhad')
    expect(cap).toHaveTextContent('1 potvrdená')
    expect(cap).toHaveTextContent('1 prevod')
  })

  it('writes a group button as merchant, place and count with the right form', async () => {
    await loaded()
    fireEvent.click(screen.getByRole('button', { name: 'Zaradiť podľa obchodníka' }))
    const region = screen.getByRole('region', { name: 'Skupiny nezaradených platieb' })
    expect(within(region).getByRole('button', { name: 'PENNY, Neuss: 2 platby' })).toBeInTheDocument()
    expect(within(region).getByRole('button', { name: 'REWE: 1 platba' })).toBeInTheDocument()
  })

  // Audit 3 and 1: the ink row exists only for a real task.
  it('shows no ink row when nothing waits', async () => {
    vi.mocked(api.listTransactions).mockResolvedValue([ROWS[4]])
    render(<Transactions />)
    await screen.findByText('Výplata')
    expect(screen.queryByRole('status', { name: 'Čakajú na zaradenie' })).not.toBeInTheDocument()
  })

  // Audit 4, then fix round a4 finding 1: the long help moved to the help panel (F1); the screen keeps one short hint.
  it('keeps the long rule text out of the screen; the help panel holds it', async () => {
    await loaded()
    expect(screen.queryByText(/Vytvorenie kategórie nevytvorí pravidlo/)).not.toBeInTheDocument()
    const lines = HELP_TOPICS.find((topic) => topic.id === 'transactions')?.lines.join('\n') ?? ''
    expect(lines).toContain('Vytvorenie kategórie nevytvorí pravidlo')
    expect(lines).toContain('Zaradiť podľa obchodníka')
  })

  // Audit 1: the filter block stays compact, the register is the first thing after it.
  it('shows the register right after one filter block', async () => {
    await loaded()
    const sheet = screen.getByRole('region', { name: 'Zoznam transakcií' })
    expect(within(sheet).getByRole('region', { name: 'Transakcie' })).toBeInTheDocument()
    expect(document.querySelectorAll('.ab-filters').length).toBe(1)
  })

  // R32: the total rule draws after a confirmed change, only then.
  it('draws the total rule after a confirmed change and not at load', async () => {
    await loaded()
    const total = () => document.querySelector('.k-ledger-total') as HTMLElement
    expect(total()).not.toHaveClass('is-changed')
    fireEvent.click(within(document.querySelector('tr[data-row-id="1"]') as HTMLElement).getByRole('button', { name: 'Potvrdiť' }))
    vi.mocked(api.listTransactions).mockResolvedValue(ROWS.map((r) => (r.id === 1 ? { ...r, status: 'confirmed' as const } : r)))
    await waitFor(() => expect(api.confirm).toHaveBeenCalled())
    await waitFor(() => expect(total()).toHaveClass('is-changed'))
  })

  // Fix round a4, finding 4 (R34a): the plain view carries a true meta line of the period and the account.
  it('shows a meta line with the period and the account in the plain view', async () => {
    await loaded()
    expect(document.querySelector('.k-page-meta')).toHaveTextContent('Obdobie: všetky dátumy. Všetky účty.')
  })

  it('names the dates of a bounded period', async () => {
    localStorage.setItem('abakus.period', JSON.stringify({ kind: 'custom', custom: { from: '2026-07-01', to: '2026-07-31' } }))
    await loaded()
    expect(document.querySelector('.k-page-meta')).toHaveTextContent(/Obdobie: .*2026.* až .*2026\./)
  })

  it('names the chosen account', async () => {
    vi.mocked(api.listAccounts).mockResolvedValueOnce([{ id: 1, iban: 'SK31', kind: 'personal', label: 'Bežný účet', has_password: false }])
    await loaded()
    await screen.findByRole('option', { name: 'Osobný - Bežný účet' })
    fireEvent.change(document.getElementById('f-acc') as HTMLSelectElement, { target: { value: '1' } })
    await waitFor(() => expect(document.querySelector('.k-page-meta')).toHaveTextContent('Účet: Bežný účet.'))
  })

  // Fix round a4, finding 2: every checkbox sits in a label that owns the click area. The size
  // comes from CSS (--k-hit) and is measured in a real browser by layout-geometry.test.ts.
  it('wraps the row checkbox, the disabled transfer one included, in a click-area label', async () => {
    await loaded()
    for (const id of [1, 6]) {
      const box = document.querySelector(`tr[data-row-id="${id}"] input[type="checkbox"]`) as HTMLInputElement
      expect(box.closest('label')).toHaveClass('ab-check-hit')
    }
    expect((document.querySelector('tr[data-row-id="6"] input[type="checkbox"]') as HTMLInputElement).disabled).toBe(true)
  })

  // Finding 1: the long key text and the rule fold leave the screen; the short hint stays as the table's description.
  it('keeps only a short key hint above the table and no fold', async () => {
    await loaded()
    const hint = document.getElementById('ab-register-keys') as HTMLElement
    expect(hint.textContent?.length).toBeLessThanOrEqual(80)
    expect(hint).toHaveTextContent('F1')
    expect(document.querySelector('.ab-fold')).toBeNull()
    expect(screen.getByRole('region', { name: 'Transakcie' })).toHaveAttribute('aria-describedby', 'ab-register-keys')
  })
})
