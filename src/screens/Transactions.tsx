import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { Account, AccountKind, Category, Status, TxFilter, TxRow } from '../api'
import { api, formatEur } from '../api'
import { Button } from '../components/Button'
import { CategoryPicker } from '../components/CategoryPicker'
import { PeriodPicker, usePeriod } from '../components/PeriodPicker'
import { OperationStatus } from '../components/OperationStatus'
import { Toast } from '../components/Toast'
import { NoteEditor } from '../components/NoteEditor'
import { statusLabel } from '../lib/categories'
import { formatDate } from '../lib/format'
import { files } from '../lib/files'
import { useAction } from '../lib/useAction'
import { periodRange, validPeriod } from '../lib/period'

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
        aria-label="Účet"
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
        aria-label="Stav"
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
      <CategoryPicker mode="filter" label="Filter kategórie" emptyLabel="Všetky kategórie" value={categoryId} onChange={onCategory} categories={categories} />
      <input
        className="k-input k-well"
        aria-label="Hľadať obchodníka alebo poznámku"
        placeholder="Hľadať obchodníka alebo poznámku..."
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
  onNoteSaved,
}: {
  row: TxRow
  categories: Category[]
  selected: boolean
  onSelect: (id: number, checked: boolean) => void
  onAssign: (id: number, categoryId: number | null) => void
  onConfirm: (id: number) => void
  onNoteSaved: () => Promise<void>
}) {
  const [expanded, setExpanded] = useState(false)
  const isTransfer = row.status === 'transfer'
  return (
    <>
      <tr>
        <td>
          <input aria-label={`Vybrať transakciu ${row.id}: ${row.merchant_raw}`} type="checkbox" checked={selected} disabled={isTransfer} onChange={(e) => onSelect(row.id, e.target.checked)} />
        </td>
        <td>{formatDate(row.tx_date)}</td>
        <td>
          <span className="k-card-badge">{row.account_kind === 'personal' ? 'O' : 'F'}</span>
        </td>
        <td>
          <div>{row.merchant_raw}</div>
          {row.place ? <div className="k-field-label">{row.place}</div> : null}
        </td>
        <td className="k-num k-money">{formatEur(row.amount_cents)}</td>
        <td>
          <CategoryPicker
            label={`Kategória transakcie ${row.id}`}
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
          <Button aria-label={`Detail transakcie ${row.id}`} aria-expanded={expanded} variant="ghost" onClick={() => setExpanded((v) => !v)}>
            {expanded ? '▾' : '▸'}
          </Button>
        </td>
      </tr>
      {expanded ? (
        <tr>
          <td colSpan={8}>
            <pre className="k-well">{row.raw_block}</pre>
            <NoteEditor txId={row.id} initialNote={row.note} onSaved={onNoteSaved} />
          </td>
        </tr>
      ) : null}
    </>
  )
}

