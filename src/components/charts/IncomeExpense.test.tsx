import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { INCOME_EXPENSE_SERIES, IncomeExpense } from './IncomeExpense'

afterEach(cleanup)

// Point 13: fee_cents already exists on Summary.by_month; the chart must
// surface it as its own series, not just income and expense.
describe('IncomeExpense fee series', () => {
  it('renders a Poplatky series alongside Príjem and Výdavky', () => {
    render(
      <IncomeExpense
        rows={[{ month: '2026-06', income_cents: 100000, expense_cents: 50000, fee_cents: 500 }]}
        onMonthClick={vi.fn()}
      />,
    )
    expect(screen.getByText('Príjem')).toBeInTheDocument()
    expect(screen.getByText('Výdavky')).toBeInTheDocument()
    expect(screen.getByText('Poplatky')).toBeInTheDocument()
  })
})

// Oracle: dataviz.md "The one-hue ramp": series identity is the dash pattern, with the lightness
// step only ordering the series, and a legend sample shows each series' own dash. Finding V8: the
// three bar series were three near-identical greens.
describe('IncomeExpense series encoding', () => {
  const rows = [{ month: '2026-06', income_cents: 100000, expense_cents: 50000, fee_cents: 500 }]

  it('gives every series its own step, which picks both its colour and its dash pattern', () => {
    expect(new Set(INCOME_EXPENSE_SERIES.map((s) => s.step)).size).toBe(3)
    expect(INCOME_EXPENSE_SERIES.map((s) => s.name)).toEqual(['Príjem', 'Výdavky', 'Poplatky'])
  })

  it('draws a legend sample per series with the same colour and dash tokens', () => {
    const { container } = render(<IncomeExpense rows={rows} onMonthClick={vi.fn()} />)
    const items = [...container.querySelectorAll('.k-legend-item')] as HTMLElement[]
    expect(items.map((item) => item.textContent)).toEqual(['Príjem', 'Výdavky', 'Poplatky'])
    const tokens = items.map((item) => [item.style.getPropertyValue('--k-legend-color'), item.style.getPropertyValue('--k-legend-dash')])
    expect(tokens).toEqual(INCOME_EXPENSE_SERIES.map((s) => [`var(--color-chart-${s.step})`, `var(--chart-dash-${s.step})`]))
    expect(items.every((item) => item.querySelector('svg.k-legend-swatch line'))).toBe(true)
  })
})
