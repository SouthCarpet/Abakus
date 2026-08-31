import type { TooltipContentProps } from 'recharts'
import { formatEur } from '../../api'

// Shared custom content for the Tooltip on all three charts: well background,
// milled border (see .k-chart-tooltip in kaliber.css), label plus one row per
// series with formatEur values. Recharts calls this with active=false between
// hovers, so it renders nothing rather than an empty well.
export function ChartTooltip({ active, payload, label }: TooltipContentProps) {
  if (!active || payload.length === 0) return null
  const title = label ?? payload[0]?.name
  return (
    <div className="k-chart-tooltip">
      {title !== undefined ? <div className="k-chart-tooltip-label">{title}</div> : null}
      {payload.map((entry, i) => (
        <div key={entry.dataKey ? String(entry.dataKey) : i} className="k-chart-tooltip-row">
          <span className="k-legend-swatch" style={{ background: entry.color }} />
          <span>{entry.name}</span>
          <span className="k-num">{formatEur(Number(entry.value))}</span>
        </div>
      ))}
    </div>
  )
}
