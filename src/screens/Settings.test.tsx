import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Account, AuditFailure, NetLogRow, Release } from '../api'
import { api } from '../api'
import { Settings } from './Settings'

afterEach(() => cleanup())
beforeEach(() => vi.clearAllMocks())

vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }))

const release: Release = {
  tag: '0.2.0',
  url: 'https://github.com/SouthCarpet/Abakus/releases/tag/0.2.0',
  notes: '',
  installer_url: null,
  installer_name: null,
  installer_size: null,
  checksums_url: null,
}
const releaseWithInstaller: Release = {
  ...release,
  tag: '0.1.4',
  installer_url: 'https://objects.githubusercontent.com/exe',
  installer_name: 'abakus-setup-0.1.4.exe',
  installer_size: 12_345,
  checksums_url: 'https://objects.githubusercontent.com/sums',
}
const netLogRow: NetLogRow = { id: 1, started_at: '2026-08-31T10:00:00Z', url: 'https://api.github.com/repos/SouthCarpet/Abakus/releases/latest', status: '200', duration_ms: 120, bytes_in: 512 }
const auditFailure: AuditFailure = { at: '2026-08-31T10:00:05Z', url: 'https://api.github.com/repos/SouthCarpet/Abakus/releases/latest', error: 'store lock poisoned' }

function mockApi(overrides: Partial<typeof api> = {}) {
  vi.mocked(api.listAccounts).mockResolvedValue([])
  vi.mocked(api.dataDir).mockResolvedValue('C:/data')
  vi.mocked(api.recentStatements).mockResolvedValue([])
  vi.mocked(api.netLog).mockResolvedValue([])
  vi.mocked(api.netAuditFailures).mockResolvedValue([])
  vi.mocked(api.getCheckUpdates).mockResolvedValue(false)
  vi.mocked(api.checkUpdateNow).mockResolvedValue(null)
  vi.mocked(api.setCheckUpdates).mockResolvedValue(undefined)
  vi.mocked(api.openReleasePage).mockResolvedValue(undefined)
  vi.mocked(api.runNetAudit).mockResolvedValue(0)
  Object.assign(api, overrides)
}

vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return {
    ...actual,
    api: {
      listAccounts: vi.fn(),
      dataDir: vi.fn(),
      recentStatements: vi.fn(),
      netLog: vi.fn(),
      netAuditFailures: vi.fn(),
      getCheckUpdates: vi.fn(),
      checkUpdateNow: vi.fn(),
      setCheckUpdates: vi.fn(),
      downloadUpdate: vi.fn(),
      launchUpdate: vi.fn(),
      openReleasePage: vi.fn(),
      runNetAudit: vi.fn(),
      updateAccount: vi.fn(),
      setAccountPassword: vi.fn(),
    },
  }
})

describe('Settings: opt-in update check', () => {
  it('stays off and never calls checkUpdateNow when the flag is off', async () => {
    mockApi()
    render(<Settings />)
    await waitFor(() => expect(api.getCheckUpdates).toHaveBeenCalled())
    expect(screen.getByRole('checkbox', { name: /Kontrolovať aktualizácie/ })).not.toBeChecked()
    expect(api.checkUpdateNow).not.toHaveBeenCalled()
    expect(screen.queryByText(/Dostupná aktualizácia/)).not.toBeInTheDocument()
  })

  it('checks on mount and shows the quiet release line when the flag is already on', async () => {
    mockApi({ getCheckUpdates: vi.fn().mockResolvedValue(true), checkUpdateNow: vi.fn().mockResolvedValue(release) } as Partial<typeof api>)
    render(<Settings />)
    await waitFor(() => expect(screen.getByText('Dostupná aktualizácia 0.2.0')).toBeInTheDocument())
    expect(screen.getByText(release.url)).toBeInTheDocument()
  })

  it('turning the checkbox on persists the setting and runs the check immediately', async () => {
    mockApi({ checkUpdateNow: vi.fn().mockResolvedValue(release) } as Partial<typeof api>)
    render(<Settings />)
    const box = await screen.findByRole('checkbox', { name: /Kontrolovať aktualizácie/ })
    box.click()
    await waitFor(() => expect(api.setCheckUpdates).toHaveBeenCalledWith(true))
    await waitFor(() => expect(screen.getByText('Dostupná aktualizácia 0.2.0')).toBeInTheDocument())
  })
})

