import type { TxRow } from '../api'

// The register's keyboard and selection rules (Kaliber R22, "Selection model"; the keys are in
// src/kaliber/bindings.ts). Pure functions, so the rules are tested without the screen.

export const PAGE_ROWS = 10

const STEP: Record<string, (index: number, last: number) => number> = {
  ArrowDown: (i) => i + 1,
  ArrowUp: (i) => i - 1,
  PageDown: (i) => i + PAGE_ROWS,
  PageUp: (i) => i - PAGE_ROWS,
  Home: () => 0,
  End: (_, last) => last,
}

export function isMoveKey(key: string): boolean {
  return key in STEP
}

/** The row the focus moves to. With no focused row, Down, PgDn and Home start at the first
 *  row; Up, PgUp and End at the last. */
export function nextFocus(rows: TxRow[], focusedId: number | null, key: string): number | null {
  if (rows.length === 0 || !(key in STEP)) return null
  const last = rows.length - 1
  const at = focusedId === null ? -1 : rows.findIndex((r) => r.id === focusedId)
  if (at === -1) return ['ArrowUp', 'PageUp', 'End'].includes(key) ? rows[last].id : rows[0].id
  const index = Math.min(Math.max(STEP[key](at, last), 0), last)
  return rows[index].id
}

/** A transfer has no checkbox: it is never selected. */
export function selectable(row: TxRow): boolean {
  return row.status !== 'transfer'
}

/** Every selectable row from the anchor to `toId`, both included, in table order. */
export function rangeIds(rows: TxRow[], anchorId: number | null, toId: number): number[] {
  const to = rows.findIndex((r) => r.id === toId)
  const from = anchorId === null ? to : rows.findIndex((r) => r.id === anchorId)
  if (to === -1) return []
  const start = Math.min(from === -1 ? to : from, to)
  const end = Math.max(from === -1 ? to : from, to)
  return rows.slice(start, end + 1).filter(selectable).map((r) => r.id)
}

/** Ctrl+A: all visible selectable rows; with all of them already selected, none. */
export function selectAll(rows: TxRow[], selected: ReadonlySet<number>): Set<number> {
  const ids = rows.filter(selectable).map((r) => r.id)
  const all = ids.length > 0 && ids.every((id) => selected.has(id))
  return all ? new Set() : new Set(ids)
}
