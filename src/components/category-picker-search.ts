import type { Category } from '../api'
import type { CategoryGroup } from '../lib/categories'

export type PickerMode = 'assignment' | 'filter'

export type PickerChoice =
  | { kind: 'empty' }
  | { kind: 'category'; id: number }
  | { kind: 'create' }

export function foldLabel(value: string): string {
  return value
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .toLowerCase()
    .trim()
    .split(/\s+/)
    .join(' ')
}

function nameMatches(name: string, query: string): boolean {
  return foldLabel(name).includes(query)
}

export function filterGroupsByQuery(groups: CategoryGroup[], rawQuery: string): CategoryGroup[] {
  const query = foldLabel(rawQuery)
  if (!query) return groups
  const visible: CategoryGroup[] = []
  for (const group of groups) {
    if (nameMatches(group.parent.name, query)) {
      visible.push(group)
      continue
    }
    const subs = group.subs.filter((sub) => nameMatches(sub.name, query))
    if (subs.length > 0) visible.push({ parent: group.parent, subs })
  }
  return visible
}

export function flattenChoices(
  groups: CategoryGroup[],
  mode: PickerMode,
  includeEmpty: boolean,
  includeCreate: boolean,
): PickerChoice[] {
  const choices: PickerChoice[] = []
  if (includeEmpty) choices.push({ kind: 'empty' })
  for (const group of groups) {
    if (mode === 'filter' || group.subs.length === 0) {
      choices.push({ kind: 'category', id: group.parent.id })
    }
    for (const sub of group.subs) choices.push({ kind: 'category', id: sub.id })
  }
  if (includeCreate) choices.push({ kind: 'create' })
  return choices
}

export function isValueAvailable(groups: CategoryGroup[], value: number | null, mode: PickerMode): boolean {
  if (value === null) return false
  return groups.some((group) => parentOrChildMatches(group, value, mode))
}

function parentOrChildMatches(group: CategoryGroup, value: number, mode: PickerMode): boolean {
  const parentSelectable = mode === 'filter' || group.subs.length === 0
  if (parentSelectable && group.parent.id === value) return true
  return group.subs.some((sub) => sub.id === value)
}

export function unavailableCategoryLabel(category: Category | undefined, id: number): string {
  if (!category) return `Kategória ID ${id} (názov nie je dostupný)`
  return `${category.name} (${category.archived ? 'archivovaná' : 'nedostupná'})`
}

export function selectedLabel(
  value: number | null,
  emptyLabel: string,
  categories: Category[],
  available: boolean,
): string {
  if (value === null) return emptyLabel
  const selected = categories.find((category) => category.id === value)
  if (available && selected) return selected.name
  return unavailableCategoryLabel(selected, value)
}

export function choiceIndexForValue(choices: PickerChoice[], value: number | null): number {
  const index = choices.findIndex((choice) => choice.kind === 'category' && choice.id === value)
  return index >= 0 ? index : 0
}

export function stepIndex(index: number, delta: number, length: number): number {
  if (length === 0) return 0
  return Math.min(length - 1, Math.max(0, index + delta))
}

export function applyChoice(
  choice: PickerChoice,
  onChange: (id: number | null) => void,
  onCreate?: () => void,
): void {
  if (choice.kind === 'create') {
    onCreate?.()
    return
  }
  onChange(choice.kind === 'empty' ? null : choice.id)
}
