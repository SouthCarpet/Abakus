import { useCallback, useEffect, useRef, useState } from 'react'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-dialog'
import type { AccountKind, Checksum, ImportReport, StatementDeletePreview, StatementHistoryRow, StatementReview, StatementReviewStatus } from '../api'
import { api, formatEur } from '../api'
import { useAction } from '../lib/useAction'
import { Button } from '../components/Button'
import { PageHead } from '../components/PageHead'
import { ariaKeys, withKey } from '../kaliber/bindings'
import { startActivity } from '../kaliber/activity'
import { useDeleteKey } from '../kaliber/deleteKey'
import { TrashGlyph } from '../kaliber/icons'
import { Card } from '../components/Card'
import { Dialog } from '../components/Dialog'
import { Field } from '../components/Field'
import { PasswordInput } from '../components/PasswordInput'
import { SetupAccountDialog, type SetupAccountTarget } from '../components/SetupAccountDialog'
import { checksumLabel, formatDate, importStatusLabel } from '../lib/format'

// Point 19: the list used to stop at the 8 most recent imports. It now shows
// every statement; only the on-screen presentation collapses the older ones.
const VISIBLE_STATEMENTS = 10
const SUCCESS_STATUSES = new Set(['imported', 'already_imported'])

function fileName(path: string): string {
  const parts = path.split(/[\\/]/)
  return parts[parts.length - 1] || path
}

function PeriodLine({ report }: { report: ImportReport }) {
  if (!report.periodStart || !report.periodEnd) return null
  return (
    <span>
      {formatDate(report.periodStart)} .. {formatDate(report.periodEnd)}
    </span>
  )
}

function ChecksumRow({ checksum }: { checksum: Checksum | null }) {
  const danger = checksum?.status === 'off_by'
  return <span className={danger ? 'ab-text-danger' : undefined}>{checksumLabel(checksum)}</span>
}

function WarningsList({ warnings }: { warnings: string[] }) {
  if (warnings.length === 0) return null
  return (
    <ul>
      {warnings.map((w) => (
        <li key={w}>{w}</li>
      ))}
    </ul>
  )
}

// A17/F4: shows the backend's own failure text instead of the generic "Chyba" badge alone.
function ImportErrorMessage({ report }: { report: ImportReport }) {
  if (!report.message) return null
  return <p className="ab-text-danger">{report.message}</p>
}

function LockedAction({
  report,
  onPassword,
}: {
  report: ImportReport
  onPassword: (password: string, remember: boolean) => void
}) {
  const [passwordOpen, setPasswordOpen] = useState(false)
  const [password, setPassword] = useState('')
  const [remember, setRemember] = useState(false)
  if (report.status !== 'locked') return null

  function submit() {
    onPassword(password, remember)
    setPasswordOpen(false)
    setPassword('')
    setRemember(false)
  }

  return (
    <>
      <Button variant="secondary" onClick={() => setPasswordOpen(true)}>
        Zadať heslo
      </Button>
      <Dialog
        open={passwordOpen}
        title="Heslo k PDF"
        onClose={() => setPasswordOpen(false)}
        actions={
          <>
            <Button variant="secondary" onClick={() => setPasswordOpen(false)}>
              Zrušiť
            </Button>
            <Button variant="primary" onClick={submit}>
              Potvrdiť
            </Button>
          </>
        }
      >
        <Field label="Heslo">
          <PasswordInput value={password} onChange={setPassword} />
        </Field>
        <label className="ab-checkbox">
          <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
          Zapamätať pre tento účet
        </label>
      </Dialog>
    </>
  )
}

function UnknownAccountAction({
  report,
  onAddAccount,
}: {
  report: ImportReport
  onAddAccount: (iban: string, kind: AccountKind) => void
}) {
  if (report.status !== 'unknown_account' || !report.iban || !report.accountKind) return null
  const iban = report.iban
  const kind = report.accountKind
  return (
    <Button variant="secondary" onClick={() => onAddAccount(iban, kind)}>
      Pridať účet
    </Button>
  )
}

