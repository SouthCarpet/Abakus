import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { AccountKind, Category } from '../../api'
import { api } from '../../api'
import type { PeriodValue } from '../PeriodPicker'
import { Card } from '../Card'
import { Button } from '../Button'
import { periodRange, validPeriod } from '../../lib/period'
import {
  emptyPanelExplanation,
  localDateFromIso,
  partitionVisibleRows,
  snapshotCaption,
} from '../../lib/recurring-labels'
import type { RecurringOverview, RecurringQuery, RecurringRow, SaveRecurringRequest } from '../../lib/recurring-api'
import { recurringApi } from '../../lib/recurring-api'
import { RecurringDetailDialog } from './RecurringDetail'
import { RecurringEditor, type RecurringEditorSource } from './RecurringEditor'
import { RecurringKpis } from './RecurringKpis'
import { RecurringTable } from './RecurringTable'

export function RecurringPanel({
  period,
  accountKind,
  today,
}: {
  period: PeriodValue
  accountKind: 'all' | AccountKind
  today: string
}) {
  const query = useMemo(() => buildQuery(period, accountKind, today), [period, accountKind, today])
  const { overview, error, loading, refreshAfterSave } = useRecurringOverview(query)
  const [annual, setAnnual] = useState(false)
  const [busy, setBusy] = useState(false)
  const [mutError, setMutError] = useState('')
  const [refreshError, setRefreshError] = useState('')
  const [editor, setEditor] = useState<RecurringEditorSource | null>(null)
  const [detailKey, setDetailKey] = useState<string | null>(null)
  const [categories, setCategories] = useState<Category[]>([])
  const pending = useRef(false)
  const lastMutation = useRef<string | null>(null)

  useEffect(() => {
    if (!editor) return
    void api.listCategories().then(setCategories).catch((e) => setMutError(String(e)))
  }, [editor])

  async function runMutation(key: string, mutate: () => Promise<void>) {
    if (pending.current) return
    pending.current = true
    setBusy(true)
    setMutError('')
    setRefreshError('')
    try {
      if (lastMutation.current !== key) {
        await mutate()
        lastMutation.current = key
      }
      await refreshAfterSave()
    } catch (e) {
      if (lastMutation.current === key) setRefreshError(String(e))
      else setMutError(String(e))
    } finally {
      pending.current = false
      setBusy(false)
    }
  }

  async function confirmRow(row: RecurringRow) {
    if (!query) return
    if (!row.cadence || !row.anchor_date) {
      setEditor({ kind: 'row', row, query })
      return
    }
    try {
      const detail = await recurringApi.detail({ series_key: row.series_key, query })
      const seed = detail.matching_transaction_ids[0] ?? detail.transactions[0]?.id
      if (seed === undefined) {
        setMutError('Chýba transakcia na uloženie výberu.')
        return
      }
      const request = confirmRequest(row, row.cadence, row.anchor_date, seed, detail.matching_transaction_ids)
      await runMutation(JSON.stringify(request), () => recurringApi.save(request).then(() => undefined))
    } catch (e) {
      setMutError(String(e))
    }
  }

  async function ignoreRow(row: RecurringRow) {
    if (!query) return
    try {
      const detail = await recurringApi.detail({ series_key: row.series_key, query })
      const seed = detail.matching_transaction_ids[0] ?? detail.transactions[0]?.id
      if (seed === undefined) {
        setMutError('Chýba transakcia na uloženie výberu.')
        return
      }
      const request = ignoreRequest(row, seed, detail.matching_transaction_ids)
      await runMutation(JSON.stringify(request), () => recurringApi.save(request).then(() => undefined))
    } catch (e) {
      setMutError(String(e))
    }
  }

  async function resetRow(row: RecurringRow) {
    const decisionId = row.decision_id
    if (decisionId === null) return
    await runMutation(`reset:${decisionId}`, () => recurringApi.reset(decisionId))
  }

  if (!query) return null

  return (
    <Card title="Pravidelné platby">
      <p>{overview ? snapshotCaption(overview) : 'Panel pravidelných platieb.'}</p>
      {overview?.unfinished_period ? <p>Obdobie ešte neskončilo</p> : null}
      {loading ? <p role="status">Načítavajú sa pravidelné platby...</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      {mutError ? <p role="alert">{mutError}</p> : null}
      {refreshError ? <p role="alert">Uložené, obnovenie zlyhalo: {refreshError}</p> : null}
      {refreshError ? (
        <Button variant="secondary" disabled={busy} onClick={() => void runMutation(lastMutation.current ?? 'refresh', async () => undefined)}>
          Obnoviť prehľad
        </Button>
      ) : null}
      {overview && !loading ? (
        <OverviewBody
          overview={overview}
          today={today}
          annual={annual}
          onAnnualChange={setAnnual}
          busy={busy}
          query={query}
          onDetail={(row) => setDetailKey(row.series_key)}
          onConfirm={(row) => void confirmRow(row)}
          onEdit={(row) => setEditor({ kind: 'row', row, query })}
          onIgnore={(row) => void ignoreRow(row)}
          onReset={(row) => void resetRow(row)}
        />
      ) : null}
      <RecurringEditor
        source={editor}
        categories={categories}
        onClose={() => setEditor(null)}
        onChanged={refreshAfterSave}
      />
      <RecurringDetailDialog
        seriesKey={detailKey}
        query={query}
        onClose={() => setDetailKey(null)}
        onEdit={(row) => {
          setDetailKey(null)
          setEditor({ kind: 'row', row, query })
        }}
      />
    </Card>
  )
}

function OverviewBody({
  overview,
  today,
  annual,
  onAnnualChange,
  busy,
  query,
  onDetail,
  onConfirm,
  onEdit,
  onIgnore,
  onReset,
}: {
  overview: RecurringOverview
  today: string
  annual: boolean
  onAnnualChange: (annual: boolean) => void
  busy: boolean
  query: RecurringQuery
  onDetail: (row: RecurringRow) => void
  onConfirm: (row: RecurringRow) => void
  onEdit: (row: RecurringRow) => void
  onIgnore: (row: RecurringRow) => void
  onReset: (row: RecurringRow) => void
}) {
  if (overview.future_period) {
    return <p>Vybrané obdobie je v budúcnosti. Panel je prázdny, neukazuje dnešné dáta pod budúcim nadpisom.</p>
  }
  const parts = partitionVisibleRows(overview.rows)
  const empty = overview.rows.length === 0
  return (
    <>
      {!empty ? (
        <RecurringKpis overview={overview} today={today} annual={annual} onAnnualChange={onAnnualChange} />
      ) : null}
      {empty ? <p>{emptyPanelExplanation()}</p> : null}
      <RecurringTable caption="Odhady" rows={parts.estimates} busy={busy} onDetail={onDetail} onConfirm={onConfirm} onEdit={onEdit} onIgnore={onIgnore} onReset={onReset} />
      <RecurringTable caption="Výdavky" rows={parts.expenses} busy={busy} onDetail={onDetail} onConfirm={onConfirm} onEdit={onEdit} onIgnore={onIgnore} onReset={onReset} />
      <RecurringTable caption="Príjmy" rows={parts.incomes} busy={busy} onDetail={onDetail} onConfirm={onConfirm} onEdit={onEdit} onIgnore={onIgnore} onReset={onReset} />
      {parts.ignored.length > 0 ? (
        <details>
          <summary>Ignorované ({parts.ignored.length})</summary>
          <RecurringTable caption="Ignorované" rows={parts.ignored} busy={busy} onDetail={onDetail} onConfirm={onConfirm} onEdit={onEdit} onIgnore={onIgnore} onReset={onReset} />
        </details>
      ) : null}
      <p className="k-field-label">Dotaz {query.from ?? 'začiatok histórie'} až {query.to ?? 'bez konca'}, dnes {query.today}.</p>
    </>
  )
}

function useRecurringOverview(query: RecurringQuery | null) {
  const [overview, setOverview] = useState<RecurringOverview | null>(null)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)
  const gen = useRef(0)
  const queryKey = query ? `${query.from}|${query.to}|${query.account_kind}|${query.today}` : ''

  const reload = useCallback(async () => {
    if (!query) return
    const mine = ++gen.current
    setOverview(null)
    setLoading(true)
    setError('')
    try {
      const value = await recurringApi.overview(query)
      if (mine !== gen.current) return
      setOverview(value)
    } catch (e) {
      if (mine !== gen.current) return
      setOverview(null)
      setError(String(e))
    } finally {
      if (mine === gen.current) setLoading(false)
    }
  }, [queryKey])

  const refreshAfterSave = useCallback(async () => {
    if (!query) return
    const mine = ++gen.current
    const value = await recurringApi.overview(query)
    if (mine !== gen.current) return
    setOverview(value)
    setError('')
  }, [queryKey])

  useEffect(() => { void reload() }, [reload])
  return { overview, error, loading, reload, refreshAfterSave }
}

