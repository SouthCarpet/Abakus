import type { AccountCoverage } from '../../lib/insights/coverage'
import { formatDate } from '../../lib/format'
import { Card } from '../Card'

function rangeText(from: string, to: string): string {
  return from === to ? formatDate(from) : `${formatDate(from)}–${formatDate(to)}`
}

function AccountRow({ coverage }: { coverage: AccountCoverage }) {
  if (!coverage.hasStatements) {
    return (
      <tr>
        <td>{coverage.accountLabel}</td>
        <td colSpan={2}>Bez výpisov</td>
      </tr>
    )
  }
  const known = coverage.knownFrom && coverage.knownTo ? rangeText(coverage.knownFrom, coverage.knownTo) : 'Žiadne platné obdobie'
  const gapsText = coverage.gaps.length === 0 ? 'Bez medzier' : coverage.gaps.map((g) => rangeText(g.from, g.to)).join(', ')
  const reviewText = coverage.invalidRanges.length > 0
    ? ` · Na kontrolu (neplatný rozsah): ${coverage.invalidRanges.map((r) => rangeText(r.from, r.to)).join(', ')}`
    : ''
  return (
    <tr>
      <td>{coverage.accountLabel}</td>
      <td>{known}</td>
      <td>{gapsText}{reviewText}</td>
    </tr>
  )
}

export function CoverageCard({ coverages, allTime }: { coverages: AccountCoverage[]; allTime: boolean }) {
  if (coverages.length === 0) return null
  return (
    <Card title="Pokrytie výpismi">
      <p>{allTime ? 'Medzery sa počítajú len medzi najstarším a najnovším výpisom daného účtu.' : 'Medzery sa počítajú vo vybranom období.'}</p>
      <div className="k-table-scroll">
        <table className="k-table">
          <thead>
            <tr><th>Účet</th><th>Známa história</th><th>Medzery</th></tr>
          </thead>
          <tbody>
            {coverages.map((c) => <AccountRow key={c.accountId} coverage={c} />)}
          </tbody>
        </table>
      </div>
    </Card>
  )
}
