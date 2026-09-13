import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent, type RefObject } from 'react'
import type { Category } from '../api'
import { groupCategories, type CategoryGroup } from '../lib/categories'
import {
  applyChoice,
  choiceIndexForValue,
  filterGroupsByQuery,
  flattenChoices,
  foldLabel,
  isValueAvailable,
  selectedLabel,
  stepIndex,
  type PickerChoice,
  type PickerMode,
} from './category-picker-search'
import './CategoryPicker.css'

const CREATE_LABEL = 'Nová kategória...'
const EMPTY_SEARCH_TEXT = 'Žiadna kategória sa nenašla.'
const SEARCH_LABEL = 'Hľadať kategóriu'

export function CategoryPicker({
  value,
  onChange,
  categories,
  kind,
  disabled,
  label = 'Kategória',
  emptyLabel = 'Nezaradené',
  mode = 'assignment',
  onCreate,
}: {
  value: number | null
  onChange: (id: number | null) => void
  categories: Category[]
  kind?: Category['kind']
  disabled?: boolean
  label?: string
  emptyLabel?: string
  mode?: PickerMode
  onCreate?: () => void
}) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [activeIndex, setActiveIndex] = useState(0)
  const rootRef = useRef<HTMLDivElement>(null)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const searchRef = useRef<HTMLInputElement>(null)
  const listId = useId()
  const offersCreate = mode === 'assignment' && !!onCreate
  const byKind = useMemo(
    () => (kind ? categories.filter((category) => category.kind === kind) : categories),
    [categories, kind],
  )
  const groups = useMemo(() => groupCategories(byKind), [byKind])
  const visibleGroups = useMemo(() => filterGroupsByQuery(groups, query), [groups, query])
  const searching = foldLabel(query) !== ''
  const choices = useMemo(
    () => flattenChoices(visibleGroups, mode, !searching, offersCreate && !searching),
    [visibleGroups, mode, searching, offersCreate],
  )
  const available = isValueAvailable(groups, value, mode)
  const triggerText = selectedLabel(value, emptyLabel, byKind, available)

  useEffect(() => {
    if (!open) return
    searchRef.current?.focus()
    setActiveIndex(choiceIndexForValue(choices, value))
  }, [open, query, choices, value])

  useEffect(() => {
    if (!open) return
    function onPointerDown(event: PointerEvent) {
      if (rootRef.current?.contains(event.target as Node)) return
      setOpen(false)
      setQuery('')
    }
    document.addEventListener('pointerdown', onPointerDown)
    return () => document.removeEventListener('pointerdown', onPointerDown)
  }, [open])

  function close(focusTrigger: boolean) {
    setOpen(false)
    setQuery('')
    if (focusTrigger) triggerRef.current?.focus()
  }

  function pick(choice: PickerChoice | undefined) {
    if (!choice) return
    applyChoice(choice, onChange, onCreate)
    close(true)
  }

  return (
    <div className="category-picker" ref={rootRef}>
      <button
        ref={triggerRef}
        type="button"
        className="k-select k-well category-picker__trigger"
        role="combobox"
        aria-label={label}
        aria-expanded={open}
        aria-controls={open ? listId : undefined}
        aria-haspopup="listbox"
        aria-activedescendant={open ? optionDomId(listId, activeIndex) : undefined}
        disabled={disabled}
        onClick={() => {
          if (disabled) return
          if (open) close(false)
          else setOpen(true)
        }}
        onKeyDown={(event) => onTriggerKey(event, disabled, setOpen, setQuery, close)}
      >
        {triggerText}
      </button>
      {open ? (
        <PickerPanel
          listId={listId}
          searchRef={searchRef}
          query={query}
          emptyLabel={emptyLabel}
          visibleGroups={visibleGroups}
          mode={mode}
          choices={choices}
          activeIndex={activeIndex}
          searching={searching}
          offersCreate={offersCreate}
          onQuery={setQuery}
          onActiveIndex={setActiveIndex}
          onPick={pick}
          onEscape={() => close(true)}
        />
      ) : null}
    </div>
  )
}

function onTriggerKey(
  event: KeyboardEvent<HTMLButtonElement>,
  disabled: boolean | undefined,
  setOpen: (open: boolean) => void,
  setQuery: (query: string) => void,
  close: (focusTrigger: boolean) => void,
) {
  if (disabled) return
  if (event.key === 'ArrowDown' || event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    setOpen(true)
    return
  }
  if (event.key === 'Escape') {
    event.preventDefault()
    close(true)
    return
  }
  if (event.key.length !== 1 || event.ctrlKey || event.metaKey || event.altKey) return
  event.preventDefault()
  setQuery(event.key)
  setOpen(true)
}

