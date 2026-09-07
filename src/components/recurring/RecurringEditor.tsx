import { useEffect, useRef, useState, type ComponentType } from 'react'
import type { Category, TxRow } from '../../api'
import type {
  Cadence,
  RecurringDecisionInput,
  RecurringDetail,
  RecurringQuery,
  RecurringRow,
  SaveRecurringRequest,
  TransactionRecurringContext,
} from '../../lib/recurring-api'
import { recurringApi } from '../../lib/recurring-api'
import { Button } from '../Button'
import { Dialog } from '../Dialog'
import { Field } from '../Field'
import type { RecurringCategoryDialogProps } from './category-dialog-props'
import { RecurringCategoryFields } from './RecurringCategoryFields'
import { RecurringMembership } from './RecurringMembership'
import { RecurringWideDialogStyle } from './recurring-dialog-style'

export type RecurringEditorSource =
  | { kind: 'row'; row: RecurringRow; query: RecurringQuery }
  | { kind: 'transaction'; tx: TxRow; context: TransactionRecurringContext; query: RecurringQuery }

const CADENCES: Cadence[] = ['monthly', 'quarterly', 'yearly']
const CADENCE_OPTION: Record<Cadence, string> = {
  monthly: 'mesačne',
  quarterly: 'štvrťročne',
  yearly: 'ročne',
}

export function RecurringEditor({
  source,
  categories,
  CategoryDialog,
  onClose,
  onChanged,
}: {
  source: RecurringEditorSource | null
  categories: Category[]
  CategoryDialog?: ComponentType<RecurringCategoryDialogProps>
  onClose: () => void
  onChanged: () => Promise<void>
}) {
  const sourceKey = source ? editorSourceKey(source) : ''
  const draft = useEditorDraft(source)
  const mutation = useEditorMutation(sourceKey, onChanged, onClose)
  const seedId = source ? seedTransactionId(source, draft.detail) : null
  const decisionId = source ? decisionIdOf(source) : null
  const forceSelected = source ? mustUseSelected(source) : false
  const members = source ? membershipPool(source, draft.detail) : []

  function close() {
    if (!mutation.busy) onClose()
  }

  function persist(decision: RecurringDecisionInput) {
    if (!source) return
    if (seedId === null) {
      mutation.setError('Chýba transakcia na uloženie výberu.')
      return
    }
    const request = buildRequest(decisionId, seedId, forceSelected || draft.useSelected, draft.selectedIds, decision)
    void mutation.run(JSON.stringify(request), () => recurringApi.save(request).then(() => undefined))
  }

  function restore() {
    if (decisionId === null) return
    void mutation.run(`reset:${decisionId}`, () => recurringApi.reset(decisionId))
  }

  return (
    <Dialog
      open={source !== null}
      title="Pravidelná platba"
      onClose={close}
      actions={(
        <EditorActions
          busy={mutation.busy}
          canReset={decisionId !== null}
          canSave={Boolean(draft.anchor)}
          onClose={close}
          onIgnore={() => persist({ mode: 'ignored' })}
          onReset={restore}
          onSave={() => persist({ mode: 'confirmed', cadence: draft.cadence, anchor_date: draft.anchor })}
        />
      )}
    >
      <EditorForm
        draft={draft}
        forceSelected={forceSelected}
        members={members}
        seedId={seedId}
        categories={categories}
        CategoryDialog={CategoryDialog}
        busy={mutation.busy}
        loadError={draft.loadError}
        error={mutation.error}
        refreshError={mutation.refreshError}
      />
    </Dialog>
  )
}

function EditorActions({
  busy,
  canReset,
  canSave,
  onClose,
  onIgnore,
  onReset,
  onSave,
}: {
  busy: boolean
  canReset: boolean
  canSave: boolean
  onClose: () => void
  onIgnore: () => void
  onReset: () => void
  onSave: () => void
}) {
  return (
    <>
      <Button variant="secondary" disabled={busy} onClick={onClose}>Zrušiť</Button>
      <Button variant="ghost" disabled={busy} onClick={onIgnore}>Toto nie je pravidelná platba</Button>
      <Button variant="ghost" disabled={busy || !canReset} onClick={onReset}>Obnoviť odhad</Button>
      <Button disabled={busy || !canSave} onClick={onSave}>Uložiť</Button>
    </>
  )
}

