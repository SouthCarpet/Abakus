import { useCallback, useEffect, useMemo, useState } from 'react'
import type { Account, Category, Status, TxFilter, TxRow } from '../api'
import { api, formatEur } from '../api'
import { Button } from '../components/Button'
import { CategoryPicker } from '../components/CategoryPicker'
import { PeriodPicker, usePeriod } from '../components/PeriodPicker'
import { Toast } from '../components/Toast'
import { statusLabel } from '../lib/categories'
import { formatDate } from '../lib/format'
import { periodRange } from '../lib/period'

const TEXT_DEBOUNCE_MS = 300
const STATUSES: Status[] = ['transfer', 'confirmed', 'suggested', 'unassigned']

function useDebouncedText(delay: number): [string, string, (v: string) => void] {
  const [text, setText] = useState('')
  const [debounced, setDebounced] = useState('')
  useEffect(() => {
    const t = setTimeout(() => setDebounced(text), delay)
    return () => clearTimeout(t)
  }, [text, delay])
  return [text, debounced, setText]
}

function FilterBar({
  accounts,
  categories,
  accountId,
  categoryId,
  status,
  text,
  onAccount,
  onCategory,
  onStatus,
  onText,
}: {
  accounts: Account[]
  categories: Category[]
  accountId: number | null
  categoryId: number | null
  status: Status | null
  text: string
  onAccount: (v: number | null) => void
  onCategory: (v: number | null) => void
  onStatus: (v: Status | null) => void
  onText: (v: string) => void
}) {
  return (
    <div className="k-row">
      <select
        className="k-select k-well"
        value={accountId ?? ''}
        onChange={(e) => onAccount(e.target.value ? Number(e.target.value) : null)}
      >
        <option value="">Všetko</option>
        {accounts.map((a) => (
          <option key={a.id} value={a.id}>
            {a.kind === 'personal' ? 'Osobný' : 'Firemný'} - {a.label}
          </option>
        ))}
      </select>
      <select
        className="k-select k-well"
        value={status ?? ''}
        onChange={(e) => onStatus((e.target.value || null) as Status | null)}
      >
        <option value="">Všetky stavy</option>
        {STATUSES.map((s) => (
          <option key={s} value={s}>
            {statusLabel(s)}
          </option>
        ))}
      </select>
      <CategoryPicker value={categoryId} onChange={onCategory} categories={categories} />
      <input
        className="k-input k-well"
        placeholder="Hľadať obchodníka..."
        value={text}
        onChange={(e) => onText(e.target.value)}
      />
    </div>
  )
}

function BulkBar({
  count,
  categories,
  onAssign,
}: {
  count: number
  categories: Category[]
  onAssign: (categoryId: number | null, applyToMatching: boolean) => void
}) {
  const [categoryId, setCategoryId] = useState<number | null>(null)
  const [applyToMatching, setApplyToMatching] = useState(false)
  if (count === 0) return null
  return (
    <div className="k-row k-well">
      <span>{count} vybraných</span>
      <CategoryPicker value={categoryId} onChange={setCategoryId} categories={categories} />
      <label className="k-checkbox">
        <input type="checkbox" checked={applyToMatching} onChange={(e) => setApplyToMatching(e.target.checked)} />
        Použiť aj na podobné
      </label>
      <Button variant="primary" disabled={categoryId === null} onClick={() => onAssign(categoryId, applyToMatching)}>
        Priradiť
      </Button>
    </div>
  )
}

export function TransactionRow({
  row,
  categories,
  selected,
  onSelect,
  onAssign,
  onConfirm,
}: {
  row: TxRow
  categories: Category[]
  selected: boolean
  onSelect: (id: number, checked: boolean) => void
  onAssign: (id: number, categoryId: number | null) => void
  onConfirm: (id: number) => void
}) {
  const [expanded, setExpanded] = useState(false)
  const isTransfer = row.status === 'transfer'
  return (
    <>
      <tr>
        <td>
          <input type="checkbox" checked={selected} disabled={isTransfer} onChange={(e) => onSelect(row.id, e.target.checked)} />
        </td>
        <td>{formatDate(row.tx_date)}</td>
        <td>
          <span className="k-card-badge">{row.account_kind === 'personal' ? 'O' : 'F'}</span>
        </td>
        <td>
          <div>{row.merchant_raw}</div>
          {row.place ? <div className="k-field-label">{row.place}</div> : null}
        </td>
        <td className="k-num">{formatEur(row.amount_cents)}</td>
        <td>
          <CategoryPicker
            value={row.category_id}
            onChange={(catId) => onAssign(row.id, catId)}
            categories={categories}
            disabled={isTransfer}
          />
        </td>
        <td>
          <span className="k-card-badge">{statusLabel(row.status)}</span>
          {row.status === 'suggested' ? (
            <Button variant="secondary" onClick={() => onConfirm(row.id)}>
              Potvrdiť
            </Button>
          ) : null}
        </td>
        <td>
          <Button variant="ghost" onClick={() => setExpanded((v) => !v)}>
            {expanded ? '▾' : '▸'}
          </Button>
        </td>
      </tr>
      {expanded ? (
        <tr>
          <td colSpan={8}>
            <pre className="k-well">{row.raw_block}</pre>
          </td>
        </tr>
      ) : null}
    </>
  )
}

