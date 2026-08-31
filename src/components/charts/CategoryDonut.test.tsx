import { cleanup, fireEvent, render } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { CategoryDonut } from './CategoryDonut'

afterEach(() => cleanup())

const rows = [
  { month: '2026-01', category_id: 1, name: 'Jedlo', cents: 800 },
  { month: '2026-01', category_id: 2, name: 'Doprava', cents: 700 },
  { month: '2026-01', category_id: 3, name: 'Bývanie', cents: 600 },
  { month: '2026-01', category_id: 4, name: 'Zábava', cents: 500 },
  { month: '2026-01', category_id: 5, name: 'Zdravie', cents: 400 },
  { month: '2026-01', category_id: 6, name: 'Oblečenie', cents: 300 },
  { month: '2026-01', category_id: 7, name: 'Dary', cents: 200 },
  { month: '2026-01', category_id: 8, name: 'Iné', cents: 100 },
]

function sectors() {
  return document.querySelectorAll('.recharts-pie-sector path')
}

describe('CategoryDonut', () => {
  it('calls onCategoryClick with the category_id of the clicked slice', () => {
    const onCategoryClick = vi.fn()
    render(<CategoryDonut rows={rows} onCategoryClick={onCategoryClick} />)
    fireEvent.click(sectors()[0])
    expect(onCategoryClick).toHaveBeenCalledWith(1)
  })

  it('does not call onCategoryClick when the Ostatné slice is clicked', () => {
    const onCategoryClick = vi.fn()
    render(<CategoryDonut rows={rows} onCategoryClick={onCategoryClick} />)
    const ostatneCell = document.querySelector('.k-chart-ostatne')
    expect(ostatneCell).not.toBeNull()
    fireEvent.click(ostatneCell as Element)
    expect(onCategoryClick).not.toHaveBeenCalled()
  })
})