export function Transactions({
  statementId,
  initialStatus = null,
  initialCategoryId = null,
  initialAccountKind,
}: {
  statementId?: number | null
  initialStatus?: Status | null
  initialCategoryId?: number | null
  // A17: a drill-down from a kind-filtered Prehľad carries that filter here,
  // so it never shows rows from an account the user already filtered out.
  initialAccountKind?: AccountKind | null
}) {
  const [accounts, setAccounts] = useState<Account[]>([])
  const [categories, setCategories] = useState<Category[]>([])
  const [rows, setRows] = useState<TxRow[]>([])
  const [period, setPeriod] = usePeriod()
  const [accountId, setAccountId] = useState<number | null>(null)
  const [categoryId, setCategoryId] = useState<number | null>(initialCategoryId)
  const [status, setStatus] = useState<Status | null>(initialStatus)
  const [text, debouncedText, setText] = useDebouncedText(TEXT_DEBOUNCE_MS)
  const [selected, setSelected] = useState<Set<number>>(new Set())
  const [toast, setToast] = useState<string | null>(null)
  // A17: distinguishes "no rows loaded yet" from "the filter really matches
  // nothing", so the empty sentence only shows once a real load finished.
  const [loaded, setLoaded] = useState(false)
  const [loadError, setLoadError] = useState('')
  const [metadataError, setMetadataError] = useState('')
  const [metadataLoaded, setMetadataLoaded] = useState(false)
  const [revision, setRevision] = useState(0)
  const [drilldown, setDrilldown] = useState({ statementId, accountKind: initialAccountKind })
  const action = useAction()
  const exportAction = useAction()
  const periodValid = validPeriod(period)
  const effectivePeriodValid = !!drilldown.statementId || periodValid

  const { from, to } = useMemo(() => periodRange(period.kind, new Date(), period.custom), [period])

  const filter: TxFilter = useMemo(
    () => ({
      from: drilldown.statementId ? null : from,
      to: drilldown.statementId ? null : to,
      account_id: accountId,
      account_kind: drilldown.accountKind ?? null,
      category_id: categoryId,
      status,
      text: debouncedText || null,
      statement_id: drilldown.statementId ?? null,
    }),
    [from, to, accountId, drilldown, categoryId, status, debouncedText],
  )

  const reload = useCallback(() => setRevision((value) => value + 1), [])
  const fetchRequest = useRef(0)

  // Shared by the filter-driven effect below and by a note save's own
  // refresh, so whichever request is actually the latest always wins: a
  // note-triggered refresh in flight when the filter changes again never
  // overwrites rows with a stale answer, and vice versa.
  async function fetchRows(): Promise<void> {
    const request = ++fetchRequest.current
    try {
      const data = await api.listTransactions(filter)
      if (request !== fetchRequest.current) return
      setRows(data); setLoaded(true); setLoadError('')
      // A note save can change which rows the active text filter matches; a
      // row that just dropped out of view must drop out of the bulk
      // selection with it, or a hidden row could still get bulk-assigned.
      const visibleIds = new Set(data.map((r) => r.id))
      setSelected((prev) => {
        const next = new Set([...prev].filter((id) => visibleIds.has(id)))
        return next.size === prev.size ? prev : next
      })
    } catch (e) {
      if (request === fetchRequest.current) setLoadError(String(e))
      throw e
    }
  }

  // `fetchRows` closes over `filter`, which is a fresh value every render.
  // A NoteEditor's save keeps whichever `onSaved` closure was current when
  // the save button was clicked; if the filter changes while that save's
  // write is still in flight, invoking that stale closure directly would
  // refetch with the OLD filter and, since it settles last, overwrite the
  // current (correctly filtered) rows. Routing every call through this ref
  // means it always runs the latest `fetchRows`, whichever render actually
  // triggers it.
  const latestFetchRows = useRef(fetchRows)
  latestFetchRows.current = fetchRows
  const refreshRows = useCallback(() => latestFetchRows.current(), [])

  useEffect(() => {
    let active = true
    setMetadataError(''); setMetadataLoaded(false)
    void Promise.all([api.listAccounts(), api.listCategories()]).then(([a, c]) => {
      if (!active) return
      setAccounts(a); setCategories(c); setMetadataLoaded(true)
    }).catch((e) => { if (active) setMetadataError(String(e)) })
    return () => { active = false }
  }, [revision])

  useEffect(() => {
    setRows([]); setSelected(new Set()); setLoaded(false); setLoadError('')
    if (!effectivePeriodValid || text !== debouncedText) { fetchRequest.current++; return }
    void fetchRows().catch(() => {})
  }, [filter, revision, effectivePeriodValid, text, debouncedText])

  function clearFilters() {
    setPeriod({ kind: 'all' }); setAccountId(null); setCategoryId(null)
    setStatus(null); setText(''); setDrilldown({ statementId: undefined, accountKind: undefined })
    setSelected(new Set())
  }

  async function exportCsv() {
    if (!effectivePeriodValid) return
    const currentFilter = { ...filter, text: text || null }
    setToast(null)
    const path = await files.saveCsv()
    if (!path) { setToast('Export zrušený.'); return }
    const count = await api.exportCsv(currentFilter, path)
    setToast(`Exportovaných transakcií: ${count}.`)
  }

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
      {drilldown.statementId ? <p>Vybraný výpis, všetky jeho dátumy.</p> : <PeriodPicker value={period} onChange={setPeriod} />}
      {drilldown.accountKind ? <p>Druh účtu: {drilldown.accountKind === 'personal' ? 'Osobný' : 'Firemný'}</p> : null}
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
      <div className="k-row">
        <Button variant="secondary" onClick={clearFilters}>Vymazať všetky filtre</Button>
        <Button variant="secondary" disabled={exportAction.busy || !effectivePeriodValid} onClick={() => void exportAction.run(exportCsv)}>Exportovať filtrované CSV</Button>
        <Button variant="ghost" onClick={reload}>Obnoviť</Button>
      </div>
      <OperationStatus error={metadataError} />
      <LoadStatus loaded={loaded} error={loadError} valid={effectivePeriodValid} />
      <OperationStatus error={action.error} busy={action.busy} />
      <OperationStatus error={exportAction.error} busy={exportAction.busy} />
      <FilteredTotals rows={rows} categories={categories} loaded={loaded} categoriesLoaded={metadataLoaded} />
      {toast ? <Toast message={toast} /> : null}
      <fieldset disabled={action.busy} className="k-section">
        <BulkBar
          count={selected.size}
          categories={categories}
          onAssign={(catId, applyToMatching) => void action.run(() => bulkAssign(catId, applyToMatching))}
        />
        <div className="k-table-scroll" role="region" aria-label="Transakcie" tabIndex={0}>
        <table className="k-table">
          <thead>
            <tr>
              <th></th>
              <th>Dátum</th>
              <th>Účet</th>
              <th>Obchodník</th>
              <th className="k-num k-money">Suma</th>
              <th>Kategória</th>
              <th>Stav</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {loaded && rows.length === 0 ? (
              <tr>
                <td colSpan={8}>Za toto obdobie nič nie je. Skús iné obdobie hore.</td>
              </tr>
            ) : null}
            {rows.map((row) => (
              <TransactionRow
                key={row.id}
                row={row}
                categories={categories}
                selected={selected.has(row.id)}
                onSelect={toggleSelect}
                onAssign={(id, catId) => void action.run(() => assignOne(id, catId))}
                onConfirm={(id) => void action.run(() => confirmOne(id))}
                onNoteSaved={refreshRows}
              />
            ))}
          </tbody>
        </table>
        </div>
      </fieldset>
    </div>
  )
}

