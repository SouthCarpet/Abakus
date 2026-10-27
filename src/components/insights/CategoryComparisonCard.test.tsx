import { cleanup, render } from '@testing-library/react'
import { afterEach, expect, it } from 'vitest'
import { CategoryComparisonCard } from './CategoryComparisonCard'

afterEach(cleanup)
// Oracle: fix1 SLOP-LAYOUT-02 requires one warning containing the three caveats as a list.
it('fix1 combines unfinished and missing-coverage warnings into one alert with three items', () => {
  const { container } = render(<CategoryComparisonCard comparison={{ currentRange: { from: '2026-10-01', to: '2026-10-31' }, previousRange: { from: '2026-08-31', to: '2026-09-30' }, incomplete: true, rows: [] }} currentCoverageIncomplete previousCoverageIncomplete />)
  const alerts = container.querySelectorAll('.k-alert--warning')
  expect(alerts).toHaveLength(1)
  expect(Array.from(alerts[0].querySelectorAll('li'), (item) => item.textContent)).toEqual([
    'Vybrané obdobie ešte nie je uzavreté. Porovnanie je predbežné.',
    'Aktuálnemu obdobiu chýbajú výpisy aspoň pre jeden účet.',
    'Predchádzajúcemu obdobiu chýbajú výpisy aspoň pre jeden účet.',
  ])
})
it('fix1 has no warning when the period is closed and coverage is complete', () => {
  const { container } = render(<CategoryComparisonCard comparison={{ currentRange: { from: '2026-10-01', to: '2026-10-31' }, previousRange: { from: '2026-08-31', to: '2026-09-30' }, incomplete: false, rows: [] }} currentCoverageIncomplete={false} previousCoverageIncomplete={false} />)
  expect(container.querySelector('.k-alert')).toBeNull()
})
