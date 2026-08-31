import { describe, expect, it } from 'vitest'
import { categoryLabel, groupCategories, statusLabel } from './categories'

const cats = [
  { id: 1, parent_id: null, name: 'Jedlo', kind: 'expense', sort: 1, system: false, archived: false },
  { id: 2, parent_id: 1, name: 'potraviny', kind: 'expense', sort: 0, system: false, archived: false },
  { id: 3, parent_id: 1, name: 'stará', kind: 'expense', sort: 1, system: false, archived: true },
  { id: 4, parent_id: null, name: 'Bývanie', kind: 'expense', sort: 0, system: false, archived: false },
] as const

describe('categories', () => {
  it('groups subs under parents, hides archived, sorts parents', () => {
    const g = groupCategories([...cats])
    expect(g.map((x) => x.parent.name)).toEqual(['Bývanie', 'Jedlo'])
    expect(g[1].subs.map((s) => s.name)).toEqual(['potraviny'])
  })
  it('labels rows', () => {
    expect(categoryLabel({ parent_name: 'Jedlo', category_name: 'potraviny' } as never)).toBe('Jedlo / potraviny')
    expect(categoryLabel({ parent_name: null, category_name: null } as never)).toBe('Nezaradené')
    expect(statusLabel('suggested')).toBe('Odhad')
  })
})
