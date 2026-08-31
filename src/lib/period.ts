export type PeriodKind = 'this_month' | 'last_month' | 'm3' | 'm6' | 'y1' | 'all' | 'custom'
export const PERIOD_LABELS: Record<PeriodKind, string> = { this_month: 'Tento mesiac', last_month: 'Minulý mesiac', m3: '3M', m6: '6M', y1: '1R', all: 'Všetko', custom: 'Vlastné' }
const iso = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
const firstOfMonth = (y: number, m: number) => new Date(y, m, 1)
const lastOfMonth = (y: number, m: number) => new Date(y, m + 1, 0)
export function periodRange(kind: PeriodKind, today: Date, custom?: { from: string; to: string }): { from: string | null; to: string | null } {
  const y = today.getFullYear(), m = today.getMonth()
  switch (kind) {
    case 'this_month': return { from: iso(firstOfMonth(y, m)), to: iso(lastOfMonth(y, m)) }
    case 'last_month': return { from: iso(firstOfMonth(y, m - 1)), to: iso(lastOfMonth(y, m - 1)) }
    case 'm3': return { from: iso(firstOfMonth(y, m - 2)), to: iso(today) }
    case 'm6': return { from: iso(firstOfMonth(y, m - 5)), to: iso(today) }
    case 'y1': return { from: iso(firstOfMonth(y, m - 11)), to: iso(today) }
    case 'custom': return custom ? { from: custom.from, to: custom.to } : { from: null, to: null }
    default: return { from: null, to: null }
  }
}