export function ImportResultCard({
  report,
  onPassword,
  onAddAccount,
  onContinue,
}: {
  report: ImportReport
  onPassword: (password: string, remember: boolean) => void
  onAddAccount: (iban: string, kind: AccountKind) => void
  onContinue?: (statementId?: number) => void
}) {
  const canContinue = onContinue && SUCCESS_STATUSES.has(report.status)
  return (
    <Card
      title={fileName(report.path)}
      badge={importStatusLabel(report.status)}
      footer={
        canContinue ? (
          <Button variant="primary" onClick={() => onContinue?.(report.statementId ?? undefined)}>
            Pokračovať na Transakcie
          </Button>
        ) : undefined
      }
    >
      <div className="ab-row">
        <span>{report.accountLabel ?? report.ibanMasked ?? '-'}</span>
        {report.statementNumber !== null ? <span className="k-num">č. {report.statementNumber}</span> : null}
        <PeriodLine report={report} />
      </div>
      <div className="ab-row">
        <span>
          {report.inserted} nových, {report.duplicates} duplicít
        </span>
        <ChecksumRow checksum={report.checksum} />
      </div>
      <ImportErrorMessage report={report} />
      <WarningsList warnings={report.warnings} />
      <LockedAction report={report} onPassword={onPassword} />
      <UnknownAccountAction report={report} onAddAccount={onAddAccount} />
    </Card>
  )
}

function toSetupTarget(report: ImportReport, password: string): SetupAccountTarget {
  return { path: report.path, iban: report.iban ?? '', ibanMasked: report.ibanMasked, kind: report.accountKind ?? 'personal', password }
}

// Shared by the automatic open (append, drop order) and the "Pridať účet"
// button reopen (front, the user asked for it now). Either way, a target
// already queued for the same file is replaced in place rather than
// duplicated.
function upsertQueue(prev: SetupAccountTarget[], targets: SetupAccountTarget[], toFront: boolean): SetupAccountTarget[] {
  const next = [...prev]
  for (const t of targets) {
    const i = next.findIndex((p) => p.path === t.path)
    if (i >= 0) next.splice(i, 1)
    if (toFront) next.unshift(t)
    else next.push(t)
  }
  return next
}

// Point 21: three states, in the fixed order the backend already applies
// (needs_attention beats evidence_incomplete beats no_open_checks). The
// no_open_checks label never claims the bank data are complete; it reports
// that today's checks found nothing open.
const REVIEW_STATUS_LABEL: Record<StatementReviewStatus, string> = {
  needs_attention: 'Vyžaduje pozornosť',
  evidence_incomplete: 'Chýbajú dôkazy',
  no_open_checks: 'Bez otvorených kontrol',
}

function reviewOpenItems(review: StatementReview): string[] {
  const items: string[] = []
  if (review.checksum.status !== 'ok') items.push(checksumLabel(review.checksum))
  if (review.parser_warnings === null) items.push('Upozornenia parsera nie sú známe')
  else items.push(...review.parser_warnings)
  if (review.unassigned_count > 0) items.push(`Nezaradených riadkov: ${review.unassigned_count}`)
  if (review.suggested_count > 0) items.push(`Odhadovaných riadkov: ${review.suggested_count}`)
  return items
}

// Point 21: owns its own fetch, like the rest of this app's independent
// panels, so one slow or failing review never blocks the statement row it
// belongs to, and a collapsed ("Staršie výpisy") row never fetches at all
// until it is actually rendered.
function StatementReviewChip({ statementId }: { statementId: number }) {
  const [review, setReview] = useState<StatementReview | null>(null)
  const [error, setError] = useState('')
  const [open, setOpen] = useState(false)

  useEffect(() => {
    let active = true
    setReview(null)
    setError('')
    setOpen(false)
    api
      .statementReview(statementId)
      .then((next) => { if (active) setReview(next) })
      .catch((e) => { if (active) setError(String(e)) })
    return () => { active = false }
  }, [statementId])

  if (error) return <span className="k-card-badge ab-text-danger">Kontrola sa nenačítala</span>
  if (!review) return <span className="k-card-badge">Načítava sa kontrola…</span>

  const items = reviewOpenItems(review)
  const danger = review.status === 'needs_attention'
  return (
    <span className="ab-review-chip">
      <Button
        variant="ghost"
        className={danger ? 'k-card-badge ab-text-danger' : 'k-card-badge'}
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        {REVIEW_STATUS_LABEL[review.status]}
      </Button>
      {open ? (
        <span className="ab-review-detail">
          {items.length === 0 ? (
            <span>Žiadne otvorené položky.</span>
          ) : (
            <ul>
              {items.map((item, i) => (
                <li key={i}>{item}</li>
              ))}
            </ul>
          )}
          {review.status === 'no_open_checks' ? <p>Nepotvrdzuje úplnosť bankových dát.</p> : null}
        </span>
      ) : null}
    </span>
  )
}

