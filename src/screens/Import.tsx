import { useCallback, useEffect, useRef, useState } from 'react'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-dialog'
import type { AccountKind, Checksum, ImportReport, RecentStatement, StatementDeletePreview } from '../api'
import { api } from '../api'
import { useAction } from '../lib/useAction'
import { Button } from '../components/Button'
import { Card } from '../components/Card'
import { Dialog } from '../components/Dialog'
import { Field } from '../components/Field'
import { PasswordInput } from '../components/PasswordInput'
import { checksumLabel, formatDate, importStatusLabel } from '../lib/format'

const RECENT_STATEMENTS_LIMIT = 8
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
  return <span className={danger ? 'k-text-danger' : undefined}>{checksumLabel(checksum)}</span>
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
  return <p className="k-text-danger">{report.message}</p>
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
        <label className="k-checkbox">
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
      <div className="k-row">
        <span>{report.accountLabel ?? report.ibanMasked ?? '-'}</span>
        {report.statementNumber !== null ? <span className="k-num">č. {report.statementNumber}</span> : null}
        <PeriodLine report={report} />
      </div>
      <div className="k-row">
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

interface AddAccountTarget {
  path: string
  iban: string
  kind: AccountKind
}

function RecentImportsRow({
  statement,
  onNavigate,
  onDelete,
}: {
  statement: RecentStatement
  onNavigate: (statementId: number) => void
  onDelete: (statement: RecentStatement) => void
}) {
  const danger = statement.checksum.status === 'off_by'
  return (
    <div className="k-round-row">
      <span>{formatDate(statement.period_end)}</span>
      <span>{statement.account_label}</span>
      <span className="k-num">č. {statement.number}</span>
      <span className="k-num">{statement.transaction_count} transakcií</span>
      <span className={danger ? 'k-card-badge k-text-danger' : 'k-card-badge'}>{checksumLabel(statement.checksum)}</span>
      <Button variant="ghost" onClick={() => onNavigate(statement.statement_id)}>
        Zobraziť transakcie
      </Button>
      {/* Secondary, not the loudest control on the row: composes the
          existing ghost variant with the existing danger text tone, so
          Zmazať stays findable without outweighing Zobraziť transakcie. */}
      <Button variant="ghost" className="k-text-danger" onClick={() => onDelete(statement)}>
        Zmazať
      </Button>
    </div>
  )
}

function RecentImportsHead() {
  return (
    <div className="k-round-row-head">
      <span>Dátum</span>
      <span>Účet</span>
      <span>Číslo</span>
      <span>Transakcie</span>
      <span>Kontrolný súčet</span>
      <span>Akcie</span>
    </div>
  )
}

