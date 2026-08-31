import { useCallback, useEffect, useRef, useState } from 'react'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { open } from '@tauri-apps/plugin-dialog'
import type { AccountKind, Checksum, ImportReport, RecentStatement } from '../api'
import { api } from '../api'
import { Button } from '../components/Button'
import { Card } from '../components/Card'
import { Dialog } from '../components/Dialog'
import { Field } from '../components/Field'
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
          <input
            className="k-input k-well"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
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
  onContinue?: () => void
}) {
  const canContinue = onContinue && SUCCESS_STATUSES.has(report.status)
  return (
    <Card
      title={fileName(report.path)}
      badge={importStatusLabel(report.status)}
      footer={
        canContinue ? (
          <Button variant="primary" onClick={onContinue}>
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

function RecentImportsRow({ statement }: { statement: RecentStatement }) {
  const danger = statement.checksum.status === 'off_by'
  return (
    <div className="k-round-row">
      <span>{formatDate(statement.period_end)}</span>
      <span>{statement.account_label}</span>
      <span className="k-num">č. {statement.number}</span>
      <span className="k-num">{statement.transaction_count} transakcií</span>
      <span className={danger ? 'k-card-badge k-text-danger' : 'k-card-badge'}>{checksumLabel(statement.checksum)}</span>
    </div>
  )
}

function RecentImports({ statements }: { statements: RecentStatement[] }) {
  return (
    <Card title="Posledné importy">
      {statements.length === 0 ? (
        <p>Zatiaľ žiadne importy.</p>
      ) : (
        <div className="k-round-list">
          {statements.map((s) => (
            <RecentImportsRow key={s.statement_id} statement={s} />
          ))}
        </div>
      )}
    </Card>
  )
}

export function Import({ onNavigateToTransactions }: { onNavigateToTransactions?: () => void }) {
  const [reports, setReports] = useState<ImportReport[]>([])
  const [recent, setRecent] = useState<RecentStatement[]>([])
  const [addAccount, setAddAccount] = useState<AddAccountTarget | null>(null)
  const [accountLabel, setAccountLabel] = useState('')
  const passwordsRef = useRef<Record<string, { password: string; remember: boolean }>>({})

  const loadRecent = useCallback(() => {
    void api.recentStatements(RECENT_STATEMENTS_LIMIT).then(setRecent)
  }, [])

  useEffect(() => {
    loadRecent()
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
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === 'drop') void runImport(event.payload.paths)
      })
      .then((fn) => {
        unlisten = fn
      })
    return () => unlisten?.()
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
    await api.saveAccount(addAccount.iban, addAccount.kind, accountLabel)
    await retryImport(addAccount.path)
    setAddAccount(null)
  }

  return (
    <div className="k-section">
      <div className="k-well k-dropzone">
        <p>Presuňte výpisy sem alebo</p>
        <Button variant="primary" onClick={() => void pickFiles()}>
          Vybrať PDF
        </Button>
      </div>
      {reports.map((report) => (
        <ImportResultCard
          key={report.path}
          report={report}
          onPassword={(password, remember) => void handlePassword(report.path, password, remember)}
          onAddAccount={(iban, kind) => handleAddAccount(report.path, iban, kind)}
          onContinue={onNavigateToTransactions}
        />
      ))}
      <RecentImports statements={recent} />
      <Dialog
        open={addAccount !== null}
        title="Pridať účet"
        onClose={() => setAddAccount(null)}
        actions={
          <>
            <Button variant="secondary" onClick={() => setAddAccount(null)}>
              Zrušiť
            </Button>
            <Button variant="primary" onClick={() => void saveAccountAndRetry()}>
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
      </Dialog>
    </div>
  )
}
