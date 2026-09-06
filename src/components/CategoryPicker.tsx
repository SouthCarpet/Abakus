import type { Category } from '../api'
import { groupCategories } from '../lib/categories'

export function CategoryPicker({
  value,
  onChange,
  categories,
  kind,
  disabled,
  label = 'Kategória',
  emptyLabel = 'Nezaradené',
  mode = 'assignment',
}: {
  value: number | null
  onChange: (id: number | null) => void
  categories: Category[]
  kind?: Category['kind']
  disabled?: boolean
  label?: string
  emptyLabel?: string
  mode?: 'assignment' | 'filter'
}) {
  const filtered = kind ? categories.filter((c) => c.kind === kind) : categories
  const groups = groupCategories(filtered)
  const selectedIsAvailable = groups.some(({ parent, subs }) =>
    ((mode === 'filter' || subs.length === 0) && parent.id === value) || subs.some((sub) => sub.id === value))
  const selected = filtered.find((category) => category.id === value)
  return (
    <select
      aria-label={label}
      className="k-select k-well"
      value={value ?? ''}
      disabled={disabled}
      onChange={(e) => onChange(Number(e.target.value) || null)}
    >
      <option value="">{emptyLabel}</option>
      {mode === 'filter' && value !== null && !selectedIsAvailable ? (
        <option value={value} disabled>{unavailableCategoryLabel(selected, value)}</option>
      ) : null}
      {groups.map(({ parent, subs }) => (
        <optgroup key={parent.id} label={parent.name}>
          {mode === 'filter' || subs.length === 0 ? <option value={parent.id}>{parent.name}</option> : null}
          {subs.map((sub) => (
                <option key={sub.id} value={sub.id}>
                  {sub.name}
                </option>
              ))}
        </optgroup>
      ))}
    </select>
  )
}

function unavailableCategoryLabel(category: Category | undefined, id: number): string {
  if (!category) return `Kategória ID ${id} (názov nie je dostupný)`
  return `${category.name} (${category.archived ? 'archivovaná' : 'nedostupná'})`
}
