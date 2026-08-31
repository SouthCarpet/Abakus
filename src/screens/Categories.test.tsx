import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { RulesTable } from './Categories'
describe('RulesTable', () => {
  it('labels rule kinds in Slovak and hides delete for seed rules', () => {
    const rules = [
      { id: 1, kind: 'exact', key: 'aldi sued', place: 'neuss', category_id: 2, category_name: 'potraviny', parent_name: 'Jedlo', hit_count: 3 },
      { id: 2, kind: 'seed', key: 'lidl', place: null, category_id: 2, category_name: 'potraviny', parent_name: 'Jedlo', hit_count: 0 },
    ] as const
    render(<RulesTable rules={[...rules]} onDelete={vi.fn()} />)
    expect(screen.getByText('presné')).toBeInTheDocument(); expect(screen.getByText('slovník')).toBeInTheDocument()
    expect(screen.getAllByRole('button', { name: 'Zmazať' })).toHaveLength(1)
  })
})
