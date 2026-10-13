import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode, type RefObject } from 'react'
import type { Account, AccountKind, Category, RuleView, Status, TxFilter, TxKind, TxRow } from '../api'
import { api, formatEur } from '../api'
import { Button } from '../components/Button'
import { CategoryDialog } from '../components/CategoryDialog'
import { CategoryPicker } from '../components/CategoryPicker'
import { PeriodPicker, usePeriod, type PeriodValue } from '../components/PeriodPicker'
import { OperationStatus } from '../components/OperationStatus'
import { MoreMenu } from '../components/MoreMenu'
import { StateLabel, StateMark } from '../components/StateLabel'
import { ExportPdfDialog } from '../components/report/ExportPdfDialog'
import { footerNote } from '../kaliber/activity'
import { binding, withKey } from '../kaliber/bindings'
import { popLayer, pushLayer, searchLayer, useEscapeLayer } from '../kaliber/escape'
import { capNote, changedKeys, InkCap, INK_HOLD_MS } from '../kaliber/ink'
import { keyLabel, matchKey } from '../kaliber/keys'
import { dropToastsWithAction, toast } from '../kaliber/toast'
import { isMoveKey, nextFocus, rangeIds, selectAll } from '../lib/register-keys'
import { NoteEditor } from '../components/NoteEditor'
import { RecurringTransactionAction } from '../components/recurring'
import { statusLabel } from '../lib/categories'
import { categoryApi } from '../lib/category-api'
import { formatDate } from '../lib/format'
import { countPhrase, pluralSk, tx, vb } from '../lib/plural'
import { files } from '../lib/files'
import { useAction } from '../lib/useAction'
import { periodRange, validPeriod } from '../lib/period'
import { groupUnassignedTransactions, type TransactionDisplayGroup } from '../lib/transaction-groups'

const TEXT_DEBOUNCE_MS = 300
// `--motion-base` is 120 ms: the class stays a little longer, then goes, so the next change draws again.
const RULE_DRAW_MS = 400
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
  const name = group.place ? `${merchant}, ${group.place}` : merchant
  return `${name}: ${countPhrase(group.count, tx.platba)}`
}

const NOTE_GROUPS = 3

// The one ink row near the top: it exists only while a real task waits (unassigned payments),
// carries the count and its split by merchant, and one action that opens the group list.
function WaitingRow({ groups, count, open, onToggle }: { groups: TransactionDisplayGroup[]; count: number; open: boolean; onToggle: () => void }) {
  if (count === 0) return null
  const shown = groups.slice(0, NOTE_GROUPS).map((g) => `${groupLabel(g)}.`).join(' ')
  const more = groups.length > NOTE_GROUPS ? ` Ďalšie skupiny: ${groups.length - NOTE_GROUPS}.` : ''
  const verb = pluralSk(count, vb.caka)
  return (
    <div className="k-ink-row" role="status" aria-label="Čakajú na zaradenie">
      <div>
        <div className="k-ink-row-text">{countPhrase(count, tx.platba)} {verb} na zaradenie</div>
        <div className="k-ink-row-note">{shown}{more}</div>
      </div>
      <button type="button" className="k-ink-action" aria-expanded={open} aria-controls="ab-unassigned-groups" onClick={onToggle}>
        Zaradiť podľa obchodníka
      </button>
    </div>
  )
}