// Keep the same income/refund/category semantics as the existing Overview summary.
function rowMoney(row: TxRow, kind: Category['kind'] | undefined) {
  if (row.status === 'transfer') return { income: 0, expense: 0 }
  const expense = kind === 'expense' || (!kind && (row.amount_cents < 0 || row.kind === 'refund'))
  if (expense) return { income: 0, expense: -row.amount_cents }
  const income = row.amount_cents > 0 && row.kind !== 'refund' ? row.amount_cents : 0
  return { income, expense: 0 }
}

function FilteredTotals({ rows, categories, loaded, categoriesLoaded }: {
  rows: TxRow[]; categories: Category[]; loaded: boolean; categoriesLoaded: boolean
}) {
  if (!loaded || !categoriesLoaded) return null
  const kinds = new Map(categories.map((category) => [category.id, category.kind]))
  const money = rows.map((row) => rowMoney(row, kinds.get(row.category_id ?? -1)))
  const income = money.reduce((sum, row) => sum + row.income, 0)
  const expense = money.reduce((sum, row) => sum + row.expense, 0)
  const transfers = rows.filter((row) => row.status === 'transfer').length
  return <section aria-label="Súčty zobrazených transakcií" className="k-row k-num">
    <span>Zobrazené transakcie: {rows.length}</span>
    <span>Príjem: {formatEur(income)}</span>
    <span>Výdavky: {formatEur(expense)}</span>
    <span>Čisté: {formatEur(income - expense)}</span>
    <span>Prevody mimo súčtov: {transfers}</span>
  </section>
}

function LoadStatus({ loaded, error, valid }: { loaded: boolean; error: string; valid: boolean }) {
  return <OperationStatus error={error} busy={!loaded && !error && valid}>Načítava sa...</OperationStatus>
}
