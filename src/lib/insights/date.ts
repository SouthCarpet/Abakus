// UTC calendar-day arithmetic for inclusive YYYY-MM-DD ranges. Using
// Date.UTC (never the local constructor) keeps leap years and DST transitions
// from perturbing day counts: a local Date at midnight can land on the wrong
// day around a DST change, UTC never does.
export interface DateRange {
  from: string
  to: string
}

function toUtcDays(iso: string): number {
  const [y, m, d] = iso.split('-').map(Number)
  return Date.UTC(y, m - 1, d) / 86_400_000
}

function fromUtcDays(days: number): string {
  const date = new Date(days * 86_400_000)
  const y = date.getUTCFullYear()
  const m = String(date.getUTCMonth() + 1).padStart(2, '0')
  const d = String(date.getUTCDate()).padStart(2, '0')
  return `${y}-${m}-${d}`
}

export function addDaysIso(iso: string, days: number): string {
  return fromUtcDays(toUtcDays(iso) + days)
}

export function dayCountInclusive(range: DateRange): number {
  return toUtcDays(range.to) - toUtcDays(range.from) + 1
}

export function rangesOverlap(a: DateRange, b: DateRange): boolean {
  return a.from <= b.to && b.from <= a.to
}

export function isValidRange(range: DateRange): boolean {
  return range.from <= range.to
}
