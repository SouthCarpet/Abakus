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

// Oracle: dataviz.md "The one-hue ramp" and finding N1 of the r3 re-verification: four stacked
// category slices were one dark-green mass. Each series needs its own step (fill lightness) and
// its own dash, and the legend swatch draws the same tokens in the same order.
describe('MonthlyStacked series encoding', () => {
  const names = ['Bývanie', 'Doprava', 'Poplatky', 'Potraviny', 'Zábava', 'Zdravie']
  const rows = names.map((name, i) => ({ month: '2026-06', category_id: i + 1, name, cents: -(100 + i * 100) }))

  it('gives each of six series its own fill step and dash, in order', () => {
    const { container } = render(<MonthlyStacked rows={rows} onCategoryClick={vi.fn()} />)
    const slices = [...container.querySelectorAll('.recharts-bar')].map((bar) => bar.querySelector('path, rect, .recharts-rectangle'))
    expect(slices).toHaveLength(6)
    const attrs = slices.map((el) => [el?.getAttribute('fill'), el?.getAttribute('stroke-dasharray')])
    expect(attrs).toEqual([1, 2, 3, 4, 5, 6].map((n) => [`var(--color-chart-${n})`, `var(--chart-dash-${n})`]))
    expect(new Set(attrs.map((a) => a[0])).size).toBe(6)
    expect(new Set(attrs.map((a) => a[1])).size).toBe(6)
  })

  it('draws a legend swatch per series with the same colour and dash tokens', () => {
    const { container } = render(<MonthlyStacked rows={rows} onCategoryClick={vi.fn()} />)
    const items = [...container.querySelectorAll('.k-legend-item')] as HTMLElement[]
    expect(items.map((item) => item.textContent)).toEqual(names)
    const tokens = items.map((item) => [item.style.getPropertyValue('--k-legend-color'), item.style.getPropertyValue('--k-legend-dash')])
    expect(tokens).toEqual([1, 2, 3, 4, 5, 6].map((n) => [`var(--color-chart-${n})`, `var(--chart-dash-${n})`]))
    expect(items.every((item) => item.querySelector('svg.k-legend-swatch line'))).toBe(true)
  })

  it('draws the folded Ostatné series muted with a solid legend line', () => {
    const many = [...rows, { month: '2026-06', category_id: 7, name: 'Iné', cents: -10 }]
    const { container } = render(<MonthlyStacked rows={many} onCategoryClick={vi.fn()} />)
    const items = [...container.querySelectorAll('.k-legend-item')] as HTMLElement[]
    const last = items[items.length - 1]
    expect(last.textContent).toBe('Ostatné')
    expect([last.style.getPropertyValue('--k-legend-color'), last.style.getPropertyValue('--k-legend-dash')]).toEqual(['var(--color-border)', 'none'])
  })
})
