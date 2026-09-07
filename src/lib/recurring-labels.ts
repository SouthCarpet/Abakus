import type { TxRow } from '../api'
import { formatEur } from './format'
import type { Cadence, Direction, RecurringOverview, RecurringRow, UnknownReason } from './recurring-api'

const MONTH_LOCATIVE = [
  'januári',
  'februári',
  'marci',
  'apríli',
  'máji',
  'júni',
  'júli',
  'auguste',
  'septembri',
  'októbri',
  'novembri',
  'decembri',
] as const

const CADENCE_LABEL: Record<Cadence, string> = {
  monthly: 'mesačne',
  quarterly: 'štvrťročne',
  yearly: 'ročne',
}

const UNKNOWN_LABEL: Record<Exclude<UnknownReason, null>, string> = {
  missing_coverage: 'Neznáme, chýba výpis',
  ambiguous_membership: 'Neznáme, nejednoznačné členstvo',
  no_evidence: 'Neznáme, chýbajú doklady',
  unmatched_history: 'Neznáme, história nesedí',
}

export function localTodayIso(now = new Date()): string {
  const y = now.getFullYear()
  const m = String(now.getMonth() + 1).padStart(2, '0')
  const d = String(now.getDate()).padStart(2, '0')
  return `${y}-${m}-${d}`
}

export function localDateFromIso(iso: string): Date {
  const [y, m, d] = iso.split('-').map(Number)
  return new Date(y, m - 1, d)
}

export function isRecurringEligible(row: TxRow): boolean {
  return row.status !== 'transfer' && row.kind !== 'refund' && row.amount_cents !== 0
}

export function formatMoney(cents: number, currency: string): string {
  if (currency === 'EUR') return formatEur(cents)
  const sign = cents < 0 ? '-' : ''
  const abs = Math.abs(cents)
  const whole = Math.floor(abs / 100).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ' ')
  return `${sign}${whole},${(abs % 100).toString().padStart(2, '0')} ${currency}`
}

export function formatBasisPoints(bps: number): string {
  const whole = Math.floor(bps / 100)
  const frac = bps % 100
  return `${whole},${frac.toString().padStart(2, '0')} %`
}

export function cadenceLabel(cadence: Cadence): string {
  return CADENCE_LABEL[cadence]
}

export function stateLabel(row: RecurringRow): string {
  if (row.state === 'active' && row.grace_until) return `V tolerancii do ${row.grace_until}`
  if (row.state === 'active') return 'Aktívna'
  if (row.state === 'upcoming') return 'Príde do konca mesiaca'
  if (row.state === 'missing') return 'Chýba platba'
  if (row.state === 'ended') return 'Odhad ukončenia'
  if (row.unknown_reason) return UNKNOWN_LABEL[row.unknown_reason]
  return 'Neznáme'
}

export function stateDetail(row: RecurringRow): string | null {
  if (row.state === 'ended') return 'Dve očakávané platby chýbajú v úplných výpisoch.'
  return null
}

export function remainingMonthLabel(asOf: string, today: string): string {
  if (asOf.slice(0, 7) === today.slice(0, 7)) return 'Ešte príde tento mesiac'
  const monthIndex = Number(asOf.slice(5, 7)) - 1
  const year = asOf.slice(0, 4)
  return `Zostávalo v ${MONTH_LOCATIVE[monthIndex]} ${year}`
}

export function priceChangeLabel(row: RecurringRow): string | null {
  const change = row.price_change
  if (!change) return null
  const amount = formatMoney(Math.abs(change.delta_cents), change.currency)
  const from = change.effective_from.slice(0, 7)
  return priceVerb(row.direction, change.delta_cents > 0, amount, from)
}

function priceVerb(direction: Direction, increased: boolean, amount: string, from: string): string {
  if (direction === 'income' && increased) return `Príjem sa zvýšil o ${amount} od ${from}`
  if (direction === 'income') return `Príjem sa znížil o ${amount} od ${from}`
  if (increased) return `Zdraželo o ${amount} od ${from}`
  return `Zlacnelo o ${amount} od ${from}`
}

export function actualChargeLabel(row: RecurringRow): string {
  if (row.amount_cents === null) return 'suma nie je známa'
  const eur = formatEur(row.amount_cents)
  if (row.currency && row.currency !== 'EUR' && row.original_amount_cents !== null) {
    return `${formatMoney(row.original_amount_cents, row.currency)} (${eur})`
  }
  return eur
}

export function apportionmentLabel(row: RecurringRow): string | null {
  if (row.monthly_cents === null || row.cadence === null || row.cadence === 'monthly') return null
  return `${formatEur(row.monthly_cents)} mesačne (${cadenceLabel(row.cadence)})`
}

export function categoryStatusLabel(row: RecurringRow, categoryName: string | null): string {
  if (row.category_id === null) return 'Viac kategórií'
  if (categoryName) return categoryName
  return 'Bez kategórie'
}

export function snapshotCaption(overview: RecurringOverview): string {
  const asOf = overview.as_of
  const history = overview.history_from ? ` Najstarší dátum detekcie: ${overview.history_from}.` : ''
  return `Panel je snímka plánu k dátumu konca vybraného obdobia, nie súčet transakcií vo filtri. Stav k ${asOf}.${history}`
}

export function emptyPanelExplanation(): string {
  return 'Na odhad treba tri mesačné alebo dve štvrťročné či ročné pozorovania. Ručné zadanie začína v detaile transakcie.'
}

export function expenseShareLabel(overview: RecurringOverview): string {
  if (overview.expense_share_basis_points === null || overview.average_expense_cents === null) {
    return 'Podiel na výdavkoch nie je dostupný.'
  }
  const share = formatBasisPoints(overview.expense_share_basis_points)
  const average = formatEur(overview.average_expense_cents)
  const months = overview.average_months.join(', ')
  return `${share} priemerných mesačných výdavkov ${average} (${months}).`
}

export function exclusionLabel(overview: RecurringOverview): string {
  const { missing, ended, unknown } = overview.excluded
  return `Mimo súčtov: chýbajúce ${missing}, odhad ukončenia ${ended}, neznáme ${unknown}.`
}

export function cashDirectionCaption(): string {
  return 'Pravidelnosť berie skutočný smer peňazí (príjem alebo výdavok). Súhrny kategórií môžu refundácie zaradiť inak.'
}

export function partitionVisibleRows(rows: RecurringRow[]): {
  estimates: RecurringRow[]
  expenses: RecurringRow[]
  incomes: RecurringRow[]
  ignored: RecurringRow[]
} {
  return {
    estimates: rows.filter((row) => row.decision === 'estimate'),
    expenses: rows.filter((row) => row.direction === 'expense' && row.decision === 'confirmed'),
    incomes: rows.filter((row) => row.direction === 'income' && row.decision === 'confirmed'),
    ignored: rows.filter((row) => row.decision === 'ignored'),
  }
}
