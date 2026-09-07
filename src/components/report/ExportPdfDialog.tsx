import { useEffect, useMemo, useRef, useState } from 'react'
import { api, type Account } from '../../api'
import { reportApi, type PdfReportOutcome, type ReportPreview, type ReportRequest } from '../../lib/report-api'
import { reportFiles } from '../../lib/report-files'
import { Button } from '../Button'
import { Dialog } from '../Dialog'
import { Field } from '../Field'
import { OperationStatus } from '../OperationStatus'

type PeriodChoice = 'month' | 'six_months' | 'year' | 'all_time'
type ScopeChoice = 'all' | 'personal' | 'business' | `account:${number}`

interface Draft {
  period: PeriodChoice
  month: string
  endingMonth: string
  year: string
  scope: ScopeChoice
}

function currentMonth(): string {
  const now = new Date()
  return `${now.getFullYear().toString().padStart(4, '0')}-${(now.getMonth() + 1).toString().padStart(2, '0')}`
}

function initialDraft(): Draft {
  const month = currentMonth()
  return { period: 'month', month, endingMonth: month, year: month.slice(0, 4), scope: 'all' }
}

function validMonth(value: string): boolean {
  return /^\d{4}-(0[1-9]|1[0-2])$/.test(value) && value >= '0001-01' && value <= '9999-12'
}

function validYear(value: string): boolean {
  return /^(?:[1-9]\d{0,3})$/.test(value) && Number(value) <= 9999
}

function requestFor(draft: Draft): ReportRequest | null {
  const scope = scopeFor(draft.scope)
  if (!scope) return null
  if (draft.period === 'month' && validMonth(draft.month)) return { period: { kind: 'month', month: draft.month }, scope }
  if (draft.period === 'six_months' && validMonth(draft.endingMonth)) return { period: { kind: 'six_months', ending_month: draft.endingMonth }, scope }
  if (draft.period === 'year' && validYear(draft.year)) return { period: { kind: 'year', year: Number(draft.year) }, scope }
  if (draft.period === 'all_time') return { period: { kind: 'all_time' }, scope }
  return null
}

function scopeFor(value: ScopeChoice): ReportRequest['scope'] | null {
  if (value === 'all') return { kind: 'all' }
  if (value === 'personal' || value === 'business') return { kind: 'kind', account_kind: value }
  const id = Number(value.slice('account:'.length))
  return Number.isSafeInteger(id) && id > 0 ? { kind: 'account', account_id: id } : null
}

function accountScope(account: Account): ScopeChoice {
  return `account:${account.id}`
}

function periodCaption(preview: ReportPreview): string {
  if (!preview.range) return 'Bez transakcií'
  return `${preview.range.from} až ${preview.range.to}`
}

function coverageCaption(preview: ReportPreview): string | null {
  const parts = [
    preview.accounts_without_statements ? `bez výpisov: ${preview.accounts_without_statements}` : '',
    preview.accounts_with_gaps ? `s medzerami: ${preview.accounts_with_gaps}` : '',
    preview.unverified_statement_count ? `neoverené výpisy: ${preview.unverified_statement_count}` : '',
    preview.invalid_statement_range_count ? `neplatné rozsahy výpisov: ${preview.invalid_statement_range_count}` : '',
  ].filter(Boolean)
  return parts.length ? `Upozornenia na pokrytie: ${parts.join(', ')}.` : null
}

function selectedPeriodLabel(draft: Draft): string {
  if (draft.period === 'month') return 'Mesiac'
  if (draft.period === 'six_months') return 'Šesť mesiacov'
  if (draft.period === 'year') return 'Rok'
  return 'Celé obdobie'
}

