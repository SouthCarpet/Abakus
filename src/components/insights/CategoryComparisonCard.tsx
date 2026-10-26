import { InlineAlert } from '../InlineAlert'
import type { CategoryComparison } from '../../lib/insights/comparison'
import { formatDate, formatEur } from '../../lib/format'
import { Card } from '../Card'

function percentText(percent: number | null): string {
  if (percent === null) return 'nedostupné'
  return `${percent > 0 ? '+' : ''}${percent.toFixed(1)} %`
}

export function CategoryComparisonCard({
  comparison,
  currentCoverageIncomplete,
  previousCoverageIncomplete,
}: {
  comparison: CategoryComparison
  currentCoverageIncomplete: boolean
  previousCoverageIncomplete: boolean
}) {
  return (
    <Card title="Porovnanie výdavkov podľa kategórií">
      <p>
        {`Aktuálne obdobie: ${formatDate(comparison.currentRange.from)}–${formatDate(comparison.currentRange.to)}. `}
        {`Predchádzajúce (rovnaký počet dní): ${formatDate(comparison.previousRange.from)}–${formatDate(comparison.previousRange.to)}.`}
      </p>
      {comparison.incomplete ? <InlineAlert kind="warning" announce={false}>Vybrané obdobie ešte nie je uzavreté. Porovnanie je predbežné.</InlineAlert> : null}
      {currentCoverageIncomplete ? <InlineAlert kind="warning" announce={false}>Aktuálnemu obdobiu chýbajú výpisy aspoň pre jeden účet.</InlineAlert> : null}
      {previousCoverageIncomplete ? <InlineAlert kind="warning" announce={false}>Predchádzajúcemu obdobiu chýbajú výpisy aspoň pre jeden účet.</InlineAlert> : null}
      <div className="ab-table-scroll">
        <table className="k-table">
          <thead>
            <tr>
              <th>Kategória</th>
              <th className="k-num k-cell-number">Aktuálne</th>
              <th className="k-num k-cell-number">Predchádzajúce</th>
              <th className="k-num k-cell-number">Zmena</th>
              <th className="k-num k-cell-number">%</th>
            </tr>
          </thead>
          <tbody>
            {comparison.rows.map((row) => (
              <tr key={row.categoryId ?? 'residual'}>
                <td>{row.parentName ? `${row.parentName} · ${row.name}` : row.name}</td>
                <td className="k-num k-cell-number">{formatEur(row.currentCents)}</td>
                <td className="k-num k-cell-number">{formatEur(row.previousCents)}</td>
                <td className="k-num k-cell-number">{formatEur(row.deltaCents)}</td>
                <td className="k-num k-cell-number">{percentText(row.percent)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  )
}
