import type { CSSProperties } from 'react'

export type SeriesStep = { name: string } & (
  /** One of the six ramp steps; it picks the colour `--color-chart-N` and the dash `--chart-dash-N`. */
  | { step: 1 | 2 | 3 | 4 | 5 | 6; muted?: undefined }
  /** The folded Ostatné series: border colour, solid line. */
  | { muted: true; step?: undefined }
)

// The legend of dataviz.md: a line sample in the series colour and its own dash pattern.
export function ChartLegend({ series }: { series: readonly SeriesStep[] }) {
  return (
    <ul className="k-legend ab-chart-legend" aria-label="Legenda grafu">
      {series.map((entry) => (
        <li
          key={entry.name}
          className="k-legend-item"
          style={
            (entry.muted
              ? { '--k-legend-color': 'var(--color-border)', '--k-legend-dash': 'none' }
              : { '--k-legend-color': `var(--color-chart-${entry.step})`, '--k-legend-dash': `var(--chart-dash-${entry.step})` }) as CSSProperties
          }
        >
          <svg className="k-legend-swatch" viewBox="0 0 36 4" aria-hidden="true" focusable="false"><line x1="0" x2="36" y1="2" y2="2" /></svg>
          {entry.name}
        </li>
      ))}
    </ul>
  )
}
