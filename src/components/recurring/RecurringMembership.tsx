import { formatDate, formatEur } from '../../lib/format'
import type { TxRow } from '../../api'

export function RecurringMembership({
  members,
  selectedIds,
  disabled,
  onToggle,
}: {
  members: TxRow[]
  selectedIds: number[]
  disabled: boolean
  onToggle: (id: number) => void
}) {
  if (members.length === 0) return <p>Žiadne kompatibilné transakcie.</p>
  return (
    <div>
      <p>Ďalšie platby priraďte ručne. Výber platí len pre označené riadky, budúce zhody sa samy nepridajú.</p>
      <ul className="k-round-list">
        {members.map((tx) => (
          <li key={tx.id} className="k-round-row">
            <label className="k-checkbox">
              <input
                type="checkbox"
                checked={selectedIds.includes(tx.id)}
                disabled={disabled}
                aria-label={`Člen ${tx.id} ${tx.merchant_raw} ${tx.tx_date}`}
                onChange={() => onToggle(tx.id)}
              />
              <span>{formatDate(tx.tx_date)} · {tx.merchant_raw} · {formatEur(tx.amount_cents)} · {tx.account_kind === 'personal' ? 'Osobný' : 'Firemný'}</span>
            </label>
          </li>
        ))}
      </ul>
    </div>
  )
}
