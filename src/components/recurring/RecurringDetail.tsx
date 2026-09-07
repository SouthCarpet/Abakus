import { useEffect, useRef, useState } from 'react'
import { formatDate, formatEur } from '../../lib/format'
import { actualChargeLabel, categoryStatusLabel, stateLabel } from '../../lib/recurring-labels'
import type { RecurringDetail, RecurringQuery, RecurringRow } from '../../lib/recurring-api'
import { recurringApi } from '../../lib/recurring-api'
import { Button } from '../Button'
import { Dialog } from '../Dialog'
import { RecurringWideDialogStyle } from './recurring-dialog-style'

export function RecurringDetailDialog({
  seriesKey,
  query,
  onClose,
  onEdit,
}: {
  seriesKey: string | null
  query: RecurringQuery
  onClose: () => void
  onEdit: (row: RecurringRow, detail: RecurringDetail) => void
}) {
  const [fullHistory, setFullHistory] = useState(false)
  const [detail, setDetail] = useState<RecurringDetail | null>(null)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)
  const gen = useRef(0)

  useEffect(() => {
    setFullHistory(false)
  }, [seriesKey])

  useEffect(() => {
    const mine = ++gen.current
    if (!seriesKey) {
      setDetail(null)
      setError('')
      setLoading(false)
      return
    }
    const requestQuery = fullHistory
      ? { from: null, to: null, account_kind: query.account_kind, today: query.today }
      : query
    setDetail(null)
    setLoading(true)
    setError('')
    void recurringApi.detail({ series_key: seriesKey, query: requestQuery }).then(
      (value) => {
        if (mine !== gen.current) return
        setDetail(value)
        setLoading(false)
      },
      (e) => {
        if (mine !== gen.current) return
        setDetail(null)
        setError(String(e))
        setLoading(false)
      },
    )
    return () => { gen.current += 1 }
  }, [seriesKey, fullHistory, query.from, query.to, query.account_kind, query.today])

  function close() {
    gen.current += 1
    setDetail(null)
    setError('')
    setLoading(false)
    onClose()
  }

  function showFullHistory(value: boolean) {
    if (value === fullHistory) return
    gen.current += 1
    setDetail(null)
    setError('')
    setLoading(true)
    setFullHistory(value)
  }

  const scopeLabel = fullHistory
    ? `Celá známa história do ${query.today}`
    : periodScopeLabel(query)

  return (
    <Dialog
      open={seriesKey !== null}
      title="Presné členstvo"
      onClose={close}
      actions={(
        <>
          <Button variant="secondary" onClick={close}>Zavrieť</Button>
          <Button
            disabled={loading || !detail}
            onClick={() => { if (detail) onEdit(detail.row, detail) }}
          >
            Upraviť pravidelnosť
          </Button>
        </>
      )}
    >
      <div data-recurring-wide>
        <RecurringWideDialogStyle />
        <div className="k-row" role="group" aria-label="Rozsah detailu">
          <Button variant={fullHistory ? 'secondary' : 'primary'} aria-pressed={!fullHistory} onClick={() => showFullHistory(false)}>
            Vybrané obdobie
          </Button>
          <Button variant={fullHistory ? 'primary' : 'secondary'} aria-pressed={fullHistory} onClick={() => showFullHistory(true)}>
            Celá známa história
          </Button>
        </div>
        <p>{scopeLabel}</p>
        {loading ? <p role="status">Načítava sa detail...</p> : null}
        {error ? <p role="alert">{error}</p> : null}
        {detail ? <DetailBody detail={detail} /> : null}
      </div>
    </Dialog>
  )
}

function DetailBody({ detail }: { detail: RecurringDetail }) {
  const row = detail.row
  const memberSet = new Set(detail.matching_transaction_ids)
  const selected = row.scope === 'selected'
  return (
    <>
      <p>{row.name} · {row.account_label} · {stateLabel(row)}</p>
      <p>{categoryStatusLabel(row, null)} · {actualChargeLabel(row)}</p>
      {row.foreign_eur_estimate ? <p>Prepočet podľa poslednej platby; kurz sa môže zmeniť</p> : null}
      <p>{selected ? 'Členovia presného výberu.' : 'Transakcie v rovnakej skupine.'} Rovnaký obchodník na inom účte alebo v inej mene sem nepatrí.</p>
      <MemberTable caption={selected ? 'Členovia vo zvolenom rozsahu' : 'Transakcie vo zvolenom rozsahu'} rows={detail.transactions} memberSet={memberSet} />
      {selected ? <p>Ďalšie platby priraďte ručne.</p> : null}
      <MemberTable caption="Kompatibilné transakcie" rows={detail.compatible_transactions} memberSet={memberSet} />
    </>
  )
}

function MemberTable({
  caption,
  rows,
  memberSet,
}: {
  caption: string
  rows: RecurringDetail['transactions']
  memberSet: Set<number>
}) {
  if (rows.length === 0) return <p className="k-field-label">{caption}: žiadne.</p>
  return (
    <div className="k-table-scroll">
      <table className="k-table">
        <caption className="k-field-label">{caption}</caption>
        <thead>
          <tr>
            <th>Dátum</th>
            <th>Popis</th>
            <th className="k-num">Suma</th>
            <th>Pôvodná</th>
            <th>Kategória</th>
            <th>Účet</th>
            <th>Člen</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((tx) => (
            <tr key={tx.id}>
              <td>{formatDate(tx.tx_date)}</td>
              <td>{tx.merchant_raw}</td>
              <td className="k-num k-money">{formatEur(tx.amount_cents)}</td>
              <td>{originalMoney(tx.orig_amount_cents, tx.orig_currency)}</td>
              <td>{tx.category_name ?? 'Bez kategórie'}</td>
              <td>{tx.account_kind === 'personal' ? 'Osobný' : 'Firemný'}</td>
              <td>{memberSet.has(tx.id) ? 'áno' : 'nie'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function originalMoney(cents: number | null, currency: string | null): string {
  if (cents === null || !currency) return 'žiadna'
  const sign = cents < 0 ? '-' : ''
  const abs = Math.abs(cents)
  const whole = Math.floor(abs / 100).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ' ')
  return `${sign}${whole},${(abs % 100).toString().padStart(2, '0')} ${currency}`
}

function periodScopeLabel(query: RecurringQuery): string {
  const asOf = query.to && query.to < query.today ? query.to : query.today
  if (query.from && query.to) return `Vybrané obdobie ${query.from} až ${query.to}, stav k ${asOf}`
  return `Celá história, stav k ${query.today}`
}