function StatementRow({
  statement,
  onNavigate,
  onDelete,
  rowProps,
}: {
  statement: StatementHistoryRow
  onNavigate: (statementId: number) => void
  onDelete: (statement: StatementHistoryRow) => void
  rowProps: (open?: () => void) => Record<string, unknown>
}) {
  const danger = statement.checksum.status === 'off_by'
  return (
    <div className="ab-round-row" {...rowProps(() => onDelete(statement))}>
      <span>{formatDate(statement.period_end)}</span>
      <span>{statement.account_label}</span>
      <span className="k-num">č. {statement.number}</span>
      <span className="k-num">{statement.transaction_count} transakcií</span>
      <span className="k-num">{formatEur(statement.total_cents)}</span>
      <span className={danger ? 'k-card-badge ab-text-danger' : 'k-card-badge'}>{checksumLabel(statement.checksum)}</span>
      <StatementReviewChip statementId={statement.statement_id} />
      <Button variant="ghost" onClick={() => onNavigate(statement.statement_id)}>
        Zobraziť transakcie
      </Button>
      {/* Kaliber R12: the danger colour shows only in the confirm step, so the row
          delete is the quiet ghost icon of the category rows. */}
      <Button
        variant="ghost"
        className="k-icon-btn ab-icon-hit"
        aria-label={`Zmazať výpis č. ${statement.number}`}
        title="Zmaže výpis natrvalo."
        onClick={() => onDelete(statement)}
      >
        <TrashGlyph />
      </Button>
    </div>
  )
}

function StatementListHead() {
  return (
    <div className="ab-round-row-head">
      <span>Dátum</span>
      <span>Účet</span>
      <span>Číslo</span>
      <span>Transakcie</span>
      <span>Suma</span>
      <span>Kontrolný súčet</span>
      <span>Kontrola</span>
      <span>Akcie</span>
    </div>
  )
}

// Newest first, by period end and then by ID, so the visible head of the
// list (before "Staršie výpisy") always shows the most recent activity.
function sortNewestFirst(rows: StatementHistoryRow[]): StatementHistoryRow[] {
  return [...rows].sort((a, b) => {
    if (a.period_end !== b.period_end) return a.period_end < b.period_end ? 1 : -1
    return b.statement_id - a.statement_id
  })
}

function StatementList({
  statements,
  onNavigate,
  onDelete,
  deletedRef,
}: {
  statements: StatementHistoryRow[]
  onNavigate: (statementId: number) => void
  onDelete: (statement: StatementHistoryRow) => void
  // Set by the list: the screen calls it when a confirmed delete succeeded (focus to the next row).
  deletedRef: { current: () => void }
}) {
  const [showOlder, setShowOlder] = useState(false)
  const sorted = sortNewestFirst(statements)
  const keys = useDeleteKey<HTMLDivElement>(statements.length)
  deletedRef.current = keys.deleted
  const visible = sorted.slice(0, VISIBLE_STATEMENTS)
  const older = sorted.slice(VISIBLE_STATEMENTS)

  return (
    <Card title="Všetky výpisy">
      {/* A phone (390 px) scrolls the statement rows inside this region, not the page. */}
      <div ref={keys.containerRef} className="ab-table-scroll" role="region" aria-label="Všetky výpisy" tabIndex={0}>
      {sorted.length === 0 ? (
        <p>Zatiaľ žiadne importy.</p>
      ) : (
        <div className="ab-round-list">
          <StatementListHead />
          {visible.map((s) => (
            <StatementRow key={s.statement_id} statement={s} onNavigate={onNavigate} onDelete={onDelete} rowProps={keys.rowProps} />
          ))}
        </div>
      )}
      {older.length > 0 ? (
        <>
          <Button variant="ghost" onClick={() => setShowOlder((v) => !v)}>
            {showOlder ? 'Zbaliť' : `Staršie výpisy (${older.length})`}
          </Button>
          {showOlder ? (
            <div className="ab-round-list">
              {older.map((s) => (
                <StatementRow key={s.statement_id} statement={s} onNavigate={onNavigate} onDelete={onDelete} rowProps={keys.rowProps} />
              ))}
            </div>
          ) : null}
        </>
      ) : null}
      </div>
    </Card>
  )
}