function EditorForm({
  draft,
  forceSelected,
  members,
  seedId,
  categories,
  CategoryDialog,
  busy,
  loadError,
  error,
  refreshError,
}: {
  draft: ReturnType<typeof useEditorDraft>
  forceSelected: boolean
  members: TxRow[]
  seedId: number | null
  categories: Category[]
  CategoryDialog?: ComponentType<RecurringCategoryDialogProps>
  busy: boolean
  loadError: string
  error: string
  refreshError: string
}) {
  return (
    <div data-recurring-wide>
      <RecurringWideDialogStyle />
      <EditorStatus loadError={loadError} busy={busy} error={error} refreshError={refreshError} />
      <Field label="Interval">
        <select className="k-select k-well" aria-label="Interval" value={draft.cadence} disabled={busy} onChange={(e) => draft.setCadence(e.target.value as Cadence)}>
          {CADENCES.map((item) => (
            <option key={item} value={item}>{CADENCE_OPTION[item]}</option>
          ))}
        </select>
      </Field>
      <Field label="Kotva">
        <input className="k-input k-well" type="date" aria-label="Kotva" value={draft.anchor} disabled={busy} onChange={(e) => draft.setAnchor(e.target.value)} />
      </Field>
      <label className="k-checkbox">
        <input
          type="checkbox"
          checked={forceSelected || draft.useSelected}
          disabled={busy || forceSelected}
          aria-label="Len označené transakcie"
          onChange={(e) => draft.setUseSelected(e.target.checked)}
        />
        Len označené transakcie
      </label>
      <SelectedMembership
        show={forceSelected || draft.useSelected}
        members={members}
        selectedIds={draft.selectedIds}
        disabled={busy}
        onToggle={draft.toggleMember}
      />
      <CategorySlot seedId={seedId} categories={categories} CategoryDialog={CategoryDialog} disabled={busy} />
    </div>
  )
}

function EditorStatus({ loadError, busy, error, refreshError }: { loadError: string; busy: boolean; error: string; refreshError: string }) {
  return (
    <>
      <Alert text={loadError} />
      {busy ? <p role="status">Ukladá sa...</p> : null}
      <Alert text={error} />
      <Alert text={refreshError ? `Uložené, obnovenie zlyhalo: ${refreshError}` : ''} />
    </>
  )
}

function Alert({ text }: { text: string }) {
  if (!text) return null
  return <p role="alert">{text}</p>
}

function SelectedMembership({
  show,
  members,
  selectedIds,
  disabled,
  onToggle,
}: {
  show: boolean
  members: TxRow[]
  selectedIds: number[]
  disabled: boolean
  onToggle: (id: number) => void
}) {
  if (!show) return null
  return <RecurringMembership members={members} selectedIds={selectedIds} disabled={disabled} onToggle={onToggle} />
}

function CategorySlot({
  seedId,
  categories,
  CategoryDialog,
  disabled,
}: {
  seedId: number | null
  categories: Category[]
  CategoryDialog?: ComponentType<RecurringCategoryDialogProps>
  disabled: boolean
}) {
  if (seedId === null) return null
  return (
    <RecurringCategoryFields
      categories={categories}
      transactionId={seedId}
      CategoryDialog={CategoryDialog}
      disabled={disabled}
    />
  )
}

function useEditorDraft(source: RecurringEditorSource | null) {
  const [detail, setDetail] = useState<RecurringDetail | null>(null)
  const [cadence, setCadence] = useState<Cadence>('monthly')
  const [anchor, setAnchor] = useState('')
  const [useSelected, setUseSelected] = useState(false)
  const [selectedIds, setSelectedIds] = useState<number[]>([])
  const [loadError, setLoadError] = useState('')
  const membershipTouched = useRef(false)
  const sourceKey = source ? editorSourceKey(source) : ''

  useEffect(() => applySource(source, {
    setDetail, setCadence, setAnchor, setUseSelected, setSelectedIds, setLoadError, membershipTouched,
  }), [sourceKey])

  function toggleMember(id: number) {
    membershipTouched.current = true
    setSelectedIds((current) => current.includes(id) ? current.filter((item) => item !== id) : [...current, id])
  }

  return { detail, cadence, setCadence, anchor, setAnchor, useSelected, setUseSelected, selectedIds, loadError, toggleMember }
}