function PreviewDetails({ preview }: { preview: ReportPreview }) {
  const coverage = coverageCaption(preview)
  return <section className="report-preview" aria-label="Náhľad PDF reportu">
    <h2>Náhľad</h2>
    <dl>
      <div><dt>Obdobie</dt><dd>{periodCaption(preview)}</dd></div>
      <div><dt>Účty</dt><dd>{preview.scope_label} ({preview.accounts.length})</dd></div>
      <div><dt>Transakcie</dt><dd>{preview.transaction_count}</dd></div>
      <div><dt>Posledná transakcia</dt><dd>{preview.latest_transaction_date ?? 'Bez transakcií'}</dd></div>
      <div><dt>Údaje zachytené</dt><dd>{preview.captured_at}</dd></div>
    </dl>
    {preview.unfinished_period ? <p>Obdobie ešte neskončilo.</p> : null}
    {coverage ? <p>{coverage}</p> : null}
    <ul className="report-account-list">
      {preview.accounts.map((account) => <li key={account.id}>{account.label} ({account.kind === 'personal' ? 'osobný' : 'firemný'} účet, IBAN ···{account.iban_suffix})</li>)}
    </ul>
    <p>Všetky transakcie vo zvolenom období a účtoch. Filtre tabuľky sa nepoužijú.</p>
    <p>Náhľad sa môže zmeniť. PDF zachytí údaje pri uložení.</p>
  </section>
}

function SuccessDetails({ outcome }: { outcome: PdfReportOutcome }) {
  return <p role="status">PDF uložené: {outcome.path}. Strany: {outcome.pages}. Transakcie zachytené pri uložení: {outcome.report.transaction_count}.</p>
}

interface ExportDialogContentProps {
  accounts: Account[]
  accountsError: string
  accountsLoading: boolean
  busy: boolean
  draft: Draft
  exportError: string
  exporting: boolean
  invalidPeriod: boolean
  outcome: PdfReportOutcome | null
  preview: ReportPreview | null
  previewError: string
  previewLoading: boolean
  updateDraft: (change: Partial<Draft>) => void
}

function PeriodInputs({ draft, updateDraft }: Pick<ExportDialogContentProps, 'draft' | 'updateDraft'>) {
  if (draft.period === 'month') return <Field label="Mesiac"><input className="k-input k-well" type="month" min="0001-01" max="9999-12" value={draft.month} onChange={(event) => updateDraft({ month: event.target.value })} /></Field>
  if (draft.period === 'six_months') return <Field label="Koncový mesiac"><input className="k-input k-well" type="month" min="0001-01" max="9999-12" value={draft.endingMonth} onChange={(event) => updateDraft({ endingMonth: event.target.value })} /></Field>
  if (draft.period === 'year') return <Field label="Rok"><input className="k-input k-well" type="number" min="1" max="9999" step="1" value={draft.year} onChange={(event) => updateDraft({ year: event.target.value })} /></Field>
  return null
}

function ExportDialogContent(props: ExportDialogContentProps) {
  const { accounts, accountsError, accountsLoading, busy, draft, exportError, exporting, invalidPeriod, outcome, preview, previewError, previewLoading, updateDraft } = props
  return <div className="report-export-dialog">
    <p>Vyberte obdobie a účty pre úplný výpis. Filtre tabuľky sa nepoužijú.</p>
    <fieldset className="k-section" disabled={busy}>
      <Field label="Obdobie">
        <select className="k-select k-well" value={draft.period} onChange={(event) => updateDraft({ period: event.target.value as PeriodChoice })}>
          <option value="month">Mesiac</option>
          <option value="six_months">Šesť mesiacov</option>
          <option value="year">Rok</option>
          <option value="all_time">Celé obdobie</option>
        </select>
      </Field>
      <PeriodInputs draft={draft} updateDraft={updateDraft} />
      <Field label="Účty">
        <select className="k-select k-well" value={draft.scope} onChange={(event) => updateDraft({ scope: event.target.value as ScopeChoice })}>
          <option value="all">Všetky účty</option>
          <option value="personal">Osobné účty</option>
          <option value="business">Firemné účty</option>
          {accounts.map((account) => <option key={account.id} value={accountScope(account)}>{account.label} ({account.kind === 'personal' ? 'osobný' : 'firemný'})</option>)}
        </select>
      </Field>
    </fieldset>
    {invalidPeriod ? <p role="alert">Zadajte platný kalendárny {draft.period === 'year' ? 'rok' : 'mesiac'}.</p> : null}
    {draft.period === 'six_months' ? <p>Report zahŕňa šesť po sebe idúcich mesiacov končiacich vo vybranom mesiaci.</p> : null}
    {draft.period === 'all_time' ? <p>{selectedPeriodLabel(draft)} použije všetky uložené dátumy transakcií vo zvolených účtoch.</p> : null}
    <OperationStatus busy={accountsLoading} error={accountsError}>Načítavajú sa účty...</OperationStatus>
    <OperationStatus busy={previewLoading} error={previewError}>Pripravuje sa náhľad...</OperationStatus>
    <OperationStatus busy={exporting} error={exportError}>Vyberá sa miesto a vytvára PDF...</OperationStatus>
    {preview ? <PreviewDetails preview={preview} /> : null}
    {outcome ? <SuccessDetails outcome={outcome} /> : null}
  </div>
}