function RecentImports({
  statements,
  onNavigate,
  onDelete,
}: {
  statements: RecentStatement[]
  onNavigate: (statementId: number) => void
  onDelete: (statement: RecentStatement) => void
}) {
  return (
    <Card title="Posledné importy">
      {statements.length === 0 ? (
        <p>Zatiaľ žiadne importy.</p>
      ) : (
        <div className="k-round-list">
          <RecentImportsHead />
          {statements.map((s) => (
            <RecentImportsRow key={s.statement_id} statement={s} onNavigate={onNavigate} onDelete={onDelete} />
          ))}
        </div>
      )}
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

export function Import({ onNavigateToTransactions }: { onNavigateToTransactions?: (statementId?: number) => void }) {
  const action = useAction()
  const deletion = useAction()
  const previewRequest = useRef(0)
  const recentRequest = useRef(0)
  const [previewError, setPreviewError] = useState('')
  const [reports, setReports] = useState<ImportReport[]>([])
  const [recent, setRecent] = useState<RecentStatement[]>([])
  const [addAccount, setAddAccount] = useState<AddAccountTarget | null>(null)
  const [accountLabel, setAccountLabel] = useState('')
  const [addAccountError, setAddAccountError] = useState('')
  const [deleteTarget, setDeleteTarget] = useState<RecentStatement | null>(null)
  const [deletePreview, setDeletePreview] = useState<StatementDeletePreview | null>(null)
  const canDrop = useRef(true)
  canDrop.current = deleteTarget === null && addAccount === null
  const passwordsRef = useRef<Record<string, { password: string; remember: boolean }>>({})

  const loadRecent = useCallback(() => {
    const request = ++recentRequest.current
    void api.recentStatements(RECENT_STATEMENTS_LIMIT).then((rows) => {
      if (request === recentRequest.current) setRecent(rows)
    }).catch((e) => { if (request === recentRequest.current) action.setError(String(e)) })
  }, [])

  useEffect(() => {
    loadRecent()
    return () => { recentRequest.current++; previewRequest.current++; passwordsRef.current = {} }
  }, [loadRecent])

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
      for (const report of incoming) {
        if (report.status !== 'unknown_account') delete passwordsRef.current[report.path]
      }
      loadRecent()
    },
    [loadRecent],
  )

  const runImport = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return
      const results = await api.importStatements(paths)
      mergeReports(results)
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

  function handleAddAccount(path: string, iban: string, kind: AccountKind) {
    setAccountLabel('')
    setAddAccountError('')
    setAddAccount({ path, iban, kind })
  }

  async function retryImport(path: string) {
    const stored = passwordsRef.current[path]
    const report = stored
      ? await api.importWithPassword(path, stored.password, stored.remember)
      : (await api.importStatements([path]))[0]
    if (report) mergeReports([report])
  }

  async function saveAccountAndRetry() {
    if (!addAccount) return
    setAddAccountError('')
    try {
      await api.saveAccount(addAccount.iban, addAccount.kind, accountLabel)
      await retryImport(addAccount.path)
      setAddAccount(null)
    } catch (e) {
      setAddAccountError(String(e))
    }
  }

  // Keep confirmation disabled until the current preview resolves.
  async function requestDelete(statement: RecentStatement) {
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
    setDeleteTarget(null)
    setDeletePreview(null)
    setReports((prev) => prev.filter((r) => r.statementId !== id))
    loadRecent()
  }

  function closeDelete() {
    if (deletion.busy) return
    previewRequest.current += 1
    setDeleteTarget(null)
  }

  return (
    <div className="k-section">
      {action.error ? <p role="alert">{action.error}</p> : null}
      {action.busy ? <p role="status">Prebieha import...</p> : null}
      <fieldset disabled={action.busy || deletion.busy} className="k-section">
        <div className="k-well k-dropzone">
          <p>Presuňte výpisy sem alebo</p>
          <Button variant="primary" onClick={() => void action.run(pickFiles)}>
            Vybrať PDF
          </Button>
        </div>
        {reports.map((report) => (
          <ImportResultCard
            key={report.path}
            report={report}
            onPassword={(password, remember) => void action.run(() => handlePassword(report.path, password, remember))}
            onAddAccount={(iban, kind) => handleAddAccount(report.path, iban, kind)}
            onContinue={onNavigateToTransactions}
          />
        ))}
        <RecentImports statements={recent} onNavigate={(id) => onNavigateToTransactions?.(id)} onDelete={(s) => void requestDelete(s)} />
      </fieldset>
      <Dialog
        open={addAccount !== null}
        title="Pridať účet"
        onClose={() => { if (!action.busy) setAddAccount(null) }}
        actions={
          <>
            <Button variant="secondary" disabled={action.busy} onClick={() => setAddAccount(null)}>
              Zrušiť
            </Button>
            <Button variant="primary" disabled={action.busy} onClick={() => void action.run(saveAccountAndRetry)}>
              Uložiť
            </Button>
          </>
        }
      >
        <Field label="Názov účtu">
          <input className="k-input k-well" value={accountLabel} onChange={(e) => setAccountLabel(e.target.value)} />
        </Field>
        <Field label="Druh účtu">
          <select
            className="k-select k-well"
            value={addAccount?.kind ?? 'personal'}
            onChange={(e) =>
              setAddAccount((prev) => (prev ? { ...prev, kind: e.target.value as AccountKind } : prev))
            }
          >
            <option value="personal">Osobný</option>
            <option value="business">Firemný</option>
          </select>
        </Field>
        {addAccountError ? <p role="alert" className="k-text-danger">{addAccountError}</p> : null}
      </Dialog>
      <Dialog
        open={deleteTarget !== null}
        title="Natrvalo zmazať výpis?"
        onClose={closeDelete}
        actions={
          <>
            <Button variant="secondary" disabled={deletion.busy} onClick={closeDelete}>
              Späť
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
