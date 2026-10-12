import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MonthlyStacked } from './MonthlyStacked'

afterEach(cleanup)

// Oracle: the r3 visual audit, finding V4, and components.md "States": a chart with no data shows
// the empty sentence inside its panel, not an empty frame.
describe('MonthlyStacked empty state', () => {
  it('shows the empty sentence and no chart frame when there is no category data', () => {
    const { container } = render(<MonthlyStacked rows={[]} onCategoryClick={vi.fn()} />)
    expect(screen.getByText('Za vybrané obdobie nie sú údaje podľa kategórií a mesiacov.')).toBeInTheDocument()
    expect(container.querySelector('.recharts-responsive-container')).toBeNull()
  })

  it('keeps the chart frame when there is data', () => {
    const { container } = render(<MonthlyStacked rows={[{ month: '2026-06', category_id: 1, name: 'Jedlo', cents: -500 }]} onCategoryClick={vi.fn()} />)
    expect(container.querySelector('.recharts-responsive-container')).not.toBeNull()
    expect(screen.queryByText(/nie sú údaje/)).toBeNull()
  })
})
