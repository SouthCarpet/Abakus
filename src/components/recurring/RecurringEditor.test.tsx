import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Category } from '../../api'
import { api } from '../../api'
import { recurringApi } from '../../lib/recurring-api'
import type { RecurringCategoryDialogProps } from './category-dialog-props'
import { RecurringEditor, type RecurringEditorSource } from './RecurringEditor'
import { deferred, R14_ESTIMATE, SAMPLE_CATEGORY, txRow } from './oracles'

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

const categories: Category[] = [SAMPLE_CATEGORY]
const query = { from: null, to: null, account_kind: null, today: '2026-09-07' }

function FakeCategoryDialog({ open, onCreated, onClose }: RecurringCategoryDialogProps) {
  if (!open) return null
  return (
    <div role="dialog" aria-label="Nová kategória">
      <button type="button" onClick={() => onCreated({ id: 99, parent_id: 9, name: 'Doména', kind: 'expense', sort: 2, system: false, archived: false })}>
        Vytvoriť Doména
      </button>
      <button type="button" onClick={onClose}>Zavrieť kategóriu</button>
    </div>
  )
}

function transactionSource(overrides: Partial<RecurringEditorSource & { kind: 'transaction' }> = {}): RecurringEditorSource {
  const tx = txRow({ id: 11, tx_date: '2026-01-15', category_id: null, category_name: null, status: 'unassigned' })
  return {
    kind: 'transaction',
    tx,
    context: {
      transaction_id: 11,
      group_key: 'netflix-key',
      ambiguous: false,
      decision: null,
      inferred_cadence: null,
      compatible_transactions: [tx],
    },
    query,
    ...overrides,
  }
}

beforeEach(() => {
  vi.mocked(recurringApi.detail).mockReset().mockResolvedValue({
    row: R14_ESTIMATE,
    transactions: [txRow({ id: 44 })],
    matching_transaction_ids: [44],
    compatible_transactions: [txRow({ id: 45, tx_date: '2026-02-15' })],
  })
  vi.mocked(recurringApi.save).mockReset().mockResolvedValue({
    id: 8, series_key: 'g:estimate', group_key: 'estimate', account_id: 1,
    scope: 'group', mode: 'confirmed', cadence: 'yearly', anchor_date: '2026-01-15', updated_at: '2026-09-07T00:00:00Z',
  })
  vi.mocked(recurringApi.reset).mockReset().mockResolvedValue(undefined)
  vi.mocked(api.assign).mockReset().mockResolvedValue({ updated: 1, rules_created: 0, skipped_transfers: 0 })
})