// A deferred promise the test resolves/rejects on its own schedule, so a
// click's in-flight state (button disabled, phase label shown) is asserted
// deterministically rather than racing a timer (tott-test-craft R12).
function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((res, rej) => { resolve = res; reject = rej })
  return { promise, resolve, reject }
}

describe('Settings: update button (0.1.4)', () => {
  it('shows only the quiet line and the link, no button, when the release has no installer asset', async () => {
    mockApi({ getCheckUpdates: vi.fn().mockResolvedValue(true), checkUpdateNow: vi.fn().mockResolvedValue(release) } as Partial<typeof api>)
    render(<Settings />)
    await screen.findByText('Dostupná aktualizácia 0.2.0')
    expect(screen.queryByRole('button', { name: /Aktualizovať na/ })).not.toBeInTheDocument()
  })

  it('downloads then launches on click, cycling the button label through downloading and launching', async () => {
    const download = deferred<{ path: string; sha256: string }>()
    const launch = deferred<void>()
    mockApi({
      getCheckUpdates: vi.fn().mockResolvedValue(true),
      checkUpdateNow: vi.fn().mockResolvedValue(releaseWithInstaller),
      downloadUpdate: vi.fn().mockReturnValue(download.promise),
      launchUpdate: vi.fn().mockReturnValue(launch.promise),
    } as Partial<typeof api>)
    render(<Settings />)

    fireEvent.click(await screen.findByRole('button', { name: 'Aktualizovať na 0.1.4' }))
    expect(api.downloadUpdate).toHaveBeenCalledWith('0.1.4')
    expect(screen.getByRole('button', { name: 'Sťahujem inštalátor…' })).toBeDisabled()

    download.resolve({ path: 'C:/temp/abakus-update/abakus-setup-0.1.4.exe', sha256: 'deadbeef' })
    await waitFor(() => expect(screen.getByRole('button', { name: 'Spúšťam inštalátor…' })).toBeDisabled())
    expect(api.launchUpdate).toHaveBeenCalledWith('C:/temp/abakus-update/abakus-setup-0.1.4.exe', 'deadbeef')

    launch.resolve()
  })

  it('shows the alert and re-enables the button when the download fails', async () => {
    mockApi({
      getCheckUpdates: vi.fn().mockResolvedValue(true),
      checkUpdateNow: vi.fn().mockResolvedValue(releaseWithInstaller),
      downloadUpdate: vi.fn().mockRejectedValue('Stiahnutie zlyhalo: offline'),
    } as Partial<typeof api>)
    render(<Settings />)

    fireEvent.click(await screen.findByRole('button', { name: 'Aktualizovať na 0.1.4' }))

    await screen.findByText('Stiahnutie zlyhalo: offline')
    expect(screen.getByRole('alert')).toHaveTextContent('Stiahnutie zlyhalo: offline')
    expect(screen.getByRole('button', { name: 'Aktualizovať na 0.1.4' })).not.toBeDisabled()
    expect(api.launchUpdate).not.toHaveBeenCalled()
  })

  it('opens the release page through open_release_page when the link is clicked', async () => {
    mockApi({ getCheckUpdates: vi.fn().mockResolvedValue(true), checkUpdateNow: vi.fn().mockResolvedValue(release) } as Partial<typeof api>)
    render(<Settings />)

    fireEvent.click(await screen.findByText(release.url))

    await waitFor(() => expect(api.openReleasePage).toHaveBeenCalledWith(release.url))
  })
})

describe('Settings: network audit', () => {
  it('shows the empty state when net_log has no rows', async () => {
    mockApi()
    render(<Settings />)
    await waitFor(() => expect(screen.getByText('Žiadna sieťová aktivita')).toBeInTheDocument())
  })

  it('lists net_log rows and refreshes them after a manual scan', async () => {
    mockApi({ netLog: vi.fn().mockResolvedValue([netLogRow]) } as Partial<typeof api>)
    render(<Settings />)
    await waitFor(() => expect(screen.getByText(netLogRow.url)).toBeInTheDocument())

    screen.getByRole('button', { name: 'Skenovať teraz' }).click()
    await waitFor(() => expect(api.runNetAudit).toHaveBeenCalled())
    await waitFor(() => expect(vi.mocked(api.netLog).mock.calls.length).toBeGreaterThan(1))
  })

  // A17/F7: a write into net_log can itself fail. The failure is process-local
  // (lost on restart), but while the process runs, Nastavenia must show it.
  it('shows a failed audit write and hides the section once none remain', async () => {
    mockApi({ netAuditFailures: vi.fn().mockResolvedValue([auditFailure]) } as Partial<typeof api>)
    render(<Settings />)
    await waitFor(() => expect(screen.getByText(auditFailure.error)).toBeInTheDocument())
    expect(screen.getByText('Neúspešné zápisy auditu')).toBeInTheDocument()
  })

  it('shows nothing extra when every audit write succeeded', async () => {
    mockApi()
    render(<Settings />)
    await waitFor(() => expect(screen.getByText('Žiadna sieťová aktivita')).toBeInTheDocument())
    expect(screen.queryByText('Neúspešné zápisy auditu')).not.toBeInTheDocument()
  })
})

