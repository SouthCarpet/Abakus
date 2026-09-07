import type { StatementHistoryRow } from '../../api'
import { checksumLabel, formatDate, formatEur } from '../../lib/format'
import { Card } from '../Card'

export function BalanceHistoryCard({ statements }: { statements: StatementHistoryRow[] }) {
  if (statements.length === 0) {
    return <Card title="Zostatky z výpisov">Za vybrané obdobie nie sú žiadne výpisy.</Card>
  }
  return (
    <Card title="Zostatky z výpisov">
      <p>Zostatok podľa výpisu k dátumu jeho uzavretia. Nejde o aktuálny zostatok účtu.</p>
      <div className="k-table-scroll">
        <table className="k-table">
          <thead>
            <tr>
              <th>Účet</th>
              <th>Výpis č.</th>
              <th>Uzavretý k</th>
              <th className="k-num">Zostatok</th>
              <th>Kontrolný súčet</th>
            </tr>
          </thead>
          <tbody>
            {statements.map((s) => (
              <tr key={s.statement_id}>
                <td>{s.account_label}</td>
                <td>{s.number}</td>
                <td>{formatDate(s.period_end)}</td>
                <td className="k-num">{s.closing_cents === null ? 'Nedostupný' : formatEur(s.closing_cents)}</td>
                <td>{checksumLabel(s.checksum)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  )
}
