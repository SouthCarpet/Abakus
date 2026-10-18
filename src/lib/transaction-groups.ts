import type { TxRow } from '../api'

export interface TransactionDisplayGroup {
  merchant: string
  place: string | null
  count: number
  ids: number[]
}

function foldLabel(label: string): string {
  return label
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .toLowerCase()
    .trim()
    .replace(/\s+/gu, ' ')
}

function countIds(rows: readonly TxRow[]): Map<number, number> {
  const counts = new Map<number, number>()
  for (const row of rows) counts.set(row.id, (counts.get(row.id) ?? 0) + 1)
  return counts
}

function groupKey(merchant: string, place: string | null): string {
  return JSON.stringify([merchant, place])
}

export function groupUnassignedTransactions(rows: readonly TxRow[]): TransactionDisplayGroup[] {
  const idCounts = countIds(rows)
  const groups: TransactionDisplayGroup[] = []
  const groupsByKey = new Map<string, TransactionDisplayGroup>()

  for (const row of rows) {
    if (row.status !== 'unassigned' || idCounts.get(row.id) !== 1) continue

    const merchant = foldLabel(row.merchant_raw)
    if (!merchant) {
      groups.push({ merchant: row.merchant_raw, place: row.place, count: 1, ids: [row.id] })
      continue
    }

    const place = row.place === null ? null : foldLabel(row.place)
    const key = groupKey(merchant, place)
    const group = groupsByKey.get(key)
    if (group) {
      group.count += 1
      group.ids.push(row.id)
      continue
    }

    const newGroup = { merchant: row.merchant_raw, place: row.place, count: 1, ids: [row.id] }
    groupsByKey.set(key, newGroup)
    groups.push(newGroup)
  }

  return groups
}