// A17/F1: what the confirm dialog reads before the user decides. Names the
// backend's own counts, never a guess, so the person deleting a statement
// knows exactly what disappears with it.
function DeleteStatementBody({ preview }: { preview: StatementDeletePreview | null }) {
  if (!preview) return <p>Načítava sa...</p>
  return (
    <p>
      {`Natrvalo sa zmaže výpis č. ${preview.number} z účtu ${preview.account_label}. Odstráni sa ${preview.transaction_count} transakcií, z nich ${preview.confirmed_count} potvrdených. Pravidlá, ktoré nepoužíva iný výpis: ${preview.rules_deleted}. Ostatné výpisy zostanú zachované. Otvorené transakcie sa môžu znovu zaradiť podľa zostávajúcich pravidiel.`}
    </p>
  )
}

// `pickRequest` (Ctrl+O from anywhere, App): a positive value opens the file chooser once;
// `onPickRequestHandled` lets App reset it, so a later visit does not open it again.
// Ctrl+O: a positive request opens the file chooser once, then App resets it.
function usePickRequest(pickRequest: number | undefined, onHandled: (() => void) | undefined, pick: () => void) {
  useEffect(() => {
    if (!pickRequest || pickRequest <= 0) return
    if (onHandled) onHandled()
    pick()
  }, [pickRequest])
}

function PickButton({ busy, onPick }: { busy: boolean; onPick: () => void }) {
  return (
    <Button variant="primary" disabled={busy} title={withKey('Vybrať PDF', 'import')} aria-keyshortcuts={ariaKeys('import')} onClick={onPick}>
      Vybrať PDF
    </Button>
  )
}