function ExportActions({ busy, canExport, close, exportReport }: { busy: boolean; canExport: boolean; close: () => void; exportReport: () => Promise<void> }) {
  return <>
    <Button variant="secondary" disabled={busy} onClick={close}>Zrušiť</Button>
    <Button disabled={!canExport} onClick={() => void exportReport()}>Uložiť PDF</Button>
  </>
}

export function ExportPdfDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [draft, setDraft] = useState(initialDraft)
  const [accounts, setAccounts] = useState<Account[]>([])
  const [accountsError, setAccountsError] = useState('')
  const [accountsLoading, setAccountsLoading] = useState(false)
  const [accountsReady, setAccountsReady] = useState(false)
  const [preview, setPreview] = useState<ReportPreview | null>(null)
  const [previewError, setPreviewError] = useState('')
  const [previewLoading, setPreviewLoading] = useState(false)
  const [exporting, setExporting] = useState(false)
  const [exportError, setExportError] = useState('')
  const [outcome, setOutcome] = useState<PdfReportOutcome | null>(null)
  const previewGeneration = useRef(0)
  const exportPending = useRef(false)
  const request = useMemo(() => requestFor(draft), [draft])
  const busy = accountsLoading || exporting

  useEffect(() => {
    if (!open) return
    let active = true
    setAccountsReady(false)
    setAccountsLoading(true)
    setAccountsError('')
    void api.listAccounts().then((value) => {
      if (active) setAccounts(value)
    }).catch((error: unknown) => {
      if (active) setAccountsError(String(error))
    }).finally(() => {
      if (active) {
        setAccountsLoading(false)
        setAccountsReady(true)
      }
    })
    return () => { active = false }
  }, [open])

  useEffect(() => {
    const generation = ++previewGeneration.current
    setPreview(null)
    setPreviewError('')
    setOutcome(null)
    if (!open || !request || !accountsReady || accountsError) {
      setPreviewLoading(false)
      return
    }
    setPreviewLoading(true)
    void reportApi.preview(request).then((value) => {
      if (previewGeneration.current === generation) setPreview(value)
    }).catch((error: unknown) => {
      if (previewGeneration.current === generation) setPreviewError(String(error))
    }).finally(() => {
      if (previewGeneration.current === generation) setPreviewLoading(false)
    })
  }, [accountsError, accountsReady, open, request])

  function updateDraft(change: Partial<Draft>) {
    if (!busy) setDraft((value) => ({ ...value, ...change }))
  }

  function close() {
    if (!busy) onClose()
  }

  async function exportReport() {
    if (!request || !preview || exportPending.current) return
    exportPending.current = true
    setExporting(true)
    setExportError('')
    setOutcome(null)
    try {
      const path = await reportFiles.save()
      if (!path) return
      const value = await reportApi.export(request, path)
      setOutcome(value)
    } catch (error) {
      setExportError(String(error))
    } finally {
      exportPending.current = false
      setExporting(false)
    }
  }

  const invalidPeriod = !request && !accountsLoading && !accountsError
  const canExport = Boolean(preview) && !busy && !previewLoading && !exportPending.current
  return <Dialog open={open} title="Exportovať PDF" onClose={close} actions={<ExportActions busy={busy} canExport={canExport} close={close} exportReport={exportReport} />}>
    <ExportDialogContent
      accounts={accounts}
      accountsError={accountsError}
      accountsLoading={accountsLoading}
      busy={busy}
      draft={draft}
      exportError={exportError}
      exporting={exporting}
      invalidPeriod={invalidPeriod}
      outcome={outcome}
      preview={preview}
      previewError={previewError}
      previewLoading={previewLoading}
      updateDraft={updateDraft}
    />
  </Dialog>
}
