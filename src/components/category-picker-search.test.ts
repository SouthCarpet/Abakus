import { describe, expect, it } from 'vitest'
import type { Category } from '../api'
import { groupCategories } from '../lib/categories'
import { filterGroupsByQuery, flattenChoices, foldLabel, selectedLabel } from './category-picker-search'

const parent: Category = { id: 10, parent_id: null, name: 'Jedlo', kind: 'expense', sort: 0, system: false, archived: false }
const child: Category = { ...parent, id: 11, parent_id: 10, name: 'Potraviny' }
const leaf: Category = { ...parent, id: 12, name: 'Doprava' }
const housing: Category = { ...parent, id: 13, name: 'Bývanie' }

describe('foldLabel', () => {
  it('strips Slovak diacritics, case and extra spaces', () => {
    expect(foldLabel('  BÝVANIE  ')).toBe('byvanie')
    expect(foldLabel('Potraviny')).toBe('potraviny')
  })
})

describe('filterGroupsByQuery', () => {
  const groups = groupCategories([parent, child, leaf, housing])

  it('returns every group when the query is empty', () => {
    expect(filterGroupsByQuery(groups, '  ').map((g) => g.parent.name)).toEqual(['Jedlo', 'Doprava', 'Bývanie'])
  })

  it('keeps parent context for a child hit and drops empty groups', () => {
    const visible = filterGroupsByQuery(groups, 'potraviny')
    expect(visible).toHaveLength(1)
    expect(visible[0].parent.name).toBe('Jedlo')
    expect(visible[0].subs.map((s) => s.name)).toEqual(['Potraviny'])
  })
})

describe('flattenChoices', () => {
  it('omits a parent with children in assignment and includes it in filter', () => {
    const groups = groupCategories([parent, child])
    expect(flattenChoices(groups, 'assignment', false, false)).toEqual([{ kind: 'category', id: 11 }])
    expect(flattenChoices(groups, 'filter', true, false)).toEqual([
      { kind: 'empty' },
      { kind: 'category', id: 10 },
      { kind: 'category', id: 11 },
    ])
  })
})

describe('selectedLabel', () => {
  it('keeps an unknown filter id from looking like the empty label', () => {
    expect(selectedLabel(99, 'Všetky kategórie', [leaf], false)).toBe('Kategória ID 99 (názov nie je dostupný)')
  })
})