function UnassignedGroupsBar({ groups, onSelectGroup }: { groups: TransactionDisplayGroup[]; onSelectGroup: (ids: number[]) => void }) {
  if (groups.length === 0) return null
  return (
    <div id="ab-unassigned-groups" className="ab-row k-well" role="region" aria-label="Skupiny nezaradených platieb">
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
  searchRef,
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
  searchRef?: RefObject<HTMLInputElement | null>
}) {
  return (
    <div className="ab-filter-row">
      <SelectField id="f-acc" label="Účet" value={accountId ?? ''} onChange={(v) => onAccount(v ? Number(v) : null)}>
        <option value="">Všetky účty</option>
        {accounts.map((a) => (
          <option key={a.id} value={a.id}>
            {a.kind === 'personal' ? 'Osobný' : 'Firemný'} - {a.label}
          </option>
        ))}
      </SelectField>
      <SelectField id="f-st" label="Stav" value={status ?? ''} onChange={(v) => onStatus((v || null) as Status | null)}>
        <option value="">Všetky stavy</option>
        {STATUSES.map((s) => (
          <option key={s} value={s}>
            {statusLabel(s)}
          </option>
        ))}
      </SelectField>
      <div className="k-select-field ab-filter-field">
        <span className="ab-field-name">Kategória</span>
        <CategoryPicker mode="filter" label="Kategória" emptyLabel="Všetky kategórie" value={categoryId} onChange={onCategory} categories={categories} />
      </div>
      <SelectField id="f-kind" label="Druh" value={kind ?? ''} onChange={(v) => onKind((v || null) as TxKind | null)}>
        <option value="">Všetky druhy</option>
        {TX_KINDS.map((k) => (
          <option key={k} value={k}>
            {TX_KIND_LABELS[k]}
          </option>
        ))}
      </SelectField>
      <div className="k-input-field ab-filter-field ab-filter-search">
        <label htmlFor="f-q">Hľadať</label>
        <input
          id="f-q"
          ref={searchRef}
          type="search"
          data-k-search=""
          className="k-input"
          aria-label="Hľadať obchodníka alebo poznámku"
          aria-keyshortcuts="Control+F"
          title={withKey('Hľadať', 'find')}
          placeholder="Obchodník alebo poznámka"
          value={text}
          onChange={(e) => onText(e.target.value)}
        />
      </div>
    </div>
  )
}

