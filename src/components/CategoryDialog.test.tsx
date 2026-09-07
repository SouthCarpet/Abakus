import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { api } from '../api'
import { CategoryDialog } from './CategoryDialog'

afterEach(() => {
  cleanup()
  vi.mocked(api.saveCategory).mockClear()
})

vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return {
    ...actual,
    api: {
      listCategories: vi.fn().mockResolvedValue([
        { id: 1, parent_id: null, name: 'Jedlo', kind: 'expense', sort: 0, system: false, archived: false },
        { id: 2, parent_id: null, name: 'Faktúry', kind: 'income', sort: 0, system: false, archived: false },
      ]),
      saveCategory: vi.fn(),
    },
  }
})

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

describe('CategoryDialog', () => {
  it('offers active top-level parents and defaults to a new top-level category', async () => {
    render(<CategoryDialog open initialParentId={null} onClose={vi.fn()} onCreated={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('option', { name: 'Jedlo' })).toBeInTheDocument())
    expect(screen.getByLabelText('Nadradená kategória')).toHaveDisplayValue('Nová hlavná kategória')
    expect(screen.getByLabelText('Druh kategórie')).not.toBeDisabled()
  })

  it('locks the kind field to the selected parent kind', async () => {
    render(<CategoryDialog open onClose={vi.fn()} onCreated={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('option', { name: 'Faktúry' })).toBeInTheDocument())

    fireEvent.change(screen.getByLabelText('Nadradená kategória'), { target: { value: '2' } })

    expect(screen.getByLabelText('Druh kategórie')).toHaveValue('income')
    expect(screen.getByLabelText('Druh kategórie')).toBeDisabled()
  })

  it('creates through api.saveCategory and calls onCreated only after persistence', async () => {
    const created: import('../api').Category = { id: 9, parent_id: null, name: 'Nová', kind: 'expense', sort: 1, system: false, archived: false }
    vi.mocked(api.saveCategory).mockResolvedValueOnce(created)
    const onCreated = vi.fn()
    render(<CategoryDialog open onClose={vi.fn()} onCreated={onCreated} />)
    await waitFor(() => expect(screen.getByRole('option', { name: 'Jedlo' })).toBeInTheDocument())

    fireEvent.change(screen.getByLabelText('Názov kategórie'), { target: { value: 'Nová' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))

    await waitFor(() => expect(onCreated).toHaveBeenCalledWith(created))
    expect(api.saveCategory).toHaveBeenCalledWith(null, null, 'Nová', 'expense')
  })

  it('preserves the typed draft and shows the error when creation fails', async () => {
    vi.mocked(api.saveCategory).mockRejectedValueOnce(new Error('kategória s týmto názvom už existuje'))
    render(<CategoryDialog open onClose={vi.fn()} onCreated={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('option', { name: 'Jedlo' })).toBeInTheDocument())
    fireEvent.change(screen.getByLabelText('Názov kategórie'), { target: { value: 'Duplicitná' } })

    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))

    expect(await screen.findByText('Error: kategória s týmto názvom už existuje')).toBeInTheDocument()
    expect(screen.getByLabelText('Názov kategórie')).toHaveValue('Duplicitná')
  })

  it('disables Uložiť while busy so a duplicate save cannot fire twice', async () => {
    const pending = deferred<{ id: number; parent_id: null; name: string; kind: 'expense'; sort: number; system: boolean; archived: boolean }>()
    vi.mocked(api.saveCategory).mockReturnValueOnce(pending.promise)
    render(<CategoryDialog open onClose={vi.fn()} onCreated={vi.fn()} />)
    await waitFor(() => expect(screen.getByRole('option', { name: 'Jedlo' })).toBeInTheDocument())
    fireEvent.change(screen.getByLabelText('Názov kategórie'), { target: { value: 'Pomalá' } })

    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))

    expect(screen.getByRole('button', { name: 'Uložiť' })).toBeDisabled()
    pending.resolve({ id: 1, parent_id: null, name: 'Pomalá', kind: 'expense', sort: 0, system: false, archived: false })
    await waitFor(() => expect(api.saveCategory).toHaveBeenCalledTimes(1))
  })
})