function applySource(
  source: RecurringEditorSource | null,
  setters: {
    setDetail: (value: RecurringDetail | null) => void
    setCadence: (value: Cadence) => void
    setAnchor: (value: string) => void
    setUseSelected: (value: boolean) => void
    setSelectedIds: (value: number[]) => void
    setLoadError: (value: string) => void
    membershipTouched: { current: boolean }
  },
) {
  if (!source) {
    setters.setDetail(null)
    return
  }
  setters.setCadence(defaultCadence(source))
  setters.setAnchor(defaultAnchor(source))
  setters.setUseSelected(usesSelectedScope(source))
  setters.setSelectedIds(initialSelectedIds(source))
  setters.setLoadError('')
  setters.membershipTouched.current = false
  if (source.kind !== 'row') {
    setters.setDetail(null)
    return
  }
  let active = true
  void recurringApi.detail({ series_key: source.row.series_key, query: source.query }).then(
    (value) => {
      if (!active) return
      setters.setDetail(value)
      if (!setters.membershipTouched.current) setters.setSelectedIds(value.matching_transaction_ids)
    },
    (e) => { if (active) setters.setLoadError(String(e)) },
  )
  return () => { active = false }
}

function useEditorMutation(sourceKey: string, onChanged: () => Promise<void>, onClose: () => void) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [refreshError, setRefreshError] = useState('')
  const pending = useRef(false)
  const lastKey = useRef<string | null>(null)

  useEffect(() => {
    lastKey.current = null
    pending.current = false
    setBusy(false)
    setError('')
    setRefreshError('')
  }, [sourceKey])

  async function run(key: string, mutate: () => Promise<void>) {
    if (pending.current) return
    pending.current = true
    setBusy(true)
    setError('')
    setRefreshError('')
    try {
      if (lastKey.current !== key) {
        await mutate()
        lastKey.current = key
      }
      await onChanged()
      onClose()
    } catch (e) {
      if (lastKey.current === key) setRefreshError(String(e))
      else setError(String(e))
    } finally {
      pending.current = false
      setBusy(false)
    }
  }

  return { busy, error, refreshError, setError, run }
}

function editorSourceKey(source: RecurringEditorSource): string {
  return source.kind === 'row' ? `row:${source.row.series_key}` : `tx:${source.tx.id}`
}

function defaultCadence(source: RecurringEditorSource): Cadence {
  if (source.kind === 'row') return source.row.cadence ?? 'monthly'
  return source.context.decision?.cadence ?? source.context.inferred_cadence ?? 'monthly'
}

function defaultAnchor(source: RecurringEditorSource): string {
  if (source.kind === 'row') return source.row.anchor_date ?? source.query.today
  return source.context.decision?.anchor_date ?? source.tx.tx_date
}

function usesSelectedScope(source: RecurringEditorSource): boolean {
  return mustUseSelected(source) || (source.kind === 'row' && source.row.manual_membership)
}

function mustUseSelected(source: RecurringEditorSource): boolean {
  if (source.kind === 'row') return source.row.scope === 'selected'
  return source.context.decision?.scope === 'selected' || source.context.ambiguous || source.context.group_key === null
}

function decisionIdOf(source: RecurringEditorSource): number | null {
  if (source.kind === 'row') return source.row.decision_id
  return source.context.decision?.id ?? null
}

function initialSelectedIds(source: RecurringEditorSource): number[] {
  return source.kind === 'transaction' ? [source.tx.id] : []
}

function seedTransactionId(source: RecurringEditorSource, detail: RecurringDetail | null): number | null {
  if (source.kind === 'transaction') return source.tx.id
  if (detail?.matching_transaction_ids[0] !== undefined) return detail.matching_transaction_ids[0]
  return detail?.transactions[0]?.id ?? null
}

function membershipPool(source: RecurringEditorSource, detail: RecurringDetail | null): TxRow[] {
  if (source.kind === 'transaction') return uniqueTx([source.tx, ...source.context.compatible_transactions])
  if (!detail) return []
  return uniqueTx([...detail.transactions, ...detail.compatible_transactions])
}

function uniqueTx(rows: TxRow[]): TxRow[] {
  const byId = new Map<number, TxRow>()
  for (const tx of rows) byId.set(tx.id, tx)
  return [...byId.values()]
}

function buildRequest(
  decisionId: number | null,
  seedId: number,
  selected: boolean,
  selectedIds: number[],
  decision: RecurringDecisionInput,
): SaveRecurringRequest {
  const ids = selectedIds.length > 0 ? selectedIds : [seedId]
  const selection = selected
    ? { scope: 'selected' as const, transaction_ids: ids }
    : { scope: 'group' as const, transaction_id: seedId }
  return { decision_id: decisionId, selection, decision }
}
