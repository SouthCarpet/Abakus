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
      {comparison.incomplete ? <p className="k-text-danger">Vybrané obdobie ešte nie je uzavreté. Porovnanie je predbežné.</p> : null}
      {currentCoverageIncomplete ? <p className="k-text-danger">Aktuálnemu obdobiu chýbajú výpisy aspoň pre jeden účet.</p> : null}
      {previousCoverageIncomplete ? <p className="k-text-danger">Predchádzajúcemu obdobiu chýbajú výpisy aspoň pre jeden účet.</p> : null}
      <div className="k-table-scroll">
        <table className="k-table">
          <thead>
            <tr>
              <th>Kategória</th>
              <th className="k-num">Aktuálne</th>
              <th className="k-num">Predchádzajúce</th>
              <th className="k-num">Zmena</th>
              <th className="k-num">%</th>
            </tr>
          </thead>
          <tbody>
            {comparison.rows.map((row) => (
              <tr key={row.categoryId ?? 'residual'}>
                <td>{row.parentName ? `${row.parentName} · ${row.name}` : row.name}</td>
                <td className="k-num">{formatEur(row.currentCents)}</td>
                <td className="k-num">{formatEur(row.previousCents)}</td>
                <td className="k-num">{formatEur(row.deltaCents)}</td>
                <td className="k-num">{percentText(row.percent)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  )
}
