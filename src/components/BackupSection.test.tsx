import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { save } from '@tauri-apps/plugin-dialog'
import { api } from '../api'
import { BackupSection } from './BackupSection'

afterEach(() => cleanup())
beforeEach(() => vi.clearAllMocks())

vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }))
vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return { ...actual, api: { backupDatabase: vi.fn() } }
})

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}

describe('BackupSection: explains the backup before any click', () => {
  it('states the backup is unencrypted, excludes passwords and source PDFs, and needs a new filename', () => {
    render(<BackupSection />)
    const text = screen.getByText(/Záloha obsahuje/).textContent ?? ''
    expect(text).toMatch(/nezašifrované/)
    expect(text).toMatch(/Heslá.*pôvodné PDF.*neukladajú/)
    expect(text).toMatch(/nový názov súboru/)
  })
})

describe('BackupSection: cancelling the save dialog', () => {
  it('invokes no backup and shows no error or success message when the dialog is cancelled', async () => {
    vi.mocked(save).mockResolvedValue(null)
    render(<BackupSection />)
    fireEvent.click(screen.getByRole('button', { name: 'Zálohovať databázu' }))
    await vi.waitFor(() => expect(save).toHaveBeenCalled())
    expect(api.backupDatabase).not.toHaveBeenCalled()
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
    expect(screen.queryByText(/Záloha uložená/)).not.toBeInTheDocument()
  })
})

describe('BackupSection: a successful backup', () => {
  it('shows the actual saved path the backend reports, not the raw dialog choice', async () => {
    vi.mocked(save).mockResolvedValue('C:/Zálohy/abakus-zaloha-2026-09-07-090422005.db')
    vi.mocked(api.backupDatabase).mockResolvedValue({ path: 'C:/Zálohy/abakus-zaloha-2026-09-07-090422005.db', bytes: 40960 })
    render(<BackupSection />)
    fireEvent.click(screen.getByRole('button', { name: 'Zálohovať databázu' }))
    expect(await screen.findByText('Záloha uložená: C:/Zálohy/abakus-zaloha-2026-09-07-090422005.db')).toBeInTheDocument()
    expect(api.backupDatabase).toHaveBeenCalledExactlyOnceWith('C:/Zálohy/abakus-zaloha-2026-09-07-090422005.db')
  })
})

describe('BackupSection: failure stays visible and retryable', () => {
  it('shows the error, re-enables the button, and succeeds on retry', async () => {
    vi.mocked(save).mockResolvedValue('C:/Zálohy/zaloha.db')
    vi.mocked(api.backupDatabase)
      .mockRejectedValueOnce(new Error('Cieľový súbor je obsadený.'))
      .mockResolvedValueOnce({ path: 'C:/Zálohy/zaloha.db', bytes: 2048 })
    render(<BackupSection />)
    const button = screen.getByRole('button', { name: 'Zálohovať databázu' })
    fireEvent.click(button)
    expect(await screen.findByRole('alert')).toHaveTextContent('Cieľový súbor je obsadený.')
    expect(button).toBeEnabled()

    fireEvent.click(button)
    expect(await screen.findByText('Záloha uložená: C:/Zálohy/zaloha.db')).toBeInTheDocument()
    expect(api.backupDatabase).toHaveBeenCalledTimes(2)
  })
})

describe('BackupSection: busy covers the dialog and the snapshot as one flow', () => {
  it('disables the button for the whole flow and ignores a second click while the save dialog is still open', async () => {
    const dialog = deferred<string | null>()
    vi.mocked(save).mockReturnValue(dialog.promise)
    render(<BackupSection />)
    const button = screen.getByRole('button', { name: 'Zálohovať databázu' })
    fireEvent.click(button)
    expect(button).toBeDisabled()
    fireEvent.click(button)
    expect(save).toHaveBeenCalledTimes(1)

    dialog.resolve(null)
    await vi.waitFor(() => expect(button).toBeEnabled())
    expect(api.backupDatabase).not.toHaveBeenCalled()
  })
})
