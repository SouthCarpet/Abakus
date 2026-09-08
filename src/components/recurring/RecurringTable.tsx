import { formatDate } from '../../lib/format'
import {
  actualChargeLabel,
  apportionmentLabel,
  cadenceLabel,
  categoryStatusLabel,
  priceChangeLabel,
  stateDetail,
  stateLabel,
} from '../../lib/recurring-labels'
import type { RecurringRow } from '../../lib/recurring-api'
import { Button } from '../Button'

export function RecurringTable({
  caption,
  rows,
  busy,
  onDetail,
  onConfirm,
  onEdit,
  onIgnore,
  onReset,
}: {
  caption: string
  rows: RecurringRow[]
  busy: boolean
  onDetail: (row: RecurringRow) => void
  onConfirm: (row: RecurringRow) => void
  onEdit: (row: RecurringRow) => void
  onIgnore: (row: RecurringRow) => void
  onReset: (row: RecurringRow) => void
}) {
  if (rows.length === 0) return null
  return (
    <div className="k-table-scroll">
      <table className="k-table">
        <caption className="k-field-label">{caption}</caption>
        <thead>
          <tr>
            <th>Názov</th>
            <th>Interval</th>
            <th className="k-num">Suma</th>
            <th>Posledná</th>
            <th>Ďalšia</th>
            <th>Stav</th>
            <th>Akcie</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <RecurringTableRow
              key={row.series_key}
              row={row}
              busy={busy}
              onDetail={onDetail}
              onConfirm={onConfirm}
              onEdit={onEdit}
              onIgnore={onIgnore}
              onReset={onReset}
            />
          ))}
        </tbody>
      </table>
    </div>
  )
}

function RecurringTableRow({
  row,
  busy,
  onDetail,
  onConfirm,
  onEdit,
  onIgnore,
  onReset,
}: {
  row: RecurringRow
  busy: boolean
  onDetail: (row: RecurringRow) => void
  onConfirm: (row: RecurringRow) => void
  onEdit: (row: RecurringRow) => void
  onIgnore: (row: RecurringRow) => void
  onReset: (row: RecurringRow) => void
}) {
  const price = priceChangeLabel(row)
  const monthly = apportionmentLabel(row)
  const ended = stateDetail(row)
  return (
    <tr>
      <td>
        <Button variant="ghost" aria-label={`Detail ${row.name}`} onClick={() => onDetail(row)}>
          {row.name}
        </Button>
        <div className="k-field-label">{row.account_label}</div>
        <div className="k-field-label">{categoryStatusLabel(row, null)}</div>
        {price ? <div><span className="k-card-badge">{price}</span></div> : null}
        {row.foreign_eur_estimate ? (
          <div className="k-field-label">Prepočet podľa poslednej platby; kurz sa môže zmeniť</div>
        ) : null}
      </td>
      <td>
        {row.cadence ? cadenceLabel(row.cadence) : 'neznámy'}
        {monthly ? <div className="k-field-label">{monthly}</div> : null}
      </td>
      <td className="k-num k-money">{actualChargeLabel(row)}</td>
      <td>{row.last_paid ? formatDate(row.last_paid) : 'chýba'}</td>
      <td>{row.next_due ? formatDate(row.next_due) : 'chýba'}</td>
      <td>
        <span className="k-card-badge">{stateLabel(row)}</span>
        {ended ? <div className="k-field-label">{ended}</div> : null}
      </td>
      <td>
        <RowActions
          row={row}
          busy={busy}
          onConfirm={onConfirm}
          onEdit={onEdit}
          onIgnore={onIgnore}
          onReset={onReset}
        />
      </td>
    </tr>
  )
}

function RowActions({
  row,
  busy,
  onConfirm,
  onEdit,
  onIgnore,
  onReset,
}: {
  row: RecurringRow
  busy: boolean
  onConfirm: (row: RecurringRow) => void
  onEdit: (row: RecurringRow) => void
  onIgnore: (row: RecurringRow) => void
  onReset: (row: RecurringRow) => void
}) {
  if (row.decision === 'estimate') {
    return (
      <div className="k-row k-recurring-actions">
        <Button variant="primary" disabled={busy} aria-label={`Potvrdiť ${row.name}`} onClick={() => onConfirm(row)}>
          Potvrdiť
        </Button>
        <Button variant="secondary" disabled={busy} aria-label={`Zmeniť interval ${row.name}`} onClick={() => onEdit(row)}>
          Zmeniť interval
        </Button>
        <Button variant="ghost" disabled={busy} aria-label={`Toto nie je pravidelná platba: ${row.name}`} onClick={() => onIgnore(row)}>
          Toto nie je pravidelná platba
        </Button>
      </div>
    )
  }
  if (row.decision === 'ignored') {
    return (
      <div className="k-row k-recurring-actions">
        <Button variant="secondary" disabled={busy} aria-label={`Upraviť ${row.name}`} onClick={() => onEdit(row)}>
          Upraviť
        </Button>
        <Button variant="ghost" disabled={busy || row.decision_id === null} aria-label={`Obnoviť odhad ${row.name}`} onClick={() => onReset(row)}>
          Obnoviť odhad
        </Button>
      </div>
    )
  }
  return (
    <div className="k-row k-recurring-actions">
      <Button variant="secondary" disabled={busy} aria-label={`Upraviť ${row.name}`} onClick={() => onEdit(row)}>
        Upraviť
      </Button>
      <Button variant="ghost" disabled={busy} aria-label={`Toto nie je pravidelná platba: ${row.name}`} onClick={() => onIgnore(row)}>
        Toto nie je pravidelná platba
      </Button>
      <Button variant="ghost" disabled={busy || row.decision_id === null} aria-label={`Obnoviť odhad ${row.name}`} onClick={() => onReset(row)}>
        Obnoviť odhad
      </Button>
    </div>
  )
}
