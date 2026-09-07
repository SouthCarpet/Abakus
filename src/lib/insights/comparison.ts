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
