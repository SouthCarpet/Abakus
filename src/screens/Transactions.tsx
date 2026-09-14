import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react'
import type { Account, AccountKind, Category, RuleView, Status, TxFilter, TxKind, TxRow } from '../api'
import { api, formatEur } from '../api'
import { Button } from '../components/Button'
import { CategoryDialog } from '../components/CategoryDialog'
import { CategoryPicker } from '../components/CategoryPicker'
import { PeriodPicker, usePeriod } from '../components/PeriodPicker'
import { OperationStatus } from '../components/OperationStatus'
import { Toast } from '../components/Toast'
import { NoteEditor } from '../components/NoteEditor'
import { RecurringTransactionAction } from '../components/recurring'
import { statusLabel } from '../lib/categories'
import { categoryApi } from '../lib/category-api'
import { formatDate } from '../lib/format'
import { files } from '../lib/files'
import { useAction } from '../lib/useAction'
import { periodRange, validPeriod } from '../lib/period'
import { groupUnassignedTransactions, type TransactionDisplayGroup } from '../lib/transaction-groups'

const TEXT_DEBOUNCE_MS = 300
const STATUSES: Status[] = ['transfer', 'confirmed', 'suggested', 'unassigned']
const ACCOUNT_MATCH_KINDS = new Set(['transfer_in', 'transfer_out', 'standing_order'])

// Point 13: fees get their own TxKind so they can be filtered separately
// from every other kind, without inventing a new concept beyond what the
// backend already recognizes.
const TX_KIND_LABELS: Record<TxKind, string> = {
  card: 'Karta',
  card_foreign: 'Karta v cudzej mene',
  refund: 'Vrátenie',
  atm: 'Bankomat',
  transfer_in: 'Prichádzajúci prevod',
  transfer_out: 'Odchádzajúci prevod',
  standing_order: 'Trvalý príkaz',
  fee: 'Poplatok',
  other: 'Iné',
}
const TX_KINDS = Object.keys(TX_KIND_LABELS) as TxKind[]

// Point 1: mirrors the backend's ExactIdentity match used by confirm's
// applyToMatching (same counterparty account for transfer-like kinds,
// otherwise same merchant and place). This is a display estimate over the
// rows already on screen; the backend still decides the real set on confirm.
function matchesForConfirm(a: TxRow, b: TxRow): boolean {
  if (ACCOUNT_MATCH_KINDS.has(a.kind) && ACCOUNT_MATCH_KINDS.has(b.kind)) {
    return !!a.counterparty_iban && a.counterparty_iban === b.counterparty_iban
  }
  return a.merchant_raw === b.merchant_raw && a.place === b.place
}

function countMatchingUnconfirmed(rows: TxRow[], row: TxRow): number {
  return rows.filter(
    (candidate) =>
      candidate.id !== row.id &&
      (candidate.status === 'suggested' || candidate.status === 'unassigned') &&
      matchesForConfirm(candidate, row),
  ).length
}

// Point 2: unassigned rows sharing a merchant (and place) are frequent and
// hard to tell apart one row at a time. `groupUnassignedTransactions` already
// computes the groups; this only decides the display sentence and wires a
// click straight into the existing bulk selection below.
function groupLabel(group: TransactionDisplayGroup): string {
  const merchant = group.merchant.trim() ? group.merchant : 'Bez obchodníka'
  const parts = group.place ? [merchant, group.place] : [merchant]
  return `${parts.join(', ')}, ${group.count} platieb, nepriradené`
}

function UnassignedGroupsBar({ rows, onSelectGroup }: { rows: TxRow[]; onSelectGroup: (ids: number[]) => void }) {
  const groups = useMemo(() => groupUnassignedTransactions(rows), [rows])
  if (groups.length === 0) return null
  return (
    <div className="k-row k-well" role="region" aria-label="Skupiny nezaradených platieb">
      {groups.map((group) => (
        <Button key={group.ids[0]} variant="ghost" onClick={() => onSelectGroup(group.ids)}>
          {groupLabel(group)}
        </Button>
      ))}
    </div>
  )
}

