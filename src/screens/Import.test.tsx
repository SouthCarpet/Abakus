import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { ImportReport } from '../api'
import { ImportResultCard } from './Import'
const base: ImportReport = { path: 'C:/x/vypis.pdf', status: 'imported', accountLabel: 'Osobný', accountKind: 'personal', ibanMasked: 'SK44...5678', iban: 'SK44', statementNumber: 6, periodStart: '2026-05-30', periodEnd: '2026-06-30', inserted: 8, duplicates: 0, checksum: { status: 'ok' }, warnings: [], message: null }
describe('ImportResultCard', () => {
  it('shows account, period, counts and checksum', () => {
    render(<ImportResultCard report={base} onPassword={vi.fn()} onAddAccount={vi.fn()} />)
    expect(screen.getByText('Osobný')).toBeInTheDocument(); expect(screen.getByText(/8 nových/)).toBeInTheDocument(); expect(screen.getByText('Kontrolný súčet sedí')).toBeInTheDocument()
  })
  it('offers a password button for a locked file', () => {
    render(<ImportResultCard report={{ ...base, status: 'locked', checksum: null }} onPassword={vi.fn()} onAddAccount={vi.fn()} />)
    expect(screen.getByRole('button', { name: 'Zadať heslo' })).toBeInTheDocument()
  })
  it('offers to add an unknown account with its kind prefilled', () => {
    const onAdd = vi.fn()
    render(<ImportResultCard report={{ ...base, status: 'unknown_account', accountLabel: null }} onPassword={vi.fn()} onAddAccount={onAdd} />)
    screen.getByRole('button', { name: 'Pridať účet' }).click()
    expect(onAdd).toHaveBeenCalledWith('SK44', 'personal')
  })
})
