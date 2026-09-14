import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { open } from '@tauri-apps/plugin-dialog'
import { api } from '../api'
import { RestoreSection } from './RestoreSection'

afterEach(() => cleanup())
beforeEach(() => vi.clearAllMocks())

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(), save: vi.fn() }))
vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return { ...actual, api: { restorePreview: vi.fn(), restoreDatabase: vi.fn() } }
})

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}

const PREVIEW = { accounts: 2, statements: 5, transactions: 120, schema_version: 6 }

describe('RestoreSection: explains the credential-manager limit before any click', () => {
  it('states account passwords are not part of a backup', () => {
    render(<RestoreSection />)
    const text = screen.getByText(/Heslá k účtom/).textContent ?? ''
    expect(text).toMatch(/Správcu poverení/)
    expect(text).toMatch(/nie sú súčasťou zálohy/)
  })
})

describe('RestoreSection: cancelling the open dialog', () => {
  it('previews nothing and opens no confirmation dialog', async () => {
    vi.mocked(open).mockResolvedValue(null)
    render(<RestoreSection />)
    fireEvent.click(screen.getByRole('button', { name: 'Vybrať zálohu' }))
    await vi.waitFor(() => expect(open).toHaveBeenCalled())
    expect(api.restorePreview).not.toHaveBeenCalled()
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })
})

describe('RestoreSection: a valid backup shows a preview before touching anything', () => {
  it('shows the exact counts and never calls restoreDatabase on its own', async () => {
    vi.mocked(open).mockResolvedValue('C:/zálohy/abakus-zaloha.db')
    vi.mocked(api.restorePreview).mockResolvedValue(PREVIEW)
    render(<RestoreSection />)
    fireEvent.click(screen.getByRole('button', { name: 'Vybrať zálohu' }))

    const dialog = await screen.findByRole('dialog')
    expect(dialog).toHaveTextContent('Účty: 2')
    expect(dialog).toHaveTextContent('Výpisy: 5')
    expect(dialog).toHaveTextContent('Transakcie: 120')
    expect(api.restorePreview).toHaveBeenCalledExactlyOnceWith('C:/zálohy/abakus-zaloha.db')
    expect(api.restoreDatabase).not.toHaveBeenCalled()
  })

  it('repeats the credential-manager limit inside the confirmation dialog', async () => {
    vi.mocked(open).mockResolvedValue('C:/zálohy/abakus-zaloha.db')
    vi.mocked(api.restorePreview).mockResolvedValue(PREVIEW)
    render(<RestoreSection />)
    fireEvent.click(screen.getByRole('button', { name: 'Vybrať zálohu' }))

    const dialog = await screen.findByRole('dialog')
    expect(dialog.textContent ?? '').toMatch(/Heslá.*Správcu poverení.*nie sú súčasťou zálohy/)
  })

  it('states the safety-copy notice inside the confirmation dialog itself, not only on the card underneath', async () => {
    vi.mocked(open).mockResolvedValue('C:/zálohy/abakus-zaloha.db')
    vi.mocked(api.restorePreview).mockResolvedValue(PREVIEW)
    render(<RestoreSection />)
    fireEvent.click(screen.getByRole('button', { name: 'Vybrať zálohu' }))

    const dialog = await screen.findByRole('dialog')
    expect(dialog.textContent ?? '').toMatch(/bezpečnostnú kópiu súčasnej databázy/)
  })
})