function PickerPanel({
  listId,
  searchRef,
  query,
  emptyLabel,
  visibleGroups,
  mode,
  choices,
  activeIndex,
  searching,
  offersCreate,
  onQuery,
  onActiveIndex,
  onPick,
  onEscape,
}: {
  listId: string
  searchRef: RefObject<HTMLInputElement | null>
  query: string
  emptyLabel: string
  visibleGroups: CategoryGroup[]
  mode: PickerMode
  choices: PickerChoice[]
  activeIndex: number
  searching: boolean
  offersCreate: boolean
  onQuery: (query: string) => void
  onActiveIndex: (index: number) => void
  onPick: (choice: PickerChoice | undefined) => void
  onEscape: () => void
}) {
  const active = choices[activeIndex]
  return (
    <div className="category-picker__panel k-milled">
      <input
        ref={searchRef}
        type="search"
        className="k-input k-well category-picker__search"
        aria-label={SEARCH_LABEL}
        value={query}
        onChange={(event) => onQuery(event.target.value)}
        onKeyDown={(event) => onSearchKey(event, choices, activeIndex, onActiveIndex, onPick, onEscape)}
      />
      <div className="category-picker__list" role="listbox" id={listId} aria-label={SEARCH_LABEL}>
        {searching && visibleGroups.length === 0 ? <p className="category-picker__empty" role="status">{EMPTY_SEARCH_TEXT}</p> : null}
        {!searching ? (
          <ChoiceOption
            id={optionDomId(listId, 0)}
            active={active?.kind === 'empty'}
            label={emptyLabel}
            onPick={() => onPick({ kind: 'empty' })}
            onHover={() => onActiveIndex(0)}
          />
        ) : null}
        {visibleGroups.map((group) => (
          <GroupOptions
            key={group.parent.id}
            listId={listId}
            group={group}
            mode={mode}
            choices={choices}
            activeIndex={activeIndex}
            onPick={onPick}
            onActiveIndex={onActiveIndex}
          />
        ))}
        {offersCreate && !searching ? (
          <ChoiceOption
            id={optionDomId(listId, choices.length - 1)}
            active={active?.kind === 'create'}
            label={CREATE_LABEL}
            onPick={() => onPick({ kind: 'create' })}
            onHover={() => onActiveIndex(choices.length - 1)}
          />
        ) : null}
      </div>
    </div>
  )
}

function GroupOptions({
  listId,
  group,
  mode,
  choices,
  activeIndex,
  onPick,
  onActiveIndex,
}: {
  listId: string
  group: CategoryGroup
  mode: PickerMode
  choices: PickerChoice[]
  activeIndex: number
  onPick: (choice: PickerChoice) => void
  onActiveIndex: (index: number) => void
}) {
  const showParent = mode === 'filter' || group.subs.length === 0
  return (
    <div role="group" aria-label={group.parent.name} className="category-picker__group">
      <div className="category-picker__group-label">{group.parent.name}</div>
      {showParent ? (
        <CategoryOption
          listId={listId}
          category={group.parent}
          choices={choices}
          activeIndex={activeIndex}
          onPick={onPick}
          onActiveIndex={onActiveIndex}
        />
      ) : null}
      {group.subs.map((sub) => (
        <CategoryOption
          key={sub.id}
          listId={listId}
          category={sub}
          choices={choices}
          activeIndex={activeIndex}
          onPick={onPick}
          onActiveIndex={onActiveIndex}
        />
      ))}
    </div>
  )
}

function CategoryOption({
  listId,
  category,
  choices,
  activeIndex,
  onPick,
  onActiveIndex,
}: {
  listId: string
  category: Category
  choices: PickerChoice[]
  activeIndex: number
  onPick: (choice: PickerChoice) => void
  onActiveIndex: (index: number) => void
}) {
  const index = choices.findIndex((choice) => choice.kind === 'category' && choice.id === category.id)
  return (
    <ChoiceOption
      id={optionDomId(listId, index)}
      active={index === activeIndex}
      label={category.name}
      onPick={() => onPick({ kind: 'category', id: category.id })}
      onHover={() => onActiveIndex(index)}
    />
  )
}

function ChoiceOption({
  id,
  active,
  label,
  onPick,
  onHover,
}: {
  id: string
  active: boolean
  label: string
  onPick: () => void
  onHover: () => void
}) {
  return (
    <button
      type="button"
      id={id}
      role="option"
      aria-selected={active}
      tabIndex={-1}
      className={`category-picker__option${active ? ' is-active' : ''}`}
      onClick={onPick}
      onMouseEnter={onHover}
    >
      {label}
    </button>
  )
}

function optionDomId(listId: string, index: number): string {
  return `${listId}-opt-${index}`
}

function onSearchKey(
  event: KeyboardEvent<HTMLInputElement>,
  choices: PickerChoice[],
  activeIndex: number,
  onActiveIndex: (index: number) => void,
  onPick: (choice: PickerChoice | undefined) => void,
  onEscape: () => void,
) {
  if (event.key === 'Escape') {
    event.preventDefault()
    onEscape()
    return
  }
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    onActiveIndex(stepIndex(activeIndex, 1, choices.length))
    return
  }
  if (event.key === 'ArrowUp') {
    event.preventDefault()
    onActiveIndex(stepIndex(activeIndex, -1, choices.length))
    return
  }
  if (event.key !== 'Enter') return
  event.preventDefault()
  onPick(choices[activeIndex])
}
