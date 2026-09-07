import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { RecurringKpis } from './RecurringKpis'
import { R14_OVERVIEW } from './oracles'

afterEach(() => cleanup())

describe('RecurringKpis R14 literal totals', () => {
  it('shows confirmed monthly expense 53,67 €, income 2 500,00 € and net 2 446,33 €', () => {
    // Oracle: recurring-acceptance R14 monthly confirmed 5367 / 250000 / 244633 cents.
    render(<RecurringKpis overview={R14_OVERVIEW} today="2026-09-01" annual={false} onAnnualChange={() => {}} />)
    expect(screen.getByText('Pravidelné výdavky za mesiac')).toBeInTheDocument()
    expect(screen.getByText('53,67 €')).toBeInTheDocument()
    expect(screen.getByText('Pravidelné príjmy za mesiac')).toBeInTheDocument()
    expect(screen.getByText('2 500,00 €')).toBeInTheDocument()
    expect(screen.getByText('Voľné po pravidelných platbách')).toBeInTheDocument()
    expect(screen.getByText('2 446,33 €')).toBeInTheDocument()
  })

  it('keeps candidate 5,00 € in Ďalšie odhady, not inside confirmed expense', () => {
    render(<RecurringKpis overview={R14_OVERVIEW} today="2026-09-01" annual={false} onAnnualChange={() => {}} />)
    expect(screen.getByText('Ďalšie odhady 5,00 €')).toBeInTheDocument()
  })

  it('shows remaining charges 12,00 € and receipts 2 500,00 € for the as_of month', () => {
    render(<RecurringKpis overview={R14_OVERVIEW} today="2026-09-01" annual={false} onAnnualChange={() => {}} />)
    expect(screen.getByText('Ešte príde tento mesiac')).toBeInTheDocument()
    expect(screen.getByText('Potvrdené výdavky 12,00 €')).toBeInTheDocument()
    expect(screen.getByText('Potvrdené príjmy 2 500,00 €')).toBeInTheDocument()
    expect(screen.getByText('Ďalšie odhady výdavkov 5,00 €')).toBeInTheDocument()
  })

  it('switches projected totals to the R14 annual figures and leaves remaining unchanged', () => {
    // Oracle: R14 annual 64404 / 3000000 / 2935596; remaining stays 1200 / 250000.
    const onAnnualChange = vi.fn()
    render(<RecurringKpis overview={R14_OVERVIEW} today="2026-09-01" annual={false} onAnnualChange={onAnnualChange} />)
    fireEvent.click(screen.getByRole('button', { name: 'Za rok' }))
    expect(onAnnualChange).toHaveBeenCalledWith(true)
  })

  it('renders annual projected numbers when annual is true and keeps remaining 12,00 €', () => {
    render(<RecurringKpis overview={R14_OVERVIEW} today="2026-09-01" annual onAnnualChange={() => {}} />)
    expect(screen.getByText('Pravidelné výdavky za rok')).toBeInTheDocument()
    expect(screen.getByText('644,04 €')).toBeInTheDocument()
    expect(screen.getByText('30 000,00 €')).toBeInTheDocument()
    expect(screen.getByText('29 355,96 €')).toBeInTheDocument()
    expect(screen.getByText('Potvrdené výdavky 12,00 €')).toBeInTheDocument()
    expect(screen.getByText('Potvrdené príjmy 2 500,00 €')).toBeInTheDocument()
  })

  it('states the R15 488 bps share and July/August denominator months', () => {
    render(<RecurringKpis overview={R14_OVERVIEW} today="2026-09-01" annual={false} onAnnualChange={() => {}} />)
    expect(screen.getByText('4,88 % priemerných mesačných výdavkov 1 100,00 € (2026-07, 2026-08).')).toBeInTheDocument()
  })

  it('names a historical remaining month instead of this month', () => {
    const historical = { ...R14_OVERVIEW, as_of: '2026-08-31', unfinished_period: false }
    render(<RecurringKpis overview={historical} today="2026-09-07" annual={false} onAnnualChange={() => {}} />)
    expect(screen.getByText('Zostávalo v auguste 2026')).toBeInTheDocument()
    expect(screen.queryByText('Ešte príde tento mesiac')).not.toBeInTheDocument()
  })
})
