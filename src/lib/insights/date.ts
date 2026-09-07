// UTC calendar-day arithmetic for inclusive YYYY-MM-DD ranges keeps day
// counts stable across leap years and daylight-saving changes.
export interface DateRange {
  from: string
  to: string
}

const ISO_DATE_RE = /^(\d{4})-(\d{2})-(\d{2})$/
const MIN_YEAR = 0
const MAX_YEAR = 9999

function parseIsoParts(iso: string): { y: number; m: number; d: number } | null {
  const match = ISO_DATE_RE.exec(iso)
  if (!match) return null
  return { y: Number(match[1]), m: Number(match[2]), d: Number(match[3]) }
}

// setUTCFullYear, unlike the Date constructor and Date.UTC, never remaps a
// 0-99 year into 1900-1999, so a low four-digit year like 0026 survives.
function toUtcDays(iso: string): number {
  const parts = parseIsoParts(iso)
  if (!parts) throw new RangeError(`Not a YYYY-MM-DD date: ${iso}`)
  const date = new Date(0)
  date.setUTCFullYear(parts.y, parts.m - 1, parts.d)
  return Math.floor(date.getTime() / 86_400_000)
}

function fromUtcDays(days: number): string {
  const date = new Date(days * 86_400_000)
  const y = date.getUTCFullYear()
  if (y < MIN_YEAR || y > MAX_YEAR) {
    throw new RangeError(`Date outside the supported ${MIN_YEAR}-${MAX_YEAR} year range`)
  }
  const m = String(date.getUTCMonth() + 1).padStart(2, '0')
  const d = String(date.getUTCDate()).padStart(2, '0')
  return `${String(y).padStart(4, '0')}-${m}-${d}`
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

// A syntactically well-formed YYYY-MM-DD only round-trips through UTC
// calendar-day arithmetic to itself if it names a real calendar date:
// setUTCFullYear silently rolls an out-of-range day (2026-02-30) into the
// next month, so the round-trip check catches what the regex alone cannot.
export function isValidIsoDate(iso: string): boolean {
  const parts = parseIsoParts(iso)
  if (!parts) return false
  const date = new Date(0)
  date.setUTCFullYear(parts.y, parts.m - 1, parts.d)
  return date.getUTCFullYear() === parts.y && date.getUTCMonth() === parts.m - 1 && date.getUTCDate() === parts.d
}

export function isValidRange(range: DateRange): boolean {
  return isValidIsoDate(range.from) && isValidIsoDate(range.to) && range.from <= range.to
}
