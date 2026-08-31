import type { Category, Status, TxRow } from '../api'

export interface CategoryGroup {
  parent: Category
  subs: Category[]
}

export function groupCategories(cats: Category[]): CategoryGroup[] {
  const parents = cats.filter((c) => c.parent_id === null && !c.archived).sort((a, b) => a.sort - b.sort)
  return parents.map((parent) => ({
    parent,
    subs: cats.filter((c) => c.parent_id === parent.id && !c.archived).sort((a, b) => a.sort - b.sort),
  }))
}

export function categoryLabel(row: TxRow): string {
  if (!row.category_name) return 'Nezaradené'
  return row.parent_name ? `${row.parent_name} / ${row.category_name}` : row.category_name
}

const STATUS_LABELS: Record<Status, string> = {
  transfer: 'Prevod',
  confirmed: 'Potvrdené',
  suggested: 'Odhad',
  unassigned: 'Nezaradené',
}

export function statusLabel(s: Status): string {
  return STATUS_LABELS[s]
}
