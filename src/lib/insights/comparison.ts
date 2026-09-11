import type { Summary } from '../../api'
import { addDaysIso, dayCountInclusive, type DateRange } from './date'

export interface CategoryComparisonRow {
  categoryId: number | null
  name: string
  parentName: string | null
  currentCents: number
  previousCents: number
  deltaCents: number
  percent: number | null
}

export interface CategoryComparison {
  currentRange: DateRange
  previousRange: DateRange
  incomplete: boolean
  rows: CategoryComparisonRow[]
}

export type YearComparisonPeriod = 'month' | 'three-months'

export interface YearComparisonRanges {
  current: DateRange
  previous: DateRange
}

const YEAR_MONTH_RE = /^(\d{4})-(0[1-9]|1[0-2])$/

function periodMonthCount(period: YearComparisonPeriod): number {
  if (period === 'month') return 1
  if (period === 'three-months') return 3
  throw new RangeError(`Unknown comparison period: ${String(period)}`)
}

function monthIndex(selectedMonth: string): number {
  const match = YEAR_MONTH_RE.exec(selectedMonth)
  if (!match) throw new RangeError(`Not a YYYY-MM calendar month: ${selectedMonth}`)
  return Number(match[1]) * 12 + Number(match[2]) - 1
}

function monthParts(index: number): { year: number; month: number } {
  if (index < 0 || index > 119_999) throw new RangeError('Comparison month is outside the supported 0000-9999 year range')
  return { year: Math.floor(index / 12), month: (index % 12) + 1 }
}

function daysInMonth(year: number, month: number): number {
  if (month !== 2) return [31, 0, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][month - 1]
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0)
  return leap ? 29 : 28
}

function formatMonthDay(index: number, day: number): string {
  const { year, month } = monthParts(index)
  return `${String(year).padStart(4, '0')}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`
}

function calendarMonthRange(fromIndex: number, toIndex: number): DateRange {
  const { year, month } = monthParts(toIndex)
  return {
    from: formatMonthDay(fromIndex, 1),
    to: formatMonthDay(toIndex, daysInMonth(year, month)),
  }
}

/** Full calendar month(s) ending in `selectedMonth`, paired with the same
 * calendar months one year earlier. Both returned ranges are inclusive. */
export function yearComparisonRanges(selectedMonth: string, period: YearComparisonPeriod): YearComparisonRanges {
  const currentTo = monthIndex(selectedMonth)
  const currentFrom = currentTo - periodMonthCount(period) + 1
  return {
    current: calendarMonthRange(currentFrom, currentTo),
    previous: calendarMonthRange(currentFrom - 12, currentTo - 12),
  }
}

/** The immediately preceding inclusive range with the same UTC calendar-day
 * count as `current`. Not calendar-month aligned: a 28-day February compares
 * against the last 28 days of January, not all of January. */
export function previousEqualRange(current: DateRange): DateRange {
  const days = dayCountInclusive(current)
  const to = addDaysIso(current.from, -1)
  const from = addDaysIso(to, -(days - 1))
  return { from, to }
}

/** A range whose end has not yet fully elapsed (today or later) is an
 * incomplete period: its totals can still change. */
export function isIncompletePeriod(range: DateRange, today: string): boolean {
  return range.to >= today
}

/** Percent change against the previous value, denominator abs(previous).
 * Returns null (never 0% or Infinity) when previous is zero. */
export function percentDelta(current: number, previous: number): number | null {
  if (previous === 0) return null
  return ((current - previous) / Math.abs(previous)) * 100
}

function sumCents(rows: Summary['by_category']): number {
  return rows.reduce((sum, row) => sum + row.cents, 0)
}

function centsOf(row: Summary['by_category'][number] | undefined): number {
  return row?.cents ?? 0
}

function firstDefined<T>(a: T | undefined, b: T | undefined, fallback: T): T {
  return a ?? b ?? fallback
}

function categoryRow(
  categoryId: number,
  current: Summary['by_category'][number] | undefined,
  previous: Summary['by_category'][number] | undefined,
): CategoryComparisonRow {
  const currentCents = centsOf(current)
  const previousCents = centsOf(previous)
  return {
    categoryId,
    name: firstDefined(current?.name, previous?.name, ''),
    parentName: firstDefined(current?.parent_name, previous?.parent_name, null),
    currentCents,
    previousCents,
    deltaCents: currentCents - previousCents,
    percent: percentDelta(currentCents, previousCents),
  }
}

/** Unions both sides by category id, including a category present on only
 * one side, per the backend's expense-only signed by_category rows. */
function unionCategoryRows(current: Summary['by_category'], previous: Summary['by_category']): CategoryComparisonRow[] {
  const currentById = new Map(current.map((c) => [c.category_id, c]))
  const previousById = new Map(previous.map((p) => [p.category_id, p]))
  const ids = new Set([...currentById.keys(), ...previousById.keys()])
  return [...ids].map((id) => categoryRow(id, currentById.get(id), previousById.get(id)))
}

/** The uncategorized residual: expense_cents minus the categorized sum, so
 * the table's rows reconcile to expense_cents. Not a drilldown target
 * (categoryId is null, not a real category id). */
function residualRow(
  current: Pick<Summary, 'by_category' | 'expense_cents'>,
  previous: Pick<Summary, 'by_category' | 'expense_cents'>,
): CategoryComparisonRow {
  const currentCents = current.expense_cents - sumCents(current.by_category)
  const previousCents = previous.expense_cents - sumCents(previous.by_category)
  return {
    categoryId: null,
    name: 'Nezaradené výdavky',
    parentName: null,
    currentCents,
    previousCents,
    deltaCents: currentCents - previousCents,
    percent: percentDelta(currentCents, previousCents),
  }
}

export function compareCategories(
  currentRange: DateRange,
  today: string,
  current: Pick<Summary, 'by_category' | 'expense_cents'>,
  previous: Pick<Summary, 'by_category' | 'expense_cents'>,
): CategoryComparison {
  return {
    currentRange,
    previousRange: previousEqualRange(currentRange),
    incomplete: isIncompletePeriod(currentRange, today),
    rows: [...unionCategoryRows(current.by_category, previous.by_category), residualRow(current, previous)],
  }
}