describe('RecurringEditor cadence, ignore, reset and draft retention', () => {
  it('saves a yearly confirmation from a single uncategorized debit without assigning a category', async () => {
    const onChanged = vi.fn().mockResolvedValue(undefined)
    const onClose = vi.fn()
    render(
      <RecurringEditor source={transactionSource()} categories={categories} onClose={onClose} onChanged={onChanged} />,
    )
    fireEvent.change(screen.getByLabelText('Interval'), { target: { value: 'yearly' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(recurringApi.save).toHaveBeenCalledWith({
      decision_id: null,
      selection: { scope: 'group', transaction_id: 11 },
      decision: { mode: 'confirmed', cadence: 'yearly', anchor_date: '2026-01-15' },
    })
    expect(api.assign).not.toHaveBeenCalled()
  })

  it('keeps the typed cadence and anchor when save fails', async () => {
    vi.mocked(recurringApi.save).mockRejectedValueOnce(new Error('neplatná kotva'))
    render(
      <RecurringEditor source={transactionSource()} categories={categories} onClose={() => {}} onChanged={vi.fn()} />,
    )
    fireEvent.change(screen.getByLabelText('Interval'), { target: { value: 'quarterly' } })
    fireEvent.change(screen.getByLabelText('Kotva'), { target: { value: '2026-02-01' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('neplatná kotva')
    expect(screen.getByLabelText('Interval')).toHaveValue('quarterly')
    expect(screen.getByLabelText('Kotva')).toHaveValue('2026-02-01')
    expect(screen.getByRole('button', { name: 'Uložiť' })).toBeEnabled()
  })

  it('reports saved-plus-refresh-failed and retries refresh without a second save', async () => {
    const onChanged = vi.fn().mockRejectedValueOnce(new Error('obnova zlyhala')).mockResolvedValueOnce(undefined)
    const onClose = vi.fn()
    render(
      <RecurringEditor source={transactionSource()} categories={categories} onClose={onClose} onChanged={onChanged} />,
    )
    fireEvent.change(screen.getByLabelText('Interval'), { target: { value: 'yearly' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Uložené, obnovenie zlyhalo')
    expect(recurringApi.save).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(recurringApi.save).toHaveBeenCalledTimes(1)
    expect(onChanged).toHaveBeenCalledTimes(2)
  })

  it('disables duplicate save while a write is pending', async () => {
    const pending = deferred<{ id: number; series_key: string; group_key: string; account_id: number; scope: 'group'; mode: 'confirmed'; cadence: 'yearly'; anchor_date: string; updated_at: string }>()
    vi.mocked(recurringApi.save).mockReturnValue(pending.promise)
    render(
      <RecurringEditor source={transactionSource()} categories={categories} onClose={() => {}} onChanged={vi.fn().mockResolvedValue(undefined)} />,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    expect(screen.getByRole('button', { name: 'Uložiť' })).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    pending.resolve({
      id: 8, series_key: 'g:estimate', group_key: 'estimate', account_id: 1,
      scope: 'group', mode: 'confirmed', cadence: 'yearly', anchor_date: '2026-01-15', updated_at: '2026-09-07T00:00:00Z',
    })
    await waitFor(() => expect(recurringApi.save).toHaveBeenCalledTimes(1))
  })

  it('saves ignore without a cadence and restores inference through reset', async () => {
    const onChanged = vi.fn().mockResolvedValue(undefined)
    const source = transactionSource()
    if (source.kind !== 'transaction') throw new Error('expected transaction source')
    source.context = {
      ...source.context,
      decision: {
        id: 5, series_key: 'g:netflix', group_key: 'netflix-key', account_id: 1,
        scope: 'group', mode: 'confirmed', cadence: 'monthly', anchor_date: '2026-01-15', updated_at: '2026-08-01T00:00:00Z',
      },
    }
    const { rerender } = render(
      <RecurringEditor source={source} categories={categories} onClose={() => {}} onChanged={onChanged} />,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Toto nie je pravidelná platba' }))
    await waitFor(() => expect(recurringApi.save).toHaveBeenCalledWith({
      decision_id: 5,
      selection: { scope: 'group', transaction_id: 11 },
      decision: { mode: 'ignored' },
    }))
    rerender(
      <RecurringEditor source={source} categories={categories} onClose={() => {}} onChanged={onChanged} />,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Obnoviť odhad' }))
    await waitFor(() => expect(recurringApi.reset).toHaveBeenCalledWith(5))
  })
})

describe('RecurringEditor selected membership', () => {
  it('forces selected scope for a blank group key and sends only checked ids', async () => {
    const tx = txRow({ id: 11 })
    const extra = txRow({ id: 12, tx_date: '2026-02-15', merchant_raw: 'Netflix' })
    const source: RecurringEditorSource = {
      kind: 'transaction',
      tx,
      context: {
        transaction_id: 11,
        group_key: null,
        ambiguous: false,
        decision: null,
        inferred_cadence: 'yearly',
        compatible_transactions: [tx, extra],
      },
      query,
    }
    render(<RecurringEditor source={source} categories={categories} onClose={() => {}} onChanged={vi.fn().mockResolvedValue(undefined)} />)
    expect(screen.getByLabelText('Len označené transakcie')).toBeChecked()
    expect(screen.getByLabelText('Len označené transakcie')).toBeDisabled()
    expect(screen.getByText(/Ďalšie platby priraďte ručne/)).toBeInTheDocument()
    fireEvent.click(screen.getByLabelText('Člen 12 Netflix 2026-02-15'))
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await waitFor(() => expect(recurringApi.save).toHaveBeenCalled())
    expect(recurringApi.save).toHaveBeenCalledWith({
      decision_id: null,
      selection: { scope: 'selected', transaction_ids: [11, 12] },
      decision: { mode: 'confirmed', cadence: 'yearly', anchor_date: '2026-01-15' },
    })
  })
})

describe('RecurringEditor category create is independent of recurrence save', () => {
  it('preselects a created category and retries assign without creating it again', async () => {
    vi.mocked(api.assign).mockRejectedValueOnce(new Error('zaradenie zlyhalo')).mockResolvedValueOnce({ updated: 1, rules_created: 0, skipped_transfers: 0 })
    render(
      <RecurringEditor
        source={transactionSource()}
        categories={categories}
        CategoryDialog={FakeCategoryDialog}
        onClose={() => {}}
        onChanged={vi.fn()}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Nová kategória...' }))
    fireEvent.click(screen.getByRole('button', { name: 'Vytvoriť Doména' }))
    fireEvent.click(screen.getByRole('button', { name: 'Zaradiť kategóriu' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('zaradenie zlyhalo')
    expect(screen.getByText(/ostala vytvorená/)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Zaradiť kategóriu' }))
    await waitFor(() => expect(api.assign).toHaveBeenCalledTimes(2))
    expect(api.assign).toHaveBeenNthCalledWith(1, [11], 99, false)
    expect(api.assign).toHaveBeenNthCalledWith(2, [11], 99, false)
    expect(recurringApi.save).not.toHaveBeenCalled()
  })
})