export function Import({ onNavigateToTransactions, pickRequest, onPickRequestHandled }: {
  onNavigateToTransactions?: (statementId?: number) => void
  pickRequest?: number
  onPickRequestHandled?: () => void
}) {
  const action = useAction()
  const deletion = useAction()
  const previewRequest = useRef(0)
  const statementDeleted = useRef<() => void>(() => {})
  const statementsRequest = useRef(0)
  const [previewError, setPreviewError] = useState('')
  const [reports, setReports] = useState<ImportReport[]>([])
  const [statements, setStatements] = useState<StatementHistoryRow[]>([])
  const [unknownQueue, setUnknownQueue] = useState<SetupAccountTarget[]>([])
  const [deleteTarget, setDeleteTarget] = useState<StatementHistoryRow | null>(null)
  const [deletePreview, setDeletePreview] = useState<StatementDeletePreview | null>(null)
  const canDrop = useRef(true)
  canDrop.current = deleteTarget === null && unknownQueue.length === 0
  const passwordsRef = useRef<Record<string, { password: string; remember: boolean }>>({})

  const loadStatements = useCallback(() => {
    const request = ++statementsRequest.current
    void api.statementHistory().then((rows) => {
      if (request === statementsRequest.current) setStatements(rows)
    }).catch((e) => { if (request === statementsRequest.current) action.setError(String(e)) })
  }, [])

  useEffect(() => {
    loadStatements()
    return () => { statementsRequest.current++; previewRequest.current++; passwordsRef.current = {} }
  }, [loadStatements])

  const mergeReports = useCallback(
    (incoming: ImportReport[]) => {
      setReports((prev) => {
        const next = [...prev]
        for (const r of incoming) {
          const i = next.findIndex((p) => p.path === r.path)
          if (i >= 0) next[i] = r
          else next.unshift(r)
        }
        return next
      })
      const unknown = incoming
        .filter((r) => r.status === 'unknown_account' && r.iban)
        .map((r) => toSetupTarget(r, passwordsRef.current[r.path]?.password ?? ''))
      if (unknown.length > 0) setUnknownQueue((prev) => upsertQueue(prev, unknown, false))
      for (const report of incoming) {
        if (report.status !== 'unknown_account') delete passwordsRef.current[report.path]
      }
      loadStatements()
    },
    [loadStatements],
  )

  const runImport = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return
      // Kaliber P1 end form: the header line runs while the import runs, the footer keeps a
      // dated receipt. The backend reports no progress, so the line stays indeterminate.
      const done = startActivity('Prebieha import výpisov')
      let receipt: string | undefined
      try {
        const results = await api.importStatements(paths)
        mergeReports(results)
        receipt = `Import dokončený, súbory: ${paths.length}.`
      } finally {
        done(receipt)
      }
    },
    [mergeReports],
  )

  useEffect(() => {
    let unlisten: (() => void) | undefined
    let disposed = false
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === 'drop' && canDrop.current) {
          const paths = event.payload.paths
          void action.run(() => runImport(paths))
        }
      })
      .then((fn) => {
        if (disposed) fn()
        else unlisten = fn
      }).catch((e) => action.setError(String(e)))
    return () => { disposed = true; unlisten?.() }
  }, [runImport])

  usePickRequest(pickRequest, onPickRequestHandled, () => void action.run(pickFiles))

  async function pickFiles() {
    const selection = await open({ multiple: true, filters: [{ name: 'PDF', extensions: ['pdf'] }] })
    if (!selection) return
    const paths = Array.isArray(selection) ? selection : [selection]
    await runImport(paths)
  }

  async function handlePassword(path: string, password: string, remember: boolean) {
    passwordsRef.current[path] = { password, remember }
    const report = await api.importWithPassword(path, password, remember)
    mergeReports([report])
  }

  // Reopens the dialog for a report the user dismissed earlier, at the front
  // of the queue: they asked for it now, so it does not wait behind other
  // pending files.
  function handleAddAccount(report: ImportReport) {
    if (!report.iban) return
    const target = toSetupTarget(report, passwordsRef.current[report.path]?.password ?? '')
    setUnknownQueue((prev) => upsertQueue(prev, [target], true))
  }

  function dequeueUnknown(path: string) {
    setUnknownQueue((prev) => prev.filter((t) => t.path !== path))
  }

  // Keep confirmation disabled until the current preview resolves.
  async function requestDelete(statement: StatementHistoryRow) {
    const request = ++previewRequest.current
    setDeletePreview(null); setPreviewError('')
    setDeleteTarget(statement)
    try {
      const preview = await api.statementDeletePreview(statement.statement_id)
      if (request === previewRequest.current) setDeletePreview(preview)
    } catch (e) { if (request === previewRequest.current) setPreviewError(String(e)) }
  }

  async function confirmDelete() {
    if (!deleteTarget || !deletePreview) return
    const id = deleteTarget.statement_id
    await api.deleteStatement(id)
    statementDeleted.current()
    setDeleteTarget(null)
    setDeletePreview(null)
    setReports((prev) => prev.filter((r) => r.statementId !== id))
    loadStatements()
  }

  function closeDelete() {
    if (deletion.busy) return
    previewRequest.current += 1
    setDeleteTarget(null)
  }

  return (
    <div className="ab-screen">
      <PageHead
        title="Import"
        primary={<PickButton busy={[action.busy, deletion.busy].some(Boolean)} onPick={() => void action.run(pickFiles)} />}
      />
      {action.error ? <p role="alert">{action.error}</p> : null}
      {action.busy ? <p role="status">Prebieha import...</p> : null}
      <fieldset disabled={action.busy || deletion.busy} className="ab-section">
        <div className="k-well ab-dropzone">
          <p>Presuňte výpisy Tatra banky sem alebo použite Vybrať PDF.</p>
        </div>
        {reports.map((report) => (
          <ImportResultCard
            key={report.path}
            report={report}
            onPassword={(password, remember) => void action.run(() => handlePassword(report.path, password, remember))}
            onAddAccount={() => handleAddAccount(report)}
            onContinue={onNavigateToTransactions}
          />
        ))}
        <StatementList statements={statements} onNavigate={(id) => onNavigateToTransactions?.(id)} onDelete={(s) => void requestDelete(s)} deletedRef={statementDeleted} />
      </fieldset>
      <SetupAccountDialog
        key={unknownQueue[0]?.path ?? 'none'}
        target={unknownQueue[0] ?? null}
        onSkip={() => dequeueUnknown(unknownQueue[0]?.path ?? '')}
        onSetup={(report) => { mergeReports([report]); dequeueUnknown(report.path) }}
      />
      <Dialog
        open={deleteTarget !== null}
        title="Natrvalo zmazať výpis?"
        onClose={closeDelete}
        actions={
          <>
            <Button variant="secondary" data-k-autofocus="" disabled={deletion.busy} onClick={closeDelete}>
              Zrušiť
            </Button>
            <Button variant="danger" disabled={!deletePreview || deletion.busy} onClick={() => void deletion.run(confirmDelete)}>
              Natrvalo zmazať
            </Button>
          </>
        }
      >
        {previewError ? <p role="alert">{previewError}</p> : <DeleteStatementBody preview={deletePreview} />}
        {deletion.error ? <p role="alert">{deletion.error}</p> : null}
        {deletion.busy ? <p role="status">Odstraňuje sa...</p> : null}
        <p>Pôvodné bankové PDF súbory sa neodstránia.</p>
      </Dialog>
    </div>
  )
}