export function Transactions({
  statementId,
  initialStatus,
  initialCategoryId,
}: {
  statementId?: number | null
  initialStatus?: Status | null
  initialCategoryId?: number | null
}) {
  const [accounts, setAccounts] = useState<Account[]>([])
  const [categories, setCategories] = useState<Category[]>([])
  const [rows, setRows] = useState<TxRow[]>([])
  const [period, setPeriod] = usePeriod()
  const [accountId, setAccountId] = useState<number | null>(null)
  const [categoryId, setCategoryId] = useState<number | null>(initialCategoryId ?? null)
  const [status, setStatus] = useState<Status | null>(initialStatus ?? null)
  const [text, debouncedText, setText] = useDebouncedText(TEXT_DEBOUNCE_MS)
  const [selected, setSelected] = useState<Set<number>>(new Set())
  const [toast, setToast] = useState<string | null>(null)

  const { from, to } = useMemo(() => periodRange(period.kind, new Date(), period.custom), [period])

  const filter: TxFilter = useMemo(
    () => ({
      from,
      to,
      account_id: accountId,
      category_id: categoryId,
      status,
      text: debouncedText || null,
      statement_id: statementId ?? null,
    }),
    [from, to, accountId, categoryId, status, debouncedText, statementId],
  )

  const reload = useCallback(() => {
    void api.listTransactions(filter).then(setRows)
  }, [filter])

  useEffect(() => {
    void api.listAccounts().then(setAccounts)
    void api.listCategories().then(setCategories)
  }, [])

  useEffect(() => {
    reload()
  }, [reload])

  function toggleSelect(id: number, checked: boolean) {
    setSelected((prev) => {
      const next = new Set(prev)
      if (checked) next.add(id)
      else next.delete(id)
      return next
    })
  }

  async function assignOne(id: number, catId: number | null) {
    if (catId === null) return
    await api.assign([id], catId, false)
    reload()
  }

  async function confirmOne(id: number) {
    await api.confirm([id])
    reload()
  }

  async function bulkAssign(catId: number | null, applyToMatching: boolean) {
    if (catId === null) return
    const outcome = await api.assign([...selected], catId, applyToMatching)
    setSelected(new Set())
    setToast(outcome.skipped_transfers > 0 ? `Prevody sa nepriraďujú, preskočené: ${outcome.skipped_transfers}` : null)
    reload()
  }

  const unassignedCount = rows.filter((r) => r.status === 'unassigned').length
  const suggestedCount = rows.filter((r) => r.status === 'suggested').length

  return (
    <div className="k-section">
      <div className="k-row">
        <h2>Transakcie</h2>
        <span className="k-card-badge">Nezaradené: {unassignedCount}</span>
        <span className="k-card-badge">Odhady: {suggestedCount}</span>
      </div>
      <PeriodPicker value={period} onChange={setPeriod} />
      <FilterBar
        accounts={accounts}
        categories={categories}
        accountId={accountId}
        categoryId={categoryId}
        status={status}
        text={text}
        onAccount={setAccountId}
        onCategory={setCategoryId}
        onStatus={setStatus}
        onText={setText}
      />
      {toast ? <Toast message={toast} /> : null}
      <BulkBar
        count={selected.size}
        categories={categories}
        onAssign={(catId, applyToMatching) => void bulkAssign(catId, applyToMatching)}
      />
      <table className="k-table">
        <thead>
          <tr>
            <th></th>
            <th>Dátum</th>
            <th>Účet</th>
            <th>Obchodník</th>
            <th className="k-num">Suma</th>
            <th>Kategória</th>
            <th>Stav</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <TransactionRow
              key={row.id}
              row={row}
              categories={categories}
              selected={selected.has(row.id)}
              onSelect={toggleSelect}
              onAssign={(id, catId) => void assignOne(id, catId)}
              onConfirm={(id) => void confirmOne(id)}
            />
          ))}
        </tbody>
      </table>
    </div>
  )
}
