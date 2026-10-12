import type { CSSProperties } from 'react'

export interface SeriesStep {
  name: string
  /** One of the six ramp steps; it picks the colour `--color-chart-N` and the dash `--chart-dash-N`. */
  step: 1 | 2 | 3 | 4 | 5 | 6
}

// The legend of dataviz.md: a line sample in the series colour and its own dash pattern.
export function ChartLegend({ series }: { series: readonly SeriesStep[] }) {
  return (
    <ul className="k-legend ab-chart-legend" aria-label="Legenda grafu">
      {series.map(({ name, step }) => (
        <li
          key={name}
          className="k-legend-item"
          style={{ '--k-legend-color': `var(--color-chart-${step})`, '--k-legend-dash': `var(--chart-dash-${step})` } as CSSProperties}
        >
          <svg className="k-legend-swatch" viewBox="0 0 36 4" aria-hidden="true" focusable="false"><line x1="0" x2="36" y1="2" y2="2" /></svg>
          {name}
        </li>
      ))}
    </ul>
  )
}
