import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { BadChecksum } from '../api'
import { ChecksumBanner } from './Overview'

afterEach(() => cleanup())

describe('ChecksumBanner', () => {
  it('renders the mismatch message and navigates to the statement on click', () => {
    const row: BadChecksum = { statement_id: 9, number: 4, account_label: 'Osobný', off_by_cents: -150 }
    const onNavigate = vi.fn()
    render(<ChecksumBanner row={row} onNavigate={onNavigate} />)
    expect(screen.getByText('Výpis č. 4 (účet Osobný) nesedí o 1,50 €')).toBeInTheDocument()
    screen.getByRole('button', { name: 'Zobraziť transakcie' }).click()
    expect(onNavigate).toHaveBeenCalledWith(9)
  })
})
