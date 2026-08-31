import type { Category } from '../api'
import { groupCategories } from '../lib/categories'

export function CategoryPicker({
  value,
  onChange,
  categories,
  kind,
  disabled,
}: {
  value: number | null
  onChange: (id: number | null) => void
  categories: Category[]
  kind?: Category['kind']
  disabled?: boolean
}) {
  const filtered = kind ? categories.filter((c) => c.kind === kind) : categories
  const groups = groupCategories(filtered)
  return (
    <select
      className="k-select k-well"
      value={value ?? ''}
      disabled={disabled}
      onChange={(e) => onChange(Number(e.target.value) || null)}
    >
      <option value="">Nezaradené</option>
      {groups.map(({ parent, subs }) => (
        <optgroup key={parent.id} label={parent.name}>
          {subs.length > 0
            ? subs.map((sub) => (
                <option key={sub.id} value={sub.id}>
                  {sub.name}
                </option>
              ))
            : <option value={parent.id}>{parent.name}</option>}
        </optgroup>
      ))}
    </select>
  )
}
