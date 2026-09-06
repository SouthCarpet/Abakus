import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { CategoryBars } from './CategoryBars'

afterEach(cleanup)

// Oracle: binding visual-corrections brief, defect 3; synthetic cents retain net signs.
function row(category_id: number, name: string, cents: number, month = '2026-01') {
  return { category_id, name, cents, month }
}

function mark(label: string) {
  return screen.getByRole('img', { name: label }).querySelector('rect')!
}

describe('signed category bars', () => {
  it('plots +150000 and -130000 cents on opposite sides of zero with signed labels', () => {
    render(<CategoryBars rows={[row(1, 'Jedlo', 150000), row(2, 'Doprava', -130000)]} onCategoryClick={vi.fn()} />)
    const positive = mark('Jedlo · 1 500,00 €')
    const negative = mark('Doprava · -1 300,00 €')
    expect(Number(positive.getAttribute('x'))).toBe(50)
    expect(Number(positive.getAttribute('width'))).toBe(50)
    expect(Number(negative.getAttribute('x'))).toBeLessThan(50)
    expect(Number(negative.getAttribute('width'))).toBeCloseTo(43.333333)
    expect(Number(negative.getAttribute('x')) + Number(negative.getAttribute('width'))).toBeCloseTo(50)
  })

  it('renders a visible negative mark when the only total is -97301 cents', () => {
    render(<CategoryBars rows={[row(4, 'Vrátené platby', -97301)]} onCategoryClick={vi.fn()} />)
    expect(mark('Vrátené platby · -973,01 €')).toHaveAttribute('width', '50')
    expect(mark('Vrátené platby · -973,01 €')).toHaveAttribute('x', '0')
  })

  it('nets +10001 and -15002 monthly cents to -5001 without changing the input', () => {
    const rows = Object.freeze([
      Object.freeze(row(3, 'Jedlo', 10001)),
      Object.freeze(row(3, 'Jedlo', -15002, '2026-02')),
    ])
    render(<CategoryBars rows={[...rows]} onCategoryClick={vi.fn()} />)
    expect(screen.getByRole('button', { name: 'Jedlo · -50,01 €' })).toBeInTheDocument()
    expect(screen.getAllByRole('button')).toHaveLength(1)
    expect(rows[0].cents).toBe(10001)
    expect(rows[1].cents).toBe(-15002)
  })

  it('keeps an eighth negative category separate from seven positive categories', () => {
    render(<CategoryBars rows={[
      row(1, 'A', 700), row(2, 'B', 600), row(3, 'C', 500), row(4, 'D', 400),
      row(5, 'E', 300), row(6, 'F', 200), row(7, 'G', 100), row(8, 'Refund', -100),
    ]} onCategoryClick={vi.fn()} />)
    expect(screen.getAllByRole('button')).toHaveLength(8)
    expect(screen.getByRole('button', { name: 'G · 1,00 €' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Refund · -1,00 €' })).toBeInTheDocument()
  })

  it('shows an explicit zero state for a zero-cent category', () => {
    render(<CategoryBars rows={[row(3, 'Jedlo', 0)]} onCategoryClick={vi.fn()} />)
    expect(screen.getByText('Súčty všetkých kategórií sú nulové.')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Jedlo · 0,00 €' })).toBeInTheDocument()
    expect(mark('Jedlo · 0,00 €')).toHaveAttribute('width', '0')
  })

  it('shows a no-data message for no category rows', () => {
    render(<CategoryBars rows={[]} onCategoryClick={vi.fn()} />)
    expect(screen.getByText('Za vybrané obdobie nie sú údaje podľa kategórií.')).toBeInTheDocument()
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
  })

  it.each([
    { cents: 150000, label: 'Jedlo · 1 500,00 €' },
    { cents: -130000, label: 'Jedlo · -1 300,00 €' },
    { cents: 0, label: 'Jedlo · 0,00 €' },
  ])('clicking a $cents-cent category navigates to its id 42', ({ cents, label }) => {
    const onCategoryClick = vi.fn()
    render(<CategoryBars rows={[row(42, 'Jedlo', cents)]} onCategoryClick={onCategoryClick} />)
    fireEvent.click(screen.getByRole('button', { name: label }))
    expect(onCategoryClick).toHaveBeenCalledExactlyOnceWith(42)
  })
})
