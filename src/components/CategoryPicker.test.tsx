import { cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Category } from '../api'
import { CategoryPicker } from './CategoryPicker'

afterEach(cleanup)

const parent: Category = { id: 10, parent_id: null, name: 'Jedlo', kind: 'expense', sort: 0, system: false, archived: false }
const child: Category = { ...parent, id: 11, parent_id: 10, name: 'Potraviny' }
const leaf: Category = { ...parent, id: 12, name: 'Doprava' }
const housing: Category = { ...parent, id: 13, name: 'Bývanie' }

function openPicker(name = 'Kategória') {
  fireEvent.click(screen.getByRole('combobox', { name }))
}

function searchbox() {
  return screen.getByRole('searchbox', { name: 'Hľadať kategóriu' })
}

// Oracle: binding parent-filter brief requires parent filters while preserving assignment choices.
describe('category filter choices', () => {
  it('offers parent 10 and child 11 and emits the selected parent ID', () => {
    const onChange = vi.fn()
    render(<CategoryPicker mode="filter" value={null} categories={[parent, child]} onChange={onChange} />)
    openPicker()
    expect(screen.getByRole('option', { name: 'Jedlo' })).toBeInTheDocument()
    expect(screen.getByRole('option', { name: 'Potraviny' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('option', { name: 'Jedlo' }))
    expect(onChange).toHaveBeenCalledExactlyOnceWith(10)
  })

  it.each(['filter', 'assignment'] as const)('keeps leaf 12 selectable in %s mode', (mode) => {
    const onChange = vi.fn()
    render(<CategoryPicker mode={mode} value={12} categories={[leaf]} onChange={onChange} />)
    expect(screen.getByRole('combobox', { name: 'Kategória' })).toHaveTextContent('Doprava')
    openPicker()
    fireEvent.click(screen.getByRole('option', { name: 'Doprava' }))
    expect(onChange).toHaveBeenCalledExactlyOnceWith(12)
  })

  it('keeps the default assignment picker child-only for a parent with children', () => {
    const onChange = vi.fn()
    render(<CategoryPicker value={11} categories={[parent, child, leaf]} onChange={onChange} />)
    expect(screen.getByRole('combobox', { name: 'Kategória' })).toHaveTextContent('Potraviny')
    openPicker()
    expect(screen.queryByRole('option', { name: 'Jedlo' })).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole('option', { name: 'Potraviny' }))
    expect(onChange).toHaveBeenCalledExactlyOnceWith(11)
  })

  it('retains an archived parent ID with an explicit unavailable label', () => {
    render(<CategoryPicker mode="filter" value={10} categories={[{ ...parent, archived: true }, child, leaf]} onChange={vi.fn()} />)
    expect(screen.getByRole('combobox', { name: 'Kategória' })).toHaveTextContent('Jedlo (archivovaná)')
    openPicker()
    expect(screen.queryByRole('option', { name: 'Potraviny' })).not.toBeInTheDocument()
  })

  it('does not offer archived categories when no filter is selected', () => {
    render(<CategoryPicker mode="filter" value={null} categories={[{ ...parent, archived: true }, leaf]} onChange={vi.fn()} />)
    openPicker()
    expect(screen.queryByRole('option', { name: /Jedlo/ })).not.toBeInTheDocument()
    expect(screen.getByRole('option', { name: 'Doprava' })).toBeEnabled()
  })

  it('keeps missing ID 99 visible instead of claiming all categories', () => {
    render(<CategoryPicker mode="filter" emptyLabel="Všetky kategórie" value={99} categories={[leaf]} onChange={vi.fn()} />)
    expect(screen.getByRole('combobox', { name: 'Kategória' })).toHaveTextContent('Kategória ID 99 (názov nie je dostupný)')
    expect(screen.getByRole('combobox', { name: 'Kategória' })).not.toHaveTextContent('Všetky kategórie')
  })

  it('labels an active child of an archived parent as unavailable while retaining its filter', () => {
    render(<CategoryPicker mode="filter" value={11} categories={[{ ...parent, archived: true }, child]} onChange={vi.fn()} />)
    expect(screen.getByRole('combobox', { name: 'Kategória' })).toHaveTextContent('Potraviny (nedostupná)')
  })

  // Section 9: assignment mode with an onCreate handler offers "Nová
  // kategória..." and picking it invokes onCreate WITHOUT sending a sentinel
  // id through onChange.
  it('offers Nová kategória only in assignment mode with onCreate, and never emits a sentinel id', () => {
    const onChange = vi.fn()
    const onCreate = vi.fn()
    render(<CategoryPicker value={null} categories={[leaf]} onChange={onChange} onCreate={onCreate} />)
    openPicker()
    fireEvent.click(screen.getByRole('option', { name: 'Nová kategória...' }))
    expect(onCreate).toHaveBeenCalledOnce()
    expect(onChange).not.toHaveBeenCalled()
  })

  it('does not offer Nová kategória in filter mode even with onCreate set', () => {
    render(<CategoryPicker mode="filter" value={null} categories={[leaf]} onChange={vi.fn()} onCreate={vi.fn()} />)
    openPicker()
    expect(screen.queryByRole('option', { name: 'Nová kategória...' })).not.toBeInTheDocument()
  })

  it('does not offer Nová kategória without an onCreate handler', () => {
    render(<CategoryPicker value={null} categories={[leaf]} onChange={vi.fn()} />)
    openPicker()
    expect(screen.queryByRole('option', { name: 'Nová kategória...' })).not.toBeInTheDocument()
  })
})

// Oracle: pilot brief requires name search during category pick; empty query
// keeps the valid list; typing must not change the selected category id.
describe('category picker search', () => {
  it('filters by name case-insensitively and keeps the selected id', () => {
    const onChange = vi.fn()
    render(<CategoryPicker value={11} categories={[parent, child, leaf]} onChange={onChange} />)

    openPicker()
    fireEvent.change(searchbox(), { target: { value: 'DOPRAVA' } })

    expect(screen.getByRole('option', { name: 'Doprava' })).toBeInTheDocument()
    expect(screen.queryByRole('option', { name: 'Potraviny' })).not.toBeInTheDocument()
    expect(onChange).not.toHaveBeenCalled()
    expect(screen.getByRole('combobox', { name: 'Kategória' })).toHaveTextContent('Potraviny')
  })

  it('matches Slovak names without depending on diacritics or case', () => {
    render(<CategoryPicker value={null} categories={[housing, leaf]} onChange={vi.fn()} />)
    openPicker()
    fireEvent.change(searchbox(), { target: { value: 'BYVANIE' } })
    expect(screen.getByRole('option', { name: 'Bývanie' })).toBeInTheDocument()
    expect(screen.queryByRole('option', { name: 'Doprava' })).not.toBeInTheDocument()
  })

  it('shows parent context for a matching child and hides empty groups', () => {
    render(<CategoryPicker value={null} categories={[parent, child, leaf]} onChange={vi.fn()} />)
    openPicker()
    fireEvent.change(searchbox(), { target: { value: 'potraviny' } })
    expect(within(screen.getByRole('group', { name: 'Jedlo' })).getByRole('option', { name: 'Potraviny' })).toBeInTheDocument()
    expect(screen.queryByRole('group', { name: 'Doprava' })).not.toBeInTheDocument()
  })

  it('returns the full valid list when search is empty', () => {
    render(<CategoryPicker value={11} categories={[parent, child, leaf]} onChange={vi.fn()} />)
    openPicker()
    expect(screen.getByRole('option', { name: 'Potraviny' })).toBeInTheDocument()
    expect(screen.getByRole('option', { name: 'Doprava' })).toBeInTheDocument()
    fireEvent.change(searchbox(), { target: { value: 'doprava' } })
    fireEvent.change(searchbox(), { target: { value: '' } })
    expect(screen.getByRole('option', { name: 'Potraviny' })).toBeInTheDocument()
    expect(screen.getByRole('option', { name: 'Doprava' })).toBeInTheDocument()
  })

  it('shows a short empty message and keeps the current name', () => {
    render(<CategoryPicker value={11} categories={[parent, child, leaf]} onChange={vi.fn()} />)
    openPicker()
    fireEvent.change(searchbox(), { target: { value: 'xyzzy' } })
    expect(screen.getByRole('status')).toHaveTextContent('Žiadna kategória sa nenašla.')
    expect(screen.queryByRole('option')).not.toBeInTheDocument()
    expect(screen.getByRole('combobox', { name: 'Kategória' })).toHaveTextContent('Potraviny')
  })
})

describe('category picker keyboard and states', () => {
  it('moves through results, confirms with Enter, and returns focus on Escape', () => {
    const onChange = vi.fn()
    render(<CategoryPicker value={11} categories={[parent, child, leaf]} onChange={onChange} />)
    const trigger = screen.getByRole('combobox', { name: 'Kategória' })
    trigger.focus()
    fireEvent.keyDown(trigger, { key: 'ArrowDown' })
    expect(searchbox()).toHaveFocus()
    expect(screen.getByRole('option', { name: 'Potraviny' })).toHaveAttribute('aria-selected', 'true')

    fireEvent.keyDown(searchbox(), { key: 'ArrowDown' })
    expect(screen.getByRole('option', { name: 'Doprava' })).toHaveAttribute('aria-selected', 'true')
    fireEvent.keyDown(searchbox(), { key: 'Enter' })
    expect(onChange).toHaveBeenCalledExactlyOnceWith(12)
    expect(screen.queryByRole('searchbox')).not.toBeInTheDocument()

    trigger.focus()
    fireEvent.keyDown(trigger, { key: 'Enter' })
    fireEvent.keyDown(searchbox(), { key: 'Escape' })
    expect(screen.queryByRole('searchbox')).not.toBeInTheDocument()
    expect(trigger).toHaveFocus()
  })

  it('sends typed characters into search from the trigger', () => {
    render(<CategoryPicker value={null} categories={[leaf]} onChange={vi.fn()} />)
    const trigger = screen.getByRole('combobox', { name: 'Kategória' })
    trigger.focus()
    fireEvent.keyDown(trigger, { key: 'd' })
    expect(searchbox()).toHaveValue('d')
  })

  it('does not open search when disabled', () => {
    render(<CategoryPicker value={12} categories={[leaf]} onChange={vi.fn()} disabled />)
    const trigger = screen.getByRole('combobox', { name: 'Kategória' })
    expect(trigger).toBeDisabled()
    expect(trigger).toHaveTextContent('Doprava')
    fireEvent.click(trigger)
    expect(screen.queryByRole('searchbox')).not.toBeInTheDocument()
  })
})
