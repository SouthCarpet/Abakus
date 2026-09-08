import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { api } from '../api'
import { categoryApi } from '../lib/category-api'
import { Categories, RulesTable } from './Categories'

describe('RulesTable', () => {
  it('labels rule kinds in Slovak and hides delete for seed rules', () => {
    const rules = [
      { id: 1, kind: 'exact', key: 'aldi sued', place: 'neuss', category_id: 2, category_name: 'potraviny', parent_name: 'Jedlo', hit_count: 3 },
      { id: 2, kind: 'seed', key: 'lidl', place: null, category_id: 2, category_name: 'potraviny', parent_name: 'Jedlo', hit_count: 0 },
    ] as const
    render(<RulesTable rules={[...rules]} onDelete={vi.fn()} />)
    expect(screen.getByText('presné')).toBeInTheDocument(); expect(screen.getByText('slovník')).toBeInTheDocument()
    expect(screen.getAllByRole('button', { name: 'Zmazať' })).toHaveLength(1)
  })

  it('carries the overflow-safe key cell class so a long merchant string wraps instead of widening the card', () => {
    const rules = [
      { id: 1, kind: 'exact', key: 'aldi sued', place: 'neuss', category_id: 2, category_name: 'potraviny', parent_name: 'Jedlo', hit_count: 3 },
    ] as const
    const { container } = render(<RulesTable rules={[...rules]} onDelete={vi.fn()} />)
    const table = container.querySelector('table')
    expect(table).toHaveClass('k-table', 'k-rules-table')
  })
})

vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return {
    ...actual,
    api: {
      listCategories: vi.fn(),
      listRules: vi.fn().mockResolvedValue([]),
      saveCategory: vi.fn(),
      archiveCategory: vi.fn(),
      deleteRule: vi.fn(),
    },
  }
})

vi.mock('../lib/category-api', () => ({ categoryApi: { preview: vi.fn(), update: vi.fn() } }))

const JEDLO: import('../api').Category = { id: 1, parent_id: null, name: 'Jedlo', kind: 'expense', sort: 0, system: false, archived: false }
const POTRAVINY: import('../api').Category = { id: 2, parent_id: 1, name: 'potraviny', kind: 'expense', sort: 0, system: false, archived: false }
const FAKTURY: import('../api').Category = { id: 3, parent_id: null, name: 'Faktúry', kind: 'income', sort: 1, system: false, archived: false }

// Oracle: live r4 measure at 1024/1280; minWidth 560 wraps the Pravidlá card
// onto its own row at 1024 and keeps both columns at 1280 with every header
// inside .k-table-scroll (scrollLeft = 0, no page overflow).
describe('Categories rules column wrap threshold', () => {
  afterEach(() => {
    cleanup()
    vi.mocked(api.listCategories).mockReset()
  })

  it('sets the rules column min-width to 560px so the flex row wraps before the table clips', async () => {
    vi.mocked(api.listCategories).mockResolvedValue([JEDLO])
    const { container } = render(<Categories />)
    await waitFor(() => expect(screen.getByText('Pravidlá')).toBeInTheDocument())
    const col = container.querySelector('.k-rules-card-col')
    expect(col).toBeInstanceOf(HTMLElement)
    expect((col as HTMLElement).style.minWidth).toBe('560px')
  })
})

describe('Categories edit flow', () => {
  afterEach(() => {
    cleanup()
    vi.mocked(api.listCategories).mockReset()
    vi.mocked(categoryApi.preview).mockReset()
    vi.mocked(categoryApi.update).mockReset()
  })

  it('updates directly without a confirmation step when no kind change is involved', async () => {
    vi.mocked(api.listCategories).mockResolvedValue([JEDLO, POTRAVINY, FAKTURY])
    vi.mocked(categoryApi.preview).mockResolvedValue({ effective_kind: 'expense', affected_categories: 0, transaction_count: 0, confirmed_count: 0, requires_confirmation: false })
    vi.mocked(categoryApi.update).mockResolvedValue({ ...JEDLO, name: 'Jedlo a nápoje' })
    render(<Categories />)
    await waitFor(() => expect(screen.getAllByRole('button', { name: 'Upraviť' })[0]).toBeInTheDocument())

    fireEvent.click(screen.getAllByRole('button', { name: 'Upraviť' })[0])
    fireEvent.change(screen.getByLabelText('Upraviť názov kategórie'), { target: { value: 'Jedlo a nápoje' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))

    await waitFor(() => expect(categoryApi.update).toHaveBeenCalledWith(expect.objectContaining({ id: 1, name: 'Jedlo a nápoje', acknowledge_kind_change: false })))
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('shows the affected/confirmed counts and blocks the write until the user confirms the kind change', async () => {
    vi.mocked(api.listCategories).mockResolvedValue([JEDLO, POTRAVINY, FAKTURY])
    vi.mocked(categoryApi.preview).mockResolvedValue({ effective_kind: 'income', affected_categories: 2, transaction_count: 3, confirmed_count: 2, requires_confirmation: true })
    render(<Categories />)
    await waitFor(() => expect(screen.getAllByRole('button', { name: 'Upraviť' })[0]).toBeInTheDocument())
    fireEvent.click(screen.getAllByRole('button', { name: 'Upraviť' })[0])

    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))

    expect(await screen.findByText(/2 kategórií a 3 transakcií/)).toBeInTheDocument()
    expect(screen.getByText(/2 potvrdených/)).toBeInTheDocument()
    expect(categoryApi.update).not.toHaveBeenCalled()

    vi.mocked(categoryApi.update).mockResolvedValue({ ...JEDLO, kind: 'income' })
    fireEvent.click(screen.getByRole('button', { name: 'Potvrdiť zmenu druhu' }))
    await waitFor(() => expect(categoryApi.update).toHaveBeenCalledWith(expect.objectContaining({ id: 1, acknowledge_kind_change: true })))
  })

  it('cancelling the confirmation step makes no write and returns to the edit form', async () => {
    vi.mocked(api.listCategories).mockResolvedValue([JEDLO, POTRAVINY, FAKTURY])
    vi.mocked(categoryApi.preview).mockResolvedValue({ effective_kind: 'income', affected_categories: 1, transaction_count: 1, confirmed_count: 0, requires_confirmation: true })
    render(<Categories />)
    await waitFor(() => expect(screen.getAllByRole('button', { name: 'Upraviť' })[0]).toBeInTheDocument())
    fireEvent.click(screen.getAllByRole('button', { name: 'Upraviť' })[0])
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await screen.findByText(/1 kategórií a 1 transakcií/)

    fireEvent.click(screen.getByRole('button', { name: 'Zrušiť' }))

    expect(screen.getByLabelText('Upraviť názov kategórie')).toBeInTheDocument()
    expect(categoryApi.update).not.toHaveBeenCalled()
  })
})
