import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { recurringApi } from '../../lib/recurring-api'
import { RecurringTransactionAction } from './RecurringTransactionAction'
import { SAMPLE_CATEGORY, txRow } from './oracles'

afterEach(() => cleanup())

vi.mock('../../lib/recurring-api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../lib/recurring-api')>()
  return {
    ...actual,
    recurringApi: {
      overview: vi.fn(),
      detail: vi.fn(),
      transactionContext: vi.fn(),
      save: vi.fn(),
      reset: vi.fn(),
    },
  }
})

vi.mock('../../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../api')>()
  return { ...actual, api: { listCategories: vi.fn().mockResolvedValue([]), assign: vi.fn() } }
})

beforeEach(() => {
  vi.mocked(recurringApi.transactionContext).mockReset()
  vi.mocked(recurringApi.save).mockReset().mockResolvedValue({
    id: 3, series_key: 's:3', group_key: 'ins-key', account_id: 1,
    scope: 'group', mode: 'confirmed', cadence: 'yearly', anchor_date: '2026-01-15', updated_at: '2026-09-07T00:00:00Z',
  })
})

describe('RecurringTransactionAction eligibility', () => {
  it('renders nothing for transfer, refund or zero rows', () => {
    const onChanged = vi.fn().mockResolvedValue(undefined)
    const { rerender } = render(
      <RecurringTransactionAction row={txRow({ status: 'transfer' })} categories={[SAMPLE_CATEGORY]} onChanged={onChanged} />,
    )
    expect(screen.queryByRole('button', { name: /Pravidelná platba/ })).not.toBeInTheDocument()
    rerender(
      <RecurringTransactionAction row={txRow({ kind: 'refund' })} categories={[SAMPLE_CATEGORY]} onChanged={onChanged} />,
    )
    expect(screen.queryByRole('button', { name: /Pravidelná platba/ })).not.toBeInTheDocument()
    rerender(
      <RecurringTransactionAction row={txRow({ amount_cents: 0 })} categories={[SAMPLE_CATEGORY]} onChanged={onChanged} />,
    )
    expect(screen.queryByRole('button', { name: /Pravidelná platba/ })).not.toBeInTheDocument()
  })
})

describe('RecurringTransactionAction U01 manual yearly confirm', () => {
  it('opens the editor from context and saves yearly without assigning a category', async () => {
    const row = txRow({ id: 11, tx_date: '2026-01-15', status: 'unassigned', category_id: null, amount_cents: -10000 })
    vi.mocked(recurringApi.transactionContext).mockResolvedValue({
      transaction_id: 11,
      group_key: 'ins-key',
      ambiguous: false,
      decision: null,
      inferred_cadence: null,
      compatible_transactions: [row],
    })
    const onChanged = vi.fn().mockResolvedValue(undefined)
    render(<RecurringTransactionAction row={row} categories={[SAMPLE_CATEGORY]} onChanged={onChanged} />)
    fireEvent.click(screen.getByRole('button', { name: 'Pravidelná platba 11' }))
    expect(await screen.findByRole('dialog', { name: 'Pravidelná platba' })).toBeInTheDocument()
    await waitFor(() => expect(screen.getByLabelText('Kotva')).toHaveValue('2026-01-15'))
    fireEvent.change(screen.getByLabelText('Interval'), { target: { value: 'yearly' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await waitFor(() => expect(recurringApi.save).toHaveBeenCalledWith({
      decision_id: null,
      selection: { scope: 'group', transaction_id: 11 },
      decision: { mode: 'confirmed', cadence: 'yearly', anchor_date: '2026-01-15' },
    }))
    expect(onChanged).toHaveBeenCalledTimes(1)
  })

  it('opens the real category creation dialog', async () => {
    const row = txRow({ id: 11, status: 'unassigned', category_id: null })
    vi.mocked(recurringApi.transactionContext).mockResolvedValue({
      transaction_id: 11,
      group_key: 'ins-key',
      ambiguous: false,
      decision: null,
      inferred_cadence: null,
      compatible_transactions: [row],
    })
    render(<RecurringTransactionAction row={row} categories={[SAMPLE_CATEGORY]} onChanged={vi.fn().mockResolvedValue(undefined)} />)
    fireEvent.click(screen.getByRole('button', { name: 'Pravidelná platba 11' }))
    expect(await screen.findByRole('dialog', { name: 'Pravidelná platba' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Nová kategória...' }))
    expect(await screen.findByRole('dialog', { name: 'Nová kategória' })).toBeInTheDocument()
  })
})