// Point 12: the table region owns arrow/Enter navigation, but only once the
// key reaches it from something that is not a form field: the search box,
// a note, and the CategoryPicker's own search all keep typing normal keys.
function isFormField(target: EventTarget | null): boolean {
  const tag = (target as HTMLElement | null)?.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT'
}

function nextFocusedRowId(rows: TxRow[], currentId: number | null, direction: 1 | -1): number | null {
  if (rows.length === 0) return null
  const currentIndex = currentId === null ? -1 : rows.findIndex((r) => r.id === currentId)
  if (currentIndex === -1) return direction === 1 ? rows[0].id : rows[rows.length - 1].id
  const nextIndex = Math.min(Math.max(currentIndex + direction, 0), rows.length - 1)
  return rows[nextIndex].id
}

function useDebouncedText(delay: number, initial = ''): [string, string, (v: string) => void] {
  const [text, setText] = useState(initial)
  const [debounced, setDebounced] = useState(initial)
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
  kind,
  text,
  onAccount,
  onCategory,
  onStatus,
  onKind,
  onText,
}: {
  accounts: Account[]
  categories: Category[]
  accountId: number | null
  categoryId: number | null
  status: Status | null
  kind: TxKind | null
  text: string
  onAccount: (v: number | null) => void
  onCategory: (v: number | null) => void
  onStatus: (v: Status | null) => void
  onKind: (v: TxKind | null) => void
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
      <select
        className="k-select k-well"
        aria-label="Druh"
        value={kind ?? ''}
        onChange={(e) => onKind((e.target.value || null) as TxKind | null)}
      >
        <option value="">Všetky druhy</option>
        {TX_KINDS.map((k) => (
          <option key={k} value={k}>
            {TX_KIND_LABELS[k]}
          </option>
        ))}
      </select>
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
  categoryId,
  onCategoryChange,
  onCreate,
  onAssign,
  onConfirm,
}: {
  count: number
  categories: Category[]
  categoryId: number | null
  onCategoryChange: (id: number | null) => void
  onCreate: () => void
  onAssign: (categoryId: number | null, applyToMatching: boolean) => void
  onConfirm: (applyToMatching: boolean) => void
}) {
  const [applyToMatching, setApplyToMatching] = useState(false)
  if (count === 0) return null
  return (
    <div className="k-row k-well">
      <span>{count} vybraných</span>
      <CategoryPicker value={categoryId} onChange={onCategoryChange} categories={categories} onCreate={onCreate} />
      <label className="k-checkbox">
        <input type="checkbox" checked={applyToMatching} onChange={(e) => setApplyToMatching(e.target.checked)} />
        Použiť aj na podobné
      </label>
      <Button variant="primary" disabled={categoryId === null} onClick={() => onAssign(categoryId, applyToMatching)}>
        Priradiť
      </Button>
      {/* Point 11: bulk confirm needs no category, so it stays enabled on any selection. */}
      <Button variant="secondary" onClick={() => onConfirm(applyToMatching)}>
        Potvrdiť vybrané
      </Button>
    </div>
  )
}

