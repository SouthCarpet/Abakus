import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Account, StatementHistoryRow } from '../../api'
import { StatementReminderBanner } from './StatementReminderBanner'

afterEach(() => cleanup())

const account = (overrides: Partial<Account> = {}): Pick<Account, 'id' | 'label' | 'kind'> => ({
  id: 1,
  kind: 'personal',
  label: 'Osobný',
  ...overrides,
})

const statement = (overrides: Partial<StatementHistoryRow> = {}): StatementHistoryRow => ({
  statement_id: 1,
  account_id: 1,
  account_label: 'Osobný',
  account_kind: 'personal',
  number: 1,
  period_start: '2026-01-01',
  period_end: '2026-01-31',
  opening_cents: 0,
  closing_cents: 0,
  transaction_count: 0,
  total_cents: 0,
  checksum: { status: 'ok' },
  ...overrides,
})

describe('StatementReminderBanner', () => {
  it('renders nothing when there is no gap to report', () => {
    render(
      <StatementReminderBanner
        accounts={[account()]}
        statements={[statement({ period_end: '2026-02-28' })]}
        accountKind={null}
        today="2026-03-08"
        onNavigateToImport={() => {}}
      />,
    )
    expect(screen.queryByText(/pokrytia výpismi/)).not.toBeInTheDocument()
  })

  it('names the gap as a coverage gap, not proof of missing transactions, and offers Import', () => {
    const onNavigateToImport = vi.fn()
    render(
      <StatementReminderBanner
        accounts={[account()]}
        statements={[statement({ period_end: '2026-01-31' })]}
        accountKind={null}
        today="2026-03-08"
        onNavigateToImport={onNavigateToImport}
      />,
    )
    expect(screen.getByText(/Osobný/)).toHaveTextContent(
      'Osobný: chýba výpis za obdobie 1. 2. 2026–28. 2. 2026. Toto je medzera v pokrytí výpismi, nie dôkaz chýbajúcich transakcií.',
    )
    fireEvent.click(screen.getByRole('button', { name: 'Prejsť na import' }))
    expect(onNavigateToImport).toHaveBeenCalled()
  })

  it('stays quiet during the seven-day grace period after month end', () => {
    render(
      <StatementReminderBanner
        accounts={[account()]}
        statements={[statement({ period_end: '2026-01-31' })]}
        accountKind={null}
        today="2026-03-07"
        onNavigateToImport={() => {}}
      />,
    )
    expect(screen.queryByRole('button', { name: 'Prejsť na import' })).not.toBeInTheDocument()
  })
})
