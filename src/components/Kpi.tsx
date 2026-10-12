import { formatEur } from '../lib/format'

export type KpiTone = 'default' | 'danger' | 'success'

function toneClass(tone: KpiTone): string {
  if (tone === 'danger') return ' ab-text-danger'
  if (tone === 'success') return ' ab-text-success'
  return ''
}

function gaugePercent(cents: number, min: number, max: number): number {
  const span = max - min
  if (span <= 0) return 0
  return Math.min(100, Math.max(0, ((cents - min) / span) * 100))
}

// One row of a `.k-summary` table (Kaliber, replaces the hero KPI tile, SLOP-LAYOUT-05):
// the label, the value and a micro-gauge under a metric with a monthly min and max.
export function Kpi({
  label,
  cents,
  min = 0,
  max,
  tone = 'default',
  baselineOnly = false,
}: {
  label: string
  cents: number
  min?: number
  max?: number
  tone?: KpiTone
  /** Renders the gauge track with no fill and no tick. For tiles like Čisté,
   * where cents is a signed net figure with no meaningful min/max span, so a
   * filled gauge would misleadingly read as "100% of something". */
  baselineOnly?: boolean
}) {
  const span = max ?? Math.max(Math.abs(cents), 1)
  const pct = gaugePercent(cents, min, span)
  return (
    <div className="k-summary-row">
      <dt>{label}</dt>
      <dd className={`k-summary-value k-num${toneClass(tone)}`}>{formatEur(cents)}</dd>
      <dd className="k-summary-gauge">
        <div className="k-gauge">
          {baselineOnly ? null : <div className="k-gauge-fill" style={{ width: `${pct}%` }} />}
        </div>
      </dd>
    </div>
  )
}