// Section 9 C05: an eligible seed-classified row shows its actual seed rule
// and a redirect picker; a learned or absent pointer shows nothing (never
// inferred from merchant text or category name).
function SeedRuleRedirect({ row, categories, onChanged, onCreate }: { row: TxRow; categories: Category[]; onChanged: () => Promise<void>; onCreate: () => void }) {
  const [rule, setRule] = useState<RuleView | null>(null)
  const [loaded, setLoaded] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    let active = true
    setLoaded(false)
    setError('')
    void categoryApi
      .seedRuleForTransaction(row.id)
      .then((r) => { if (active) { setRule(r); setLoaded(true) } })
      .catch((e) => { if (active) { setError(String(e)); setLoaded(true) } })
    return () => { active = false }
  }, [row.id])

  async function redirect(categoryId: number | null) {
    if (categoryId === null || !rule) return
    setBusy(true)
    setError('')
    try {
      await categoryApi.redirectRule(rule.id, categoryId)
      await onChanged()
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  if (!loaded || !rule) return error ? <p role="alert">{error}</p> : null
  return (
    <div className="k-row">
      {error ? <p role="alert">{error}</p> : null}
      <span>Zaradené podľa slovníka: {rule.parent_name ? `${rule.parent_name} / ${rule.category_name}` : rule.category_name}</span>
      <CategoryPicker
        label={`Presmerovať pravidlo transakcie ${row.id}`}
        value={null}
        emptyLabel="Presmerovať na..."
        onChange={(id) => void redirect(id)}
        categories={categories}
        disabled={busy}
        onCreate={onCreate}
      />
    </div>
  )
}

export function TransactionRow({
  row,
  categories,
  selected,
  matchingCount = 0,
  focused = false,
  onSelect,
  onAssign,
  onConfirm,
  onNoteSaved,
  onCreateCategory,
}: {
  row: TxRow
  categories: Category[]
  selected: boolean
  // Point 1: how many other unconfirmed rows share this row's merchant/place
  // (or account, for transfer-like kinds). 0 hides the apply-to-matching option.
  matchingCount?: number
  // Point 12: true for the row currently holding the keyboard roving highlight.
  focused?: boolean
  onSelect: (id: number, checked: boolean) => void
  onAssign: (id: number, categoryId: number | null) => void
  onConfirm: (id: number, applyToMatching: boolean) => void
  onNoteSaved: () => Promise<void>
  onCreateCategory: (rowId: number) => void
}) {
  const [expanded, setExpanded] = useState(false)
  const [applyToMatching, setApplyToMatching] = useState(false)
  const isTransfer = row.status === 'transfer'
  return (
    <>
      <tr className={focused ? 'is-active' : undefined}>
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
            onCreate={() => onCreateCategory(row.id)}
          />
        </td>
        <td>
          <span className="k-card-badge">{statusLabel(row.status)}</span>
          {row.status === 'suggested' ? (
            <>
              <Button variant="secondary" onClick={() => onConfirm(row.id, applyToMatching)}>
                Potvrdiť
              </Button>
              {matchingCount > 0 ? (
                <label className="k-checkbox">
                  <input type="checkbox" checked={applyToMatching} onChange={(e) => setApplyToMatching(e.target.checked)} />
                  {`Potvrdiť aj podobné (${matchingCount})`}
                </label>
              ) : null}
            </>
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
            <RecurringTransactionAction row={row} categories={categories} onChanged={onNoteSaved} />
            {!isTransfer ? <SeedRuleRedirect row={row} categories={categories} onChanged={onNoteSaved} onCreate={() => onCreateCategory(row.id)} /> : null}
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
  initialText,
  dbGeneration,
}: {
  statementId?: number | null
  initialStatus?: Status | null
  initialCategoryId?: number | null
  // A17: a drill-down from a kind-filtered Prehľad carries that filter here,
  // so it never shows rows from an account the user already filtered out.
  initialAccountKind?: AccountKind | null
  // Point 17: a merchant click in Overview lands here with the merchant name
  // already applied as the search filter (point 9's substring search).
  initialText?: string
  // 091/B10 p3 fix: bumped by App once a database restore lands. A plain
  // remount already gets a fresh start (App gives this screen a fresh `key`
  // on every navigation into it), so this only matters for an instance that
  // stays mounted across a generation bump; either way it drops the old-DB
  // transient state below and refetches.
  dbGeneration?: number
}) {
  const [accounts, setAccounts] = useState<Account[]>([])
  const [categories, setCategories] = useState<Category[]>([])
  const [rows, setRows] = useState<TxRow[]>([])
  const [period, setPeriod] = usePeriod()
  const [accountId, setAccountId] = useState<number | null>(null)
  const [categoryId, setCategoryId] = useState<number | null>(initialCategoryId)
  const [status, setStatus] = useState<Status | null>(initialStatus)
  const [kind, setKind] = useState<TxKind | null>(null)
  const [text, debouncedText, setText] = useDebouncedText(TEXT_DEBOUNCE_MS, initialText)
  const [selected, setSelected] = useState<Set<number>>(new Set())
  // Point 12: which row currently holds the keyboard roving highlight.
  const [focusedRowId, setFocusedRowId] = useState<number | null>(null)
  const [toast, setToast] = useState<string | null>(null)
  // A17: distinguishes "no rows loaded yet" from "the filter really matches
  // nothing", so the empty sentence only shows once a real load finished.
  const [loaded, setLoaded] = useState(false)
  const [loadError, setLoadError] = useState('')
  const [metadataError, setMetadataError] = useState('')
  const [metadataLoaded, setMetadataLoaded] = useState(false)
  const [revision, setRevision] = useState(0)
  const [drilldown, setDrilldown] = useState({ statementId, accountKind: initialAccountKind })
  const [bulkCategoryId, setBulkCategoryId] = useState<number | null>(null)
  // Section 9 U05: which picker asked for "Nová kategória...", so the one
  // shared CategoryDialog knows where the created category's own explicit
  // assignment goes: the bulk selection, or straight onto that one row.
  const [createFor, setCreateFor] = useState<'bulk' | { rowId: number } | null>(null)
  // Point 15: undo covers a bulk assignment only (the backend gives no undo
  // id for confirm). Valid only for the most recent bulk assignment: any
  // other mutation clears it, and a new bulk assignment replaces it.
  const [lastUndo, setLastUndo] = useState<{ id: string } | null>(null)
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
      kind,
      text: debouncedText || null,
      statement_id: drilldown.statementId ?? null,
    }),
    [from, to, accountId, drilldown, categoryId, status, kind, debouncedText],
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

  // 091/B10 p3 fix: a `dbGeneration` bump means the database underneath was
  // just replaced by a restore. `reload()` alone (via `revision`) already
  // gets accounts, categories and rows fetched again through the effects
  // below, but it does not touch `lastUndo`: an undo id from the OLD
  // database must never be offered against the new one. Selection and the
  // keyboard focus are old-DB row ids too, so they go with it. The ref skips
  // only the mount run, so a real second generation still resets and
  // refetches even if `dbGeneration` never left its initial value before.
  const isFirstDbGeneration = useRef(true)
  useEffect(() => {
    if (isFirstDbGeneration.current) { isFirstDbGeneration.current = false; return }
    setLastUndo(null)
    setSelected(new Set())
    setFocusedRowId(null)
    reload()
  }, [dbGeneration, reload])

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

  // Point 12: a row that scrolls out of the current filter must drop the
  // keyboard highlight with it, or the highlight would point at nothing.
  useEffect(() => {
    if (focusedRowId !== null && !rows.some((r) => r.id === focusedRowId)) setFocusedRowId(null)
  }, [rows, focusedRowId])

  function onTableKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (isFormField(event.target)) return
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      setFocusedRowId(nextFocusedRowId(rows, focusedRowId, event.key === 'ArrowDown' ? 1 : -1))
      return
    }
    if (event.key !== 'Enter') return
    const focusedRow = rows.find((r) => r.id === focusedRowId)
    if (!focusedRow || focusedRow.status !== 'suggested') return
    event.preventDefault()
    void action.run(() => confirmOne(focusedRow.id, false))
  }

  function clearFilters() {
    setPeriod({ kind: 'all' }); setAccountId(null); setCategoryId(null)
    setStatus(null); setKind(null); setText(''); setDrilldown({ statementId: undefined, accountKind: undefined })
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
    setLastUndo(null)
    await api.assign([id], catId, false)
    reload()
  }

  async function confirmOne(id: number, applyToMatching: boolean) {
    setLastUndo(null)
    await api.confirm([id], applyToMatching)
    reload()
  }

  // Point 15: only the bulk-assign command (`api.bulkAssign`) is undoable;
  // the backend keeps a single most-recent slot, so a fresh undo id always
  // replaces whichever one is showing.
  async function bulkAssign(catId: number | null, applyToMatching: boolean) {
    if (catId === null) return
    const outcome = await api.bulkAssign([...selected], catId, applyToMatching)
    setSelected(new Set())
    setBulkCategoryId(null)
    setToast(outcome.skipped_transfers > 0 ? `Prevody sa nepriraďujú, preskočené: ${outcome.skipped_transfers}` : null)
    setLastUndo(outcome.undo_id ? { id: outcome.undo_id } : null)
    reload()
  }

  // Point 11: bulk confirm has no undo id from the backend, so it never
  // touches lastUndo other than invalidating a stale one from an earlier
  // bulk assignment (the write itself makes that old undo id go stale).
  async function bulkConfirm(applyToMatching: boolean) {
    setLastUndo(null)
    const outcome = await api.confirm([...selected], applyToMatching)
    setSelected(new Set())
    setToast(`Potvrdených: ${outcome.updated}.`)
    reload()
  }

  async function undoLast() {
    if (!lastUndo) return
    const outcome = await api.undoLastAssignment(lastUndo.id)
    setLastUndo(null)
    setToast(outcome ? `Vrátených transakcií: ${outcome.restored_rows}.` : 'Vrátenie sa nepodarilo, akcia už nie je aktuálna.')
    if (outcome) reload()
  }

  // Section 9 U05: creation and assignment stay two distinct, explicit
  // actions. The category is already persisted once this fires; a bulk
  // context only preselects it (the existing Priradiť button still assigns),
  // while a single row's picker create IS itself that row's explicit
  // assignment choice, so it assigns immediately. Either way, a failed
  // assignment never re-creates the category: it stays available to retry.
  async function categoryCreated(category: Category) {
    setCategories((prev) => (prev.some((c) => c.id === category.id) ? prev : [...prev, category]))
    const context = createFor
    setCreateFor(null)
    if (context === 'bulk') { setBulkCategoryId(category.id); return }
    if (context) await action.run(() => assignOne(context.rowId, category.id))
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
        kind={kind}
        text={text}
        onAccount={setAccountId}
        onCategory={setCategoryId}
        onStatus={setStatus}
        onKind={setKind}
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
      {lastUndo ? (
        <div className="k-row k-well" role="status">
          <span>Priradenie dokončené.</span>
          <Button variant="ghost" onClick={() => void action.run(undoLast)}>Späť</Button>
        </div>
      ) : null}
      <p className="k-field-label">
        Vytvorenie kategórie nevytvorí pravidlo. Priradenie kategórie alebo potvrdenie návrhu pri rozpoznateľnom obchodníkovi vytvorí pravidlo pre ďalšie platby. Použiť aj na podobné navyše zaradí už importované nezaradené alebo navrhnuté platby rovnakého obchodníka.
      </p>
      <fieldset disabled={action.busy} className="k-section">
        <UnassignedGroupsBar rows={rows} onSelectGroup={(ids) => setSelected(new Set(ids))} />
        <BulkBar
          count={selected.size}
          categories={categories}
          categoryId={bulkCategoryId}
          onCategoryChange={setBulkCategoryId}
          onCreate={() => setCreateFor('bulk')}
          onAssign={(catId, applyToMatching) => void action.run(() => bulkAssign(catId, applyToMatching))}
          onConfirm={(applyToMatching) => void action.run(() => bulkConfirm(applyToMatching))}
        />
        <div className="k-table-scroll" role="region" aria-label="Transakcie" tabIndex={0} onKeyDown={onTableKeyDown}>
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
                matchingCount={row.status === 'suggested' ? countMatchingUnconfirmed(rows, row) : 0}
                focused={focusedRowId === row.id}
                onSelect={toggleSelect}
                onAssign={(id, catId) => void action.run(() => assignOne(id, catId))}
                onConfirm={(id, applyToMatching) => void action.run(() => confirmOne(id, applyToMatching))}
                onNoteSaved={refreshRows}
                onCreateCategory={(rowId) => setCreateFor({ rowId })}
              />
            ))}
          </tbody>
        </table>
        </div>
      </fieldset>
      <CategoryDialog open={createFor !== null} onClose={() => setCreateFor(null)} onCreated={(category) => void categoryCreated(category)} />
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