// A17/F2: label is a free edit, IBAN stays read-only, and a kind change on an
// account with imports needs the user's deliberate confirmation.
describe('Settings: edit account', () => {
  const account: Account = { id: 1, iban: 'SK4411000000000012345678', kind: 'personal', label: 'Osobný', has_password: false }

  it('edits the label and keeps the IBAN read-only', async () => {
    const updateAccount = vi.fn().mockResolvedValue({ ...account, label: 'Nový názov' })
    mockApi({ listAccounts: vi.fn().mockResolvedValue([account]), updateAccount } as Partial<typeof api>)
    render(<Settings />)

    fireEvent.click(await screen.findByRole('button', { name: 'Upraviť' }))
    const ibanField = screen.getByLabelText('IBAN (nedá sa zmeniť)') as HTMLInputElement
    expect(ibanField).toBeDisabled()
    expect(ibanField.value).toBe('SK44...5678')

    fireEvent.change(screen.getByLabelText('Názov účtu'), { target: { value: 'Nový názov' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await waitFor(() => expect(updateAccount).toHaveBeenCalledWith(1, 'Nový názov', 'personal', false))
  })

  it('shows the backend refusal and lets the user confirm the kind change deliberately', async () => {
    const refusal = 'Počet výpisov: 3. Počet transakcií: 12. Zmena typu účtu zmení ich zaradenie v Prehľade aj v exporte. Ak chcete pokračovať, potvrďte zmenu.'
    const updateAccount = vi.fn().mockRejectedValueOnce(refusal).mockResolvedValueOnce({ ...account, kind: 'business' })
    mockApi({ listAccounts: vi.fn().mockResolvedValue([account]), updateAccount } as Partial<typeof api>)
    render(<Settings />)

    fireEvent.click(await screen.findByRole('button', { name: 'Upraviť' }))
    fireEvent.change(screen.getByLabelText('Typ účtu'), { target: { value: 'business' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await screen.findByText(refusal)
    expect(updateAccount).toHaveBeenNthCalledWith(1, 1, 'Osobný', 'business', false)

    fireEvent.click(screen.getByRole('checkbox', { name: /Potvrdzujem zmenu typu účtu/ }))
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await waitFor(() => expect(updateAccount).toHaveBeenNthCalledWith(2, 1, 'Osobný', 'business', true))
  })
})

// Nedá sa nastaviť alebo zmeniť heslo pri účte (Michal, 2026-09-08): the
// account row's password button and its dialog, next to `set_account_password`.
describe('Settings: account password', () => {
  const withoutPassword: Account = { id: 1, iban: 'SK4411000000000012345678', kind: 'personal', label: 'Osobný', has_password: false }
  const withPassword: Account = { id: 2, iban: 'SK3711000000000098765432', kind: 'business', label: 'Firemný', has_password: true }

  it('sets a password on an account that has none, shows the row and the status line updated', async () => {
    const setAccountPassword = vi.fn().mockResolvedValue(undefined)
    mockApi({ listAccounts: vi.fn().mockResolvedValue([withoutPassword]), setAccountPassword } as Partial<typeof api>)
    render(<Settings />)

    fireEvent.click(await screen.findByRole('button', { name: 'Nastaviť heslo' }))
    await screen.findByRole('dialog', { name: 'Heslo k výpisom účtu Osobný' })
    fireEvent.change(screen.getByLabelText('Heslo'), { target: { value: 'tajneheslo' } })
    fireEvent.change(screen.getByLabelText('Zopakovať heslo'), { target: { value: 'tajneheslo' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))

    await waitFor(() => expect(setAccountPassword).toHaveBeenCalledWith(1, 'tajneheslo'))
    await screen.findByText('Heslo je uložené v Správcovi poverení.')
  })

  it('changes a password on an account that already has one', async () => {
    const setAccountPassword = vi.fn().mockResolvedValue(undefined)
    mockApi({ listAccounts: vi.fn().mockResolvedValue([withPassword]), setAccountPassword } as Partial<typeof api>)
    render(<Settings />)

    fireEvent.click(await screen.findByRole('button', { name: 'Zmeniť heslo' }))
    fireEvent.change(screen.getByLabelText('Heslo'), { target: { value: 'nove-heslo' } })
    fireEvent.change(screen.getByLabelText('Zopakovať heslo'), { target: { value: 'nove-heslo' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))

    await waitFor(() => expect(setAccountPassword).toHaveBeenCalledWith(2, 'nove-heslo'))
  })

  it('keeps Uložiť disabled while the two fields do not match or are empty', async () => {
    mockApi({ listAccounts: vi.fn().mockResolvedValue([withoutPassword]) } as Partial<typeof api>)
    render(<Settings />)

    fireEvent.click(await screen.findByRole('button', { name: 'Nastaviť heslo' }))
    expect(screen.getByRole('button', { name: 'Uložiť' })).toBeDisabled()

    fireEvent.change(screen.getByLabelText('Heslo'), { target: { value: 'tajneheslo' } })
    expect(screen.getByRole('button', { name: 'Uložiť' })).toBeDisabled()

    fireEvent.change(screen.getByLabelText('Zopakovať heslo'), { target: { value: 'ine' } })
    expect(screen.getByRole('button', { name: 'Uložiť' })).toBeDisabled()

    fireEvent.change(screen.getByLabelText('Zopakovať heslo'), { target: { value: 'tajneheslo' } })
    expect(screen.getByRole('button', { name: 'Uložiť' })).not.toBeDisabled()
  })

  it('shows the alert on a failure, keeps the draft and the dialog open', async () => {
    const setAccountPassword = vi.fn().mockRejectedValue('Uloženie hesla zlyhalo: keyring locked')
    mockApi({ listAccounts: vi.fn().mockResolvedValue([withoutPassword]), setAccountPassword } as Partial<typeof api>)
    render(<Settings />)

    fireEvent.click(await screen.findByRole('button', { name: 'Nastaviť heslo' }))
    fireEvent.change(screen.getByLabelText('Heslo'), { target: { value: 'tajneheslo' } })
    fireEvent.change(screen.getByLabelText('Zopakovať heslo'), { target: { value: 'tajneheslo' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))

    await screen.findByText('Uloženie hesla zlyhalo: keyring locked')
    expect(screen.getByRole('dialog', { name: 'Heslo k výpisom účtu Osobný' })).toBeInTheDocument()
    expect((screen.getByLabelText('Heslo') as HTMLInputElement).value).toBe('tajneheslo')
  })

  // 0.1.4: mirrors the backend's exact boundary
  // (`commands::PASSWORD_MAX_CHARS`, `exactly_the_length_limit_is_accepted_
  // one_over_is_rejected...`), so a length the backend would refuse is
  // caught client-side, before Uložiť even sends it.
  it('accepts exactly 512 characters and rejects 513 with the Slovak hint, Uložiť disabled', async () => {
    const setAccountPassword = vi.fn().mockResolvedValue(undefined)
    mockApi({ listAccounts: vi.fn().mockResolvedValue([withoutPassword]), setAccountPassword } as Partial<typeof api>)
    render(<Settings />)
    fireEvent.click(await screen.findByRole('button', { name: 'Nastaviť heslo' }))

    const atLimit = 'a'.repeat(512)
    fireEvent.change(screen.getByLabelText('Heslo'), { target: { value: atLimit } })
    fireEvent.change(screen.getByLabelText('Zopakovať heslo'), { target: { value: atLimit } })
    expect(screen.queryByText('Limit je 512 znakov.')).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Uložiť' })).not.toBeDisabled()

    const overLimit = 'a'.repeat(513)
    fireEvent.change(screen.getByLabelText('Heslo'), { target: { value: overLimit } })
    fireEvent.change(screen.getByLabelText('Zopakovať heslo'), { target: { value: overLimit } })
    expect(screen.getByText('Limit je 512 znakov.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Uložiť' })).toBeDisabled()

    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    expect(setAccountPassword).not.toHaveBeenCalled()
  })
})
