import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Category } from '../api'
import { CategoryPicker } from './CategoryPicker'

afterEach(cleanup)

const leaf: Category = { id: 12, parent_id: null, name: 'Doprava', kind: 'expense', sort: 0, system: false, archived: false }

// Audit 091-r4-a4 finding 6: the trigger shows the expand mark, as the r4 gallery does (R28).
describe('category picker expand mark', () => {
  it('draws a chevron inside the trigger, hidden from the screen reader, closed and open', () => {
    render(<CategoryPicker value={null} categories={[leaf]} onChange={vi.fn()} />)
    const trigger = screen.getByRole('combobox', { name: 'Kategória' })
    const chevron = trigger.querySelector('svg.category-picker__chevron')
    expect(chevron).not.toBeNull()
    expect(chevron).toHaveAttribute('aria-hidden', 'true')
    // Oracle: A2 requires the sprite chevron, small beside body text.
    expect(chevron).toHaveClass('k-icon', 'k-icon--sm')
    expect(chevron?.querySelector('use')).toHaveAttribute('href', '/kaliber-icons.svg#k-icon-chevron-down')
    fireEvent.click(trigger)
    expect(trigger.querySelector('svg.category-picker__chevron')).not.toBeNull()
  })

  it('keeps the visible text of the trigger unchanged', () => {
    render(<CategoryPicker value={12} categories={[leaf]} onChange={vi.fn()} />)
    expect(screen.getByRole('combobox', { name: 'Kategória' }).textContent).toBe('Doprava')
  })
})