function buildQuery(period: PeriodValue, accountKind: 'all' | AccountKind, today: string): RecurringQuery | null {
  if (!validPeriod(period)) return null
  const { from, to } = periodRange(period.kind, localDateFromIso(today), period.custom)
  return { from, to, account_kind: accountKind === 'all' ? null : accountKind, today }
}

function confirmRequest(
  row: RecurringRow,
  cadence: NonNullable<RecurringRow['cadence']>,
  anchor: string,
  seed: number,
  matchingIds: number[],
): SaveRecurringRequest {
  const selection = row.scope === 'selected'
    ? { scope: 'selected' as const, transaction_ids: matchingIds.length > 0 ? matchingIds : [seed] }
    : { scope: 'group' as const, transaction_id: seed }
  return {
    decision_id: row.decision_id,
    selection,
    decision: { mode: 'confirmed', cadence, anchor_date: anchor },
  }
}

function ignoreRequest(row: RecurringRow, seed: number, matchingIds: number[]): SaveRecurringRequest {
  const selection = row.scope === 'selected'
    ? { scope: 'selected' as const, transaction_ids: matchingIds.length > 0 ? matchingIds : [seed] }
    : { scope: 'group' as const, transaction_id: seed }
  return { decision_id: row.decision_id, selection, decision: { mode: 'ignored' } }
}