// A select with a permanent label above it (Kaliber: a select never lives on its placeholder).
function SelectField({ id, label, value, onChange, children }: { id: string; label: string; value: string | number; onChange: (value: string) => void; children: ReactNode }) {
  return (
    <div className="k-select-field ab-filter-field">
      <label htmlFor={id}>{label}</label>
      <span className="k-select-wrap">
        <select id={id} className="k-select k-well" value={value} onChange={(e) => onChange(e.target.value)}>
          {children}
        </select>
        <svg className="k-select-chevron" viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
          <path d="M2.5 4.5 6 8l3.5-3.5" fill="none" stroke="currentColor" strokeWidth="1.5" />
        </svg>
      </span>
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
    <div className="ab-row k-well">
      <span>Vybrané: {count}</span>
      <CategoryPicker value={categoryId} onChange={onCategoryChange} categories={categories} onCreate={onCreate} />
      <label className="ab-checkbox">
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
    <div className="ab-row">
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

interface BulkAssignment {
  ids: number[]
  catId: number
  applyToMatching: boolean
}

function rowClass(focused: boolean, selected: boolean, inked: boolean): string {
  return ['k-ink', focused && 'is-active', selected && 'is-selected', inked && 'is-inked'].filter(Boolean).join(' ')
}

export function TransactionRow({
  row,
  categories,
  selected,
  matchingCount = 0,
  accountLabel,
  focused = false,
  inked = false,
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
  // The account's own name (Bezny ucet), when the screen knows it; the kind word otherwise.
  accountLabel?: string
  // Point 12: true for the row currently holding the keyboard roving highlight.
  focused?: boolean
  // Kaliber F2: the row's category or state changed by the last write (entry ink).
  inked?: boolean
  onSelect: (id: number, checked: boolean, range?: boolean) => void
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
      <tr className={rowClass(focused, selected, inked)} data-row-id={row.id}>
        <td>
          <input
            aria-label={`Vybrať transakciu ${row.id}: ${row.merchant_raw}`}
            type="checkbox"
            checked={selected}
            disabled={isTransfer}
            onChange={() => {}}
            onClick={(e) => onSelect(row.id, e.currentTarget.checked, e.shiftKey)}
          />
        </td>
        <td className="ab-hide-narrow k-num">{formatDate(row.tx_date)}</td>
        <td className="ab-hide-narrow ab-muted">{accountLabel ?? (row.account_kind === 'personal' ? 'Osobný' : 'Firemný')}</td>
        <td className="ab-merch">
          <b>{row.merchant_raw}</b>
          {row.place ? <small>{row.place}</small> : null}
          <small className="ab-narrow-only">{formatDate(row.tx_date)}</small>
        </td>
        <td className="ab-cat">
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
          <StateLabel status={row.status} />
        </td>
        <td className="ab-act">
          {row.status === 'suggested' ? (
            <>
              <Button variant="secondary" onClick={() => onConfirm(row.id, applyToMatching)}>
                Potvrdiť
              </Button>
              {matchingCount > 0 ? (
                <label className="ab-checkbox">
                  <input type="checkbox" checked={applyToMatching} onChange={(e) => setApplyToMatching(e.target.checked)} />
                  {`Potvrdiť aj podobné (${matchingCount})`}
                </label>
              ) : null}
            </>
          ) : null}
        </td>
        <td className="k-num k-cell-number ab-money">{formatEur(row.amount_cents)}</td>
        <td>
          <Button aria-label={`Detail transakcie ${row.id}`} aria-expanded={expanded} variant="ghost" onClick={() => setExpanded((v) => !v)}>
            {expanded ? '▾' : '▸'}
          </Button>
        </td>
      </tr>
      {expanded ? (
        <tr>
          <td colSpan={9}>
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
  onNavigateToImport,
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
  onNavigateToImport?: () => void
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
  const [anchorId, setAnchorId] = useState<number | null>(null)
  const [inked, setInked] = useState<ReadonlySet<number>>(new Set())
  const [pdfOpen, setPdfOpen] = useState(false)
  const [groupsOpen, setGroupsOpen] = useState(false)
  // R32: the total rule draws once after a confirmed change (a write that changed rows).
  const [ruleDrawn, setRuleDrawn] = useState(false)
  const searchRef = useRef<HTMLInputElement>(null)
  const tableRef = useRef<HTMLDivElement>(null)
  // F2 entry ink: the signatures of the last loaded rows, and whether the next load follows a
  // write (only a write inks; a filter change or a refresh never does).
  const lastSignatures = useRef<Map<number, string> | null>(null)
  const inkNextLoad = useRef(false)
  const inkCap = useRef(new InkCap())
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
  const lastBulk = useRef<BulkAssignment | null>(null)
  const redoBulk = useRef<BulkAssignment | null>(null)
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
  const reloadAfterWrite = useCallback(() => { inkNextLoad.current = true; setRevision((value) => value + 1) }, [])
  const fetchRequest = useRef(0)

  // Shared by the filter-driven effect below and by a note save's own
  // refresh, so whichever request is actually the latest always wins: a
  // note-triggered refresh in flight when the filter changes again never
  // overwrites rows with a stale answer, and vice versa.
  function inkRows(data: TxRow[]) {
    const next = new Map(data.map((r) => [r.id, `${r.status}|${r.category_id ?? ''}`]))
    const changed = inkNextLoad.current ? changedKeys(lastSignatures.current, next) : []
    inkNextLoad.current = false
    lastSignatures.current = next
    if (changed.length === 0) return
    setRuleDrawn(true)
    if (!inkCap.current.admit(changed.length, Date.now())) {
      setInked(new Set())
      footerNote(capNote(changed.map((id) => data.find((r) => r.id === id)?.merchant_raw ?? String(id))))
      return
    }
    setInked(new Set(changed))
  }

  useEffect(() => {
    if (!ruleDrawn) return
    const timer = setTimeout(() => setRuleDrawn(false), RULE_DRAW_MS)
    return () => clearTimeout(timer)
  }, [ruleDrawn])

  useEffect(() => {
    if (inked.size === 0) return
    const timer = setTimeout(() => setInked(new Set()), INK_HOLD_MS)
    return () => clearTimeout(timer)
  }, [inked])

  async function fetchRows(): Promise<void> {
    const request = ++fetchRequest.current
    try {
      const data = await api.listTransactions(filter)
      if (request !== fetchRequest.current) return
      inkRows(data)
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
    forgetUndo()
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

  // The register keys (Kaliber R22, docs/plan-091-kaliber-r3.md): the table region owns them
  // once the key comes from something that is not a form field, so the search box, a note and
  // the CategoryPicker's own search keep typing normal keys.
  function onTableKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (isFormField(event.target)) return
    if (matchKey(event.nativeEvent, 'primary+a')) {
      event.preventDefault()
      setSelected((prev) => selectAll(rows, prev))
      return
    }
    if (moveWithKey(event)) return
    rowKey(event)
  }

  // Arrows, Home, End, PgUp and PgDn move the focus; with Shift they extend the selection.
  function moveWithKey(event: KeyboardEvent<HTMLDivElement>): boolean {
    if (!isMoveKey(event.key) || event.ctrlKey || event.metaKey || event.altKey) return false
    event.preventDefault()
    const next = nextFocus(rows, focusedRowId, event.key)
    setFocusedRowId(next)
    if (!event.shiftKey || next === null) return true
    const from = anchorId ?? focusedRowId ?? next
    setAnchorId(from)
    setSelected((prev) => new Set([...prev, ...rangeIds(rows, from, next)]))
    return true
  }

  // Space toggles the focused row; Enter accepts its suggestion.
  function rowKey(event: KeyboardEvent<HTMLDivElement>) {
    const focusedRow = rows.find((r) => r.id === focusedRowId)
    if (!focusedRow) return
    if (matchKey(event.nativeEvent, 'space')) {
      event.preventDefault()
      if (focusedRow.status !== 'transfer') toggleSelect(focusedRow.id, !selected.has(focusedRow.id))
      return
    }
    if (event.key !== 'Enter' || focusedRow.status !== 'suggested') return
    event.preventDefault()
    void action.run(() => confirmOne(focusedRow.id, false))
  }

  // Escape layer 6: the search field with text clears first (the focus stays in it).
  useEffect(() => {
    pushLayer(searchLayer)
    return () => popLayer(searchLayer)
  }, [])

  // Escape layer 7: a selection clears, the rows stay and the focus stays.
  useEscapeLayer('selection', selected.size > 0, () => setSelected(new Set()))

  // Ctrl+F focuses the filter field (WebView2 would open its own find bar); Ctrl+Z undoes the
  // last bulk assignment, but never inside a text field, where Ctrl+Z undoes typing.
  const latestUndo = useRef<() => void>(() => {})
  latestUndo.current = () => {
    const id = lastUndo?.id
    if (id && !action.busy) void action.run(() => undoAssignment(id))
  }
  const latestRedo = useRef<() => void>(() => {})
  latestRedo.current = () => {
    const bulk = redoBulk.current
    if (!bulk || action.busy) return
    redoBulk.current = null
    void action.run(() => assignBulk(bulk))
  }
  useEffect(() => {
    function onKeyDown(event: globalThis.KeyboardEvent) {
      if (document.querySelector('[aria-modal="true"]')) return
      if (matchKey(event, 'primary+f')) {
        event.preventDefault()
        searchRef.current?.focus()
        searchRef.current?.select()
        return
      }
      if (isFormField(event.target)) return
      if (matchKey(event, 'primary+z')) {
        event.preventDefault()
        latestUndo.current()
        return
      }
      if (matchKey(event, 'primary+y') || matchKey(event, 'primary+shift+z')) {
        event.preventDefault()
        latestRedo.current()
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  function clearFilters() {
    setPeriod({ kind: 'all' }); setAccountId(null); setCategoryId(null)
    setStatus(null); setKind(null); setText(''); setDrilldown({ statementId: undefined, accountKind: undefined })
    setSelected(new Set())
  }

  async function exportCsv() {
    if (!effectivePeriodValid) return
    const currentFilter = { ...filter, text: text || null }
    const path = await files.saveCsv()
    if (!path) { toast('Export zrušený.'); return }
    const count = await api.exportCsv(currentFilter, path)
    toast(`Exportovaných transakcií: ${count}.`)
  }

  // A click toggles one row and moves the anchor; Shift+click selects from the anchor.
  function toggleSelect(id: number, checked: boolean, range = false) {
    setFocusedRowId(id)
    if (range && anchorId !== null) {
      setSelected((prev) => new Set([...prev, ...rangeIds(rows, anchorId, id)]))
      return
    }
    setAnchorId(id)
    setSelected((prev) => {
      const next = new Set(prev)
      if (checked) next.add(id)
      else next.delete(id)
      return next
    })
  }

  function forgetUndo() {
    setLastUndo(null)
    redoBulk.current = null
    dropToastsWithAction()
  }

  async function assignOne(id: number, catId: number | null) {
    if (catId === null) return
    forgetUndo()
    await api.assign([id], catId, false)
    reloadAfterWrite()
  }

  async function confirmOne(id: number, applyToMatching: boolean) {
    forgetUndo()
    await api.confirm([id], applyToMatching)
    reloadAfterWrite()
  }

  // Point 15: only the bulk-assign command (`api.bulkAssign`) is undoable;
  // the backend keeps a single most-recent slot, so a fresh undo id always
  // replaces whichever one is showing.
  async function bulkAssign(catId: number | null, applyToMatching: boolean) {
    if (catId === null) return
    await assignBulk({ ids: [...selected], catId, applyToMatching })
  }

  // The one undoable command. Redo (Ctrl+Y, Ctrl+Shift+Z) runs the same assignment again
  // after an undo; any other write drops the redo (forgetUndo).
  async function assignBulk(bulk: BulkAssignment) {
    const outcome = await api.bulkAssign(bulk.ids, bulk.catId, bulk.applyToMatching)
    lastBulk.current = bulk
    redoBulk.current = null
    setSelected(new Set())
    setBulkCategoryId(null)
    dropToastsWithAction()
    if (outcome.skipped_transfers > 0) toast(`Prevody sa nepriraďujú, preskočené: ${outcome.skipped_transfers}`)
    setLastUndo(outcome.undo_id ? { id: outcome.undo_id } : null)
    const undoId = outcome.undo_id
    if (undoId) toast('Priradenie dokončené.', { label: 'Späť', run: () => void action.run(() => undoAssignment(undoId)) })
    reloadAfterWrite()
  }

  // Point 11: bulk confirm has no undo id from the backend, so it never
  // touches lastUndo other than invalidating a stale one from an earlier
  // bulk assignment (the write itself makes that old undo id go stale).
  async function bulkConfirm(applyToMatching: boolean) {
    forgetUndo()
    const outcome = await api.confirm([...selected], applyToMatching)
    setSelected(new Set())
    toast(`Potvrdených: ${outcome.updated}.`)
    reloadAfterWrite()
  }

  async function undoAssignment(id: string) {
    const outcome = await api.undoLastAssignment(id)
    forgetUndo()
    toast(outcome ? `Vrátených transakcií: ${outcome.restored_rows}.` : 'Vrátenie sa nepodarilo, akcia už nie je aktuálna.')
    if (!outcome) return
    redoBulk.current = lastBulk.current
    reloadAfterWrite()
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

  const groups = useMemo(() => groupUnassignedTransactions(rows), [rows])
  const counts = countStatuses(rows)
  const accountLabels = useMemo(() => new Map(accounts.map((a) => [a.id, a.label])), [accounts])
  return (
    <div className="ab-screen">
      <ScreenHead
        drilldown={drilldown}
        onNavigateToImport={onNavigateToImport}
        menu={[
          { label: 'Obnoviť', onSelect: reload },
          { label: 'Exportovať filtrované CSV', disabled: exportAction.busy || !effectivePeriodValid, onSelect: () => void exportAction.run(exportCsv) },
          { label: 'Exportovať PDF', onSelect: () => setPdfOpen(true) },
        ]}
      />
      <WaitingRow groups={groups} count={counts.unassigned} open={groupsOpen} onToggle={() => setGroupsOpen((open) => !open)} />
      <section className="k-sheet-surface ab-flush" aria-label="Zoznam transakcií">
        <div className="ab-filters">
          <PeriodRow statementDrilldown={!!drilldown.statementId} period={period} onPeriod={setPeriod} onClear={clearFilters} />
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
            searchRef={searchRef}
          />
        </div>
        <OperationStatus error={metadataError} />
        <LoadStatus loaded={loaded} error={loadError} valid={effectivePeriodValid} />
        <OperationStatus error={action.error} busy={action.busy} />
        <OperationStatus error={exportAction.error} busy={exportAction.busy} />
        <StateCounts rows={rows} counts={counts} loaded={loaded} />
        {lastUndo ? (
          <p className="ab-field-label ab-pad">Posledné hromadné priradenie sa dá vrátiť: {keyLabel(binding('undo').spec)}.</p>
        ) : null}
        <fieldset disabled={action.busy} className="ab-section">
          <div className="ab-section ab-pad">
          {groupsOpen ? <UnassignedGroupsBar groups={groups} onSelectGroup={(ids) => setSelected(new Set(ids))} /> : null}
          <BulkBar
            count={selected.size}
            categories={categories}
            categoryId={bulkCategoryId}
            onCategoryChange={setBulkCategoryId}
            onCreate={() => setCreateFor('bulk')}
            onAssign={(catId, applyToMatching) => void action.run(() => bulkAssign(catId, applyToMatching))}
            onConfirm={(applyToMatching) => void action.run(() => bulkConfirm(applyToMatching))}
          />
          <p className="ab-field-label ab-measure" id="ab-register-keys">
            Klávesy v tabuľke: šípky a Home, End, PgUp, PgDn presúvajú, medzerník vyberie riadok, Enter prijme odhad, Ctrl+A vyberie všetko, Esc zruší výber. Zoznam je v Pomocníkovi (F1).
          </p>
          <details className="ab-fold">
            <summary>Ako sa vytvárajú pravidlá</summary>
            <p className="ab-field-label ab-measure">
              Vytvorenie kategórie nevytvorí pravidlo. Priradenie kategórie alebo potvrdenie návrhu pri rozpoznateľnom obchodníkovi vytvorí pravidlo pre ďalšie platby. Použiť aj na podobné navyše zaradí už importované nezaradené alebo navrhnuté platby rovnakého obchodníka.
            </p>
          </details>
          </div>
          <div ref={tableRef} className="ab-table-scroll" role="region" aria-label="Transakcie" aria-describedby="ab-register-keys" tabIndex={0} onKeyDown={onTableKeyDown}>
            <table className="k-table ab-register">
              <thead>
                <tr>
                  <th></th>
                  <th className="ab-hide-narrow">Dátum</th>
                  <th className="ab-hide-narrow">Účet</th>
                  <th>Obchodník</th>
                  <th>Kategória</th>
                  <th>Stav</th>
                  <th><span className="k-sr-only">Akcia</span></th>
                  <th className="k-num k-cell-number ab-money">Suma</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {loaded && rows.length === 0 ? <EmptyRegisterRow onNavigateToImport={onNavigateToImport} /> : null}
                {rows.map((row) => (
                  <TransactionRow
                    key={row.id}
                    row={row}
                    categories={categories}
                    selected={selected.has(row.id)}
                    matchingCount={row.status === 'suggested' ? countMatchingUnconfirmed(rows, row) : 0}
                    accountLabel={accountLabels.get(row.account_id)}
                    focused={focusedRowId === row.id}
                    inked={inked.has(row.id)}
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
        <FilteredTotals rows={rows} categories={categories} loaded={loaded} categoriesLoaded={metadataLoaded} drawn={ruleDrawn} />
      </section>
      <ExportPdfDialog open={pdfOpen} onClose={() => setPdfOpen(false)} />
      <CategoryDialog open={createFor !== null} onClose={() => setCreateFor(null)} onCreated={(category) => void categoryCreated(category)} />
    </div>
  )
}

type MenuItem = { label: string; disabled?: boolean; onSelect: () => void }

// Page head of the r4 gallery screen: the title, a short meta line for a drill-down, then the
// import shortcut and the "Ďalšie" menu.
function ScreenHead({ drilldown, onNavigateToImport, menu }: {
  drilldown: { statementId?: number | null; accountKind?: AccountKind | null }
  onNavigateToImport?: () => void
  menu: MenuItem[]
}) {
  const meta = [
    drilldown.statementId ? 'Vybraný výpis, všetky jeho dátumy.' : '',
    drilldown.accountKind ? `Druh účtu: ${drilldown.accountKind === 'personal' ? 'Osobný' : 'Firemný'}.` : '',
  ].filter(Boolean).join(' ')
  return (
    <div className="ab-head">
      <div className="k-page-head">
        <h2 className="k-page-title">Transakcie</h2>
        {meta ? <p className="k-page-meta">{meta}</p> : null}
      </div>
      <div className="ab-head-actions">
        {onNavigateToImport ? <Button variant="secondary" onClick={onNavigateToImport}>Import výpisu</Button> : null}
        <MoreMenu items={menu} />
      </div>
    </div>
  )
}

function PeriodRow({ statementDrilldown, period, onPeriod, onClear }: {
  statementDrilldown: boolean
  period: PeriodValue
  onPeriod: (value: PeriodValue) => void
  onClear: () => void
}) {
  return (
    <div className="ab-filter-row">
      {statementDrilldown ? null : <PeriodPicker value={period} onChange={onPeriod} />}
      <button type="button" className="k-link ab-filter-clear" onClick={onClear}>Vymazať všetky filtre</button>
    </div>
  )
}

function EmptyRegisterRow({ onNavigateToImport }: { onNavigateToImport?: () => void }) {
  return (
    <tr>
      <td colSpan={9}>
        <span>Za toto obdobie nič nie je. Skús iné obdobie hore.</span>{' '}
        {onNavigateToImport ? <Button variant="ghost" onClick={onNavigateToImport}>Importovať výpis</Button> : null}
      </td>
    </tr>
  )
}

const STATE_ORDER: Status[] = ['unassigned', 'suggested', 'confirmed', 'transfer']

function countStatuses(rows: TxRow[]): Record<Status, number> {
  const counts: Record<Status, number> = { unassigned: 0, suggested: 0, confirmed: 0, transfer: 0 }
  for (const row of rows) counts[row.status] += 1
  return counts
}

// The caption line of the register: the total count, then the count of each state with its mark.
function StateCounts({ rows, counts, loaded }: { rows: TxRow[]; counts: Record<Status, number>; loaded: boolean }) {
  if (!loaded) return null
  const forms = { unassigned: tx.nezaradene, suggested: tx.odhad, confirmed: tx.potvrdene, transfer: tx.prevod }
  return (
    <div className="ab-cap" role="status" aria-label="Počty podľa stavu">
      <b>{countPhrase(rows.length, tx.platba)}</b>
      {STATE_ORDER.filter((state) => state !== 'transfer' || counts.transfer > 0).map((state) => (
        <span key={state} className="k-state-label">
          <StateMark status={state} />
          {countPhrase(counts[state], forms[state])}
        </span>
      ))}
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

function FilteredTotals({ rows, categories, loaded, categoriesLoaded, drawn }: {
  rows: TxRow[]; categories: Category[]; loaded: boolean; categoriesLoaded: boolean; drawn: boolean
}) {
  if (!loaded || !categoriesLoaded) return null
  const kinds = new Map(categories.map((category) => [category.id, category.kind]))
  const money = rows.map((row) => rowMoney(row, kinds.get(row.category_id ?? -1)))
  const income = money.reduce((sum, row) => sum + row.income, 0)
  const expense = money.reduce((sum, row) => sum + row.expense, 0)
  const transfers = rows.filter((row) => row.status === 'transfer').length
  // The hidden colon keeps the text of each line readable as "Príjem: 1 850,00 €" for a screen reader.
  return <section aria-label="Súčty zobrazených transakcií" className="ab-pad">
    <p className="ab-field-label ab-ledger-cap">Zobrazené transakcie: {rows.length}</p>
    <div className="k-ledger ab-ledger-end">
      <div className="k-ledger-row"><span>Príjem<span className="k-sr-only">:</span></span>{' '}<span className="k-ledger-amount">{formatEur(income)}</span></div>
      <div className="k-ledger-row"><span>Výdavky<span className="k-sr-only">:</span></span>{' '}<span className="k-ledger-amount">{formatEur(expense)}</span></div>
      <div className={`k-ledger-total k-ledger-total--lead${drawn ? ' is-changed' : ''}`}><span>Čisté<span className="k-sr-only">:</span></span>{' '}<span className="k-ledger-amount">{formatEur(income - expense)}</span></div>
    </div>
    {transfers > 0 ? <p className="ab-field-label ab-ledger-cap">Prevody mimo súčtov: {transfers}</p> : null}
  </section>
}

function LoadStatus({ loaded, error, valid }: { loaded: boolean; error: string; valid: boolean }) {
  return <OperationStatus error={error} busy={!loaded && !error && valid}>Načítava sa...</OperationStatus>
}
