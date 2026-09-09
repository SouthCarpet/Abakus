import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Account } from '../api'
import { api } from '../api'
import { SetPasswordDialog } from './SetPasswordDialog'

afterEach(() => cleanup())
beforeEach(() => vi.clearAllMocks())

vi.mock('../api', () => ({
  api: { setAccountPassword: vi.fn() },
}))

const account: Account = { id: 1, iban: 'SK44', kind: 'personal', label: 'Osobný', has_password: false }

describe('SetPasswordDialog', () => {
  // Both fields need their own toggle, not one shared state: mixing up
  // "Heslo" and "Zopakovať heslo" while confirming a typo would be worse
  // than the native icon this replaces.
  it('gives each password field its own toggle', () => {
    render(<SetPasswordDialog account={account} onClose={vi.fn()} onSaved={vi.fn()} />)
    const toggles = screen.getAllByRole('button', { name: 'Zobraziť' })
    expect(toggles).toHaveLength(2)

    fireEvent.click(toggles[0])
    const passwordInputs = Array.from(document.querySelectorAll('.k-password-row input')) as HTMLInputElement[]
    expect(passwordInputs[0].type).toBe('text')
    expect(passwordInputs[1].type).toBe('password')
  })

  it('still requires the two fields to match before Uložiť is enabled, toggle or not', () => {
    render(<SetPasswordDialog account={account} onClose={vi.fn()} onSaved={vi.fn()} />)
    const [heslo, zopakovat] = Array.from(document.querySelectorAll('.k-password-row input')) as HTMLInputElement[]
    const save = screen.getByRole('button', { name: 'Uložiť' })
    expect(save).toBeDisabled()

    fireEvent.click(screen.getAllByRole('button', { name: 'Zobraziť' })[0])
    fireEvent.change(heslo, { target: { value: 'tajne' } })
    expect(save).toBeDisabled()
    fireEvent.change(zopakovat, { target: { value: 'tajne' } })
    expect(save).not.toBeDisabled()
  })

  it('calls setAccountPassword with the typed value on save', async () => {
    vi.mocked(api.setAccountPassword).mockResolvedValue(undefined)
    const onSaved = vi.fn().mockResolvedValue(undefined)
    render(<SetPasswordDialog account={account} onClose={vi.fn()} onSaved={onSaved} />)
    const [heslo, zopakovat] = Array.from(document.querySelectorAll('.k-password-row input')) as HTMLInputElement[]
    fireEvent.change(heslo, { target: { value: 'tajne123' } })
    fireEvent.change(zopakovat, { target: { value: 'tajne123' } })
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť' }))
    await screen.findByText('Heslo je uložené v Správcovi poverení.')
    expect(api.setAccountPassword).toHaveBeenCalledWith(1, 'tajne123')
  })
})
