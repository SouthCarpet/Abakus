import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Category } from '../api'
import { CategoryPicker } from './CategoryPicker'

afterEach(cleanup)

const parent: Category = { id: 10, parent_id: null, name: 'Jedlo', kind: 'expense', sort: 0, system: false, archived: false }
const child: Category = { ...parent, id: 11, parent_id: 10, name: 'Potraviny' }
const leaf: Category = { ...parent, id: 12, name: 'Doprava' }

// Oracle: binding parent-filter brief requires parent filters while preserving assignment choices.
describe('category filter choices', () => {
  it('offers parent 10 and child 11 and emits the selected parent ID', () => {
    const onChange = vi.fn()
    render(<CategoryPicker mode="filter" value={null} categories={[parent, child]} onChange={onChange} />)
    expect(screen.getByRole('option', { name: 'Jedlo' })).toHaveValue('10')
    expect(screen.getByRole('option', { name: 'Potraviny' })).toHaveValue('11')
    fireEvent.change(screen.getByRole('combobox'), { target: { value: '10' } })
    expect(onChange).toHaveBeenCalledExactlyOnceWith(10)
  })

  it.each(['filter', 'assignment'] as const)('keeps leaf 12 selectable in %s mode', (mode) => {
    const onChange = vi.fn()
    render(<CategoryPicker mode={mode} value={12} categories={[leaf]} onChange={onChange} />)
    expect(screen.getByRole('combobox')).toHaveDisplayValue('Doprava')
    fireEvent.change(screen.getByRole('combobox'), { target: { value: '12' } })
    expect(onChange).toHaveBeenCalledExactlyOnceWith(12)
  })

  it('keeps the default assignment picker child-only for a parent with children', () => {
    const onChange = vi.fn()
    render(<CategoryPicker value={11} categories={[parent, child, leaf]} onChange={onChange} />)
    expect(screen.queryByRole('option', { name: 'Jedlo' })).not.toBeInTheDocument()
    expect(screen.getByRole('combobox')).toHaveDisplayValue('Potraviny')
    fireEvent.change(screen.getByRole('combobox'), { target: { value: '11' } })
    expect(onChange).toHaveBeenCalledExactlyOnceWith(11)
  })

  it('retains an archived parent ID with an explicit disabled label', () => {
    render(<CategoryPicker mode="filter" value={10} categories={[{ ...parent, archived: true }, child, leaf]} onChange={vi.fn()} />)
    expect(screen.getByRole('combobox')).toHaveValue('10')
    expect(screen.getByRole('combobox')).toHaveDisplayValue('Jedlo (archivovaná)')
    expect(screen.getByRole('option', { name: 'Jedlo (archivovaná)' })).toBeDisabled()
    expect(screen.queryByRole('option', { name: 'Potraviny' })).not.toBeInTheDocument()
  })

  it('does not offer archived categories when no filter is selected', () => {
    render(<CategoryPicker mode="filter" value={null} categories={[{ ...parent, archived: true }, leaf]} onChange={vi.fn()} />)
    expect(screen.queryByRole('option', { name: /Jedlo/ })).not.toBeInTheDocument()
    expect(screen.getByRole('option', { name: 'Doprava' })).toBeEnabled()
  })

  it('keeps missing ID 99 visible instead of claiming all categories', () => {
    render(<CategoryPicker mode="filter" emptyLabel="Všetky kategórie" value={99} categories={[leaf]} onChange={vi.fn()} />)
    expect(screen.getByRole('combobox')).toHaveValue('99')
    expect(screen.getByRole('combobox')).toHaveDisplayValue('Kategória ID 99 (názov nie je dostupný)')
    expect(screen.getByRole('option', { name: /Kategória ID 99/ })).toBeDisabled()
  })

  it('labels an active child of an archived parent as unavailable while retaining its filter', () => {
    render(<CategoryPicker mode="filter" value={11} categories={[{ ...parent, archived: true }, child]} onChange={vi.fn()} />)
    expect(screen.getByRole('combobox')).toHaveValue('11')
    expect(screen.getByRole('combobox')).toHaveDisplayValue('Potraviny (nedostupná)')
  })
})
