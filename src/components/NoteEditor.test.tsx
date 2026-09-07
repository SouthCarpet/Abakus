import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '../api'
import { NoteEditor } from './NoteEditor'

afterEach(() => cleanup())
beforeEach(() => vi.clearAllMocks())

vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return { ...actual, api: { saveTransactionNote: vi.fn() } }
})

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}

const label = (id: number) => `Poznámka k transakcii ${id}`

describe('NoteEditor: showing and editing the note', () => {
  it('shows the transaction row current note text in the labeled textarea', () => {
    render(<NoteEditor txId={5} initialNote="predošlá poznámka" onSaved={vi.fn()} />)
    expect(screen.getByLabelText(label(5))).toHaveValue('predošlá poznámka')
  })

  it('saves the exact typed text, including embedded newlines, when Uložiť poznámku is clicked', async () => {
    vi.mocked(api.saveTransactionNote).mockResolvedValue(undefined)
    render(<NoteEditor txId={5} initialNote="" onSaved={vi.fn().mockResolvedValue(undefined)} />)
    fireEvent.change(screen.getByLabelText(label(5)), { target: { value: 'riadok 1\nriadok 2' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    await screen.findByText('Poznámka uložená.')
    expect(api.saveTransactionNote).toHaveBeenCalledExactlyOnceWith(5, 'riadok 1\nriadok 2')
  })

  it('saving an emptied textarea clears the note', async () => {
    vi.mocked(api.saveTransactionNote).mockResolvedValue(undefined)
    render(<NoteEditor txId={5} initialNote="stará poznámka" onSaved={vi.fn().mockResolvedValue(undefined)} />)
    fireEvent.change(screen.getByLabelText(label(5)), { target: { value: '' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    await screen.findByText('Poznámka uložená.')
    expect(api.saveTransactionNote).toHaveBeenCalledExactlyOnceWith(5, '')
  })
})

describe('NoteEditor: saving state and duplicate clicks', () => {
  it('disables Uložiť poznámku while a save is pending and ignores a second click', async () => {
    const pending = deferred<void>()
    vi.mocked(api.saveTransactionNote).mockReturnValue(pending.promise)
    render(<NoteEditor txId={5} initialNote="text" onSaved={vi.fn().mockResolvedValue(undefined)} />)
    const button = screen.getByRole('button', { name: 'Uložiť poznámku' })
    fireEvent.click(button)
    expect(button).toBeDisabled()
    fireEvent.click(button)
    pending.resolve()
    await screen.findByText('Poznámka uložená.')
    expect(api.saveTransactionNote).toHaveBeenCalledTimes(1)
  })
})

describe('NoteEditor: failed save keeps the draft', () => {
  it('keeps the typed draft and shows the backend error when the save itself fails', async () => {
    vi.mocked(api.saveTransactionNote).mockRejectedValueOnce(new Error('Poznámka presahuje limit 2000 znakov.'))
    render(<NoteEditor txId={5} initialNote="pôvodná" onSaved={vi.fn()} />)
    fireEvent.change(screen.getByLabelText(label(5)), { target: { value: 'rozpísaná poznámka, ešte neuložená' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Poznámka presahuje limit 2000 znakov.')
    expect(screen.getByLabelText(label(5))).toHaveValue('rozpísaná poznámka, ešte neuložená')
    expect(screen.getByRole('button', { name: 'Uložiť poznámku' })).toBeEnabled()
  })

  it('surfaces a rejected NUL character from the backend without losing the draft', async () => {
    vi.mocked(api.saveTransactionNote).mockRejectedValueOnce(new Error('Poznámka nesmie obsahovať znak U+0000.'))
    render(<NoteEditor txId={5} initialNote="" onSaved={vi.fn()} />)
    fireEvent.change(screen.getByLabelText(label(5)), { target: { value: 'a\0b' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('U+0000')
    expect(screen.getByLabelText(label(5))).toHaveValue('a\0b')
  })
})

describe('NoteEditor: mutation success vs refresh failure', () => {
  it('reports the note as saved even when the parent refresh afterward fails, and does not claim the save failed', async () => {
    vi.mocked(api.saveTransactionNote).mockResolvedValue(undefined)
    const onSaved = vi.fn().mockRejectedValue(new Error('Zoznam sa nepodarilo obnoviť.'))
    render(<NoteEditor txId={5} initialNote="" onSaved={onSaved} />)
    fireEvent.change(screen.getByLabelText(label(5)), { target: { value: 'nová poznámka' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Poznámka je uložená, obnovenie zoznamu zlyhalo: Error: Zoznam sa nepodarilo obnoviť.')
    expect(screen.queryByText('Poznámka uložená.')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Uložiť poznámku' })).toBeEnabled()
  })

  it('retries the refresh without repeating the write when the user clicks save again with the same unchanged draft', async () => {
    vi.mocked(api.saveTransactionNote).mockResolvedValue(undefined)
    const onSaved = vi.fn().mockRejectedValueOnce(new Error('Zoznam sa nepodarilo obnoviť.')).mockResolvedValueOnce(undefined)
    render(<NoteEditor txId={5} initialNote="" onSaved={onSaved} />)
    fireEvent.change(screen.getByLabelText(label(5)), { target: { value: 'nová poznámka' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    await screen.findByRole('alert')
    expect(api.saveTransactionNote).toHaveBeenCalledTimes(1)

    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    await screen.findByText('Poznámka uložená.')
    expect(api.saveTransactionNote).toHaveBeenCalledTimes(1)
    expect(onSaved).toHaveBeenCalledTimes(2)
  })

  it('invokes the parent refresh exactly once after a successful save', async () => {
    vi.mocked(api.saveTransactionNote).mockResolvedValue(undefined)
    const onSaved = vi.fn().mockResolvedValue(undefined)
    render(<NoteEditor txId={5} initialNote="" onSaved={onSaved} />)
    fireEvent.change(screen.getByLabelText(label(5)), { target: { value: 'raz' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    await screen.findByText('Poznámka uložená.')
    expect(api.saveTransactionNote).toHaveBeenCalledTimes(1)
    expect(onSaved).toHaveBeenCalledTimes(1)
  })
})

// Oracle: contract.md TxRow.note max is 2000 Unicode code points; U+1F600 is one
// code point encoded as a surrogate pair, so .repeat() builds an exact boundary.
describe('NoteEditor: the 2000 code-point limit counts Unicode code points, not UTF-16 units', () => {
  it('allows exactly 2000 code points', () => {
    render(<NoteEditor txId={5} initialNote={'😀'.repeat(2000)} onSaved={vi.fn()} />)
    expect(screen.getByText('2000 / 2000 znakov')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Uložiť poznámku' })).toBeEnabled()
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('blocks saving at 2001 code points and shows the length warning', () => {
    render(<NoteEditor txId={5} initialNote={'😀'.repeat(2001)} onSaved={vi.fn()} />)
    expect(screen.getByText('2001 / 2000 znakov')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Uložiť poznámku' })).toBeDisabled()
    expect(screen.getByRole('alert')).toHaveTextContent('Poznámka je príliš dlhá')
    expect(api.saveTransactionNote).not.toHaveBeenCalled()
  })
})

describe('NoteEditor: stays correct across a change of transaction', () => {
  it('shows the new transaction note and its own status after the row identity changes, unaffected by a save still pending for the previous one', async () => {
    const staleSave = deferred<void>()
    vi.mocked(api.saveTransactionNote).mockReturnValueOnce(staleSave.promise)
    const { rerender } = render(<NoteEditor txId={5} initialNote="poznámka A" onSaved={vi.fn()} />)
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))

    rerender(<NoteEditor txId={9} initialNote="poznámka B" onSaved={vi.fn()} />)
    expect(screen.getByLabelText(label(9))).toHaveValue('poznámka B')
    expect(screen.queryByText('Ukladá sa...')).not.toBeInTheDocument()

    // The stale save for transaction 5 settles only after 9 is already showing.
    staleSave.resolve()
    await Promise.resolve()
    await Promise.resolve()
    expect(screen.getByLabelText(label(9))).toHaveValue('poznámka B')
    expect(screen.queryByText('Poznámka uložená.')).not.toBeInTheDocument()

    // Saving is not left locked by the stale request settling under the new id.
    vi.mocked(api.saveTransactionNote).mockResolvedValueOnce(undefined)
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    await screen.findByText('Poznámka uložená.')
    expect(api.saveTransactionNote).toHaveBeenLastCalledWith(9, 'poznámka B')
  })

  it('never applies a stale save result after the component unmounts', async () => {
    const pending = deferred<void>()
    vi.mocked(api.saveTransactionNote).mockReturnValue(pending.promise)
    const onSaved = vi.fn().mockResolvedValue(undefined)
    const { unmount } = render(<NoteEditor txId={5} initialNote="poznámka" onSaved={onSaved} />)
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť poznámku' }))
    unmount()
    pending.resolve()
    await Promise.resolve()
    await Promise.resolve()
  })
})