describe('RestoreSection: an invalid or foreign backup is refused before any dialog opens', () => {
  it('shows the backend Slovak error and opens no confirmation dialog', async () => {
    vi.mocked(open).mockResolvedValue('C:/zálohy/cudzi-subor.db')
    vi.mocked(api.restorePreview).mockRejectedValue(new Error('Vybraný súbor nie je platná záloha Abakusu (chýbajú tabuľky: accounts).'))
    render(<RestoreSection />)
    fireEvent.click(screen.getByRole('button', { name: 'Vybrať zálohu' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Vybraný súbor nie je platná záloha Abakusu')
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })
})

describe('RestoreSection: confirming a restore', () => {
  it('calls restoreDatabase with the picked path, then reports the safety-copy path and tells the user to review data', async () => {
    vi.mocked(open).mockResolvedValue('C:/zálohy/abakus-zaloha.db')
    vi.mocked(api.restorePreview).mockResolvedValue(PREVIEW)
    vi.mocked(api.restoreDatabase).mockResolvedValue({ safety_copy_path: 'C:/Users/x/AppData/Local/Abakus/abakus-pred-obnovou-2026-09-13-1200.db' })
    render(<RestoreSection />)
    fireEvent.click(screen.getByRole('button', { name: 'Vybrať zálohu' }))
    await screen.findByRole('dialog')

    fireEvent.click(screen.getByRole('button', { name: 'Obnoviť databázu' }))

    expect(await screen.findByText(/Skontrolujte dáta/)).toBeInTheDocument()
    expect(screen.getByText(/C:\/Users\/x\/AppData\/Local\/Abakus\/abakus-pred-obnovou-2026-09-13-1200\.db/)).toBeInTheDocument()
    expect(api.restoreDatabase).toHaveBeenCalledExactlyOnceWith('C:/zálohy/abakus-zaloha.db')
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('keeps the dialog open and retryable when the restore itself fails', async () => {
    vi.mocked(open).mockResolvedValue('C:/zálohy/abakus-zaloha.db')
    vi.mocked(api.restorePreview).mockResolvedValue(PREVIEW)
    vi.mocked(api.restoreDatabase)
      .mockRejectedValueOnce(new Error('Obnovu nemožno spustiť, kým prebieha import. Počkajte, kým import skončí, a skúste znova.'))
      .mockResolvedValueOnce({ safety_copy_path: 'C:/Users/x/AppData/Local/Abakus/abakus-pred-obnovou-2026-09-13-1200.db' })
    render(<RestoreSection />)
    fireEvent.click(screen.getByRole('button', { name: 'Vybrať zálohu' }))
    await screen.findByRole('dialog')

    const confirm = screen.getByRole('button', { name: 'Obnoviť databázu' })
    fireEvent.click(confirm)
    expect(await screen.findByRole('alert')).toHaveTextContent('Obnovu nemožno spustiť')
    expect(screen.getByRole('dialog')).toBeInTheDocument()

    fireEvent.click(confirm)
    expect(await screen.findByText(/Skontrolujte dáta/)).toBeInTheDocument()
    expect(api.restoreDatabase).toHaveBeenCalledTimes(2)
  })
})

describe('RestoreSection: busy covers pick, preview and confirm as their own flows', () => {
  it('disables the pick button for the whole pick+preview flow', async () => {
    const dialog = deferred<string | null>()
    vi.mocked(open).mockReturnValue(dialog.promise)
    render(<RestoreSection />)
    const pick = screen.getByRole('button', { name: 'Vybrať zálohu' })
    fireEvent.click(pick)
    expect(pick).toBeDisabled()
    fireEvent.click(pick)
    expect(open).toHaveBeenCalledTimes(1)

    dialog.resolve(null)
    await vi.waitFor(() => expect(pick).toBeEnabled())
    expect(api.restorePreview).not.toHaveBeenCalled()
  })

  it('disables the confirm button while the restore itself is running', async () => {
    vi.mocked(open).mockResolvedValue('C:/zálohy/abakus-zaloha.db')
    vi.mocked(api.restorePreview).mockResolvedValue(PREVIEW)
    const restoring = deferred<{ safety_copy_path: string }>()
    vi.mocked(api.restoreDatabase).mockReturnValue(restoring.promise)
    render(<RestoreSection />)
    fireEvent.click(screen.getByRole('button', { name: 'Vybrať zálohu' }))
    await screen.findByRole('dialog')
    const confirm = screen.getByRole('button', { name: 'Obnoviť databázu' })

    fireEvent.click(confirm)
    expect(confirm).toBeDisabled()
    fireEvent.click(confirm)
    expect(api.restoreDatabase).toHaveBeenCalledTimes(1)

    restoring.resolve({ safety_copy_path: 'C:/safety.db' })
    await vi.waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
  })
})
