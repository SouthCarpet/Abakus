import { cleanup, render } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { CategoryBars } from './CategoryBars'
import { IncomeExpense } from './IncomeExpense'
import { MonthlyStacked } from './MonthlyStacked'

afterEach(cleanup)

// Audit 091-r4-a4 finding 5 (SLOP-LAYOUT-02, block): the card around a chart is the one frame.
// The data area inside it has no second border and no raised surface; space separates it.
const monthly = [{ month: '2026-06', category_id: 1, name: 'Jedlo', cents: -500 }]
const byMonth = [{ month: '2026-06', income_cents: 100000, expense_cents: 50000, fee_cents: 500 }]

describe('chart data areas inside their card', () => {
  it.each([
    ['MonthlyStacked', () => <MonthlyStacked rows={monthly} onCategoryClick={vi.fn()} />],
    ['IncomeExpense', () => <IncomeExpense rows={byMonth} onMonthClick={vi.fn()} />],
    ['CategoryBars', () => <CategoryBars rows={monthly} onCategoryClick={vi.fn()} />],
  ])('%s draws no second frame', (_name, view) => {
    const { container } = render(view())
    expect(container.querySelector('.k-well, .k-milled, .k-card, [class*="border"]')).toBeNull()
  })
})
