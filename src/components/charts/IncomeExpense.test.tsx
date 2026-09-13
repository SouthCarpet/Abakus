import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { IncomeExpense } from './IncomeExpense'

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
