import { useEffect, useRef, useState } from 'react'
import { files } from '../lib/files'
import { useAction } from '../lib/useAction'
import { BackupSection } from '../components/BackupSection'
import { DeleteAccountDialog } from '../components/DeleteAccountDialog'
import type { Account, AccountKind, AuditFailure, NetLogRow, Release } from '../api'
import { api } from '../api'
import { Button } from '../components/Button'
import { Card } from '../components/Card'
import { Dialog } from '../components/Dialog'
import { Field } from '../components/Field'
import { usePeriod } from '../components/PeriodPicker'
import { fromSkParts, maskIban } from '../lib/iban'
import { periodRange, validPeriod } from '../lib/period'

const NET_LOG_LIMIT = 20

function formatLogTime(startedAt: string): string {
  return startedAt.replace('T', ' ').replace('Z', '').slice(0, 19)
}

// A17/F7: a write into net_log can itself fail (poisoned store, disk error).
// The request still happened, so a silent loss would leave Nastavenia
// looking complete while the audit trail has a gap. This shows exactly that.
function AuditFailuresSection({ failures }: { failures: AuditFailure[] }) {
  if (failures.length === 0) return null
  return (
    <div className="k-section">
      <p className="k-card-title k-text-danger">Neúspešné zápisy auditu</p>
      <table className="k-table">
        <thead>
          <tr>
            <th>Čas</th>
            <th>Adresa</th>
            <th>Chyba</th>
          </tr>
        </thead>
        <tbody>
          {failures.map((f, i) => (
            <tr key={`${f.at}-${i}`}>
              <td>{formatLogTime(f.at)}</td>
              <td>{f.url}</td>
              <td>{f.error}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <p>Tieto zlyhania sú dostupné len počas behu aplikácie. Po reštarte sa odstránia.</p>
    </div>
  )
}

function NetLogSection({ rows, failures, onScanNow }: { rows: NetLogRow[]; failures: AuditFailure[]; onScanNow: () => void }) {
  return (
    <div className="k-section">
      <p className="k-card-title">Sieťová aktivita</p>
      {rows.length === 0 ? (
        <p>Žiadna sieťová aktivita</p>
      ) : (
        <table className="k-table">
          <thead>
            <tr>
              <th>Čas</th>
              <th>Adresa</th>
              <th>Stav</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.id}>
                <td>{formatLogTime(row.started_at)}</td>
                <td>{row.url}</td>
                <td>{row.status}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      <p>Vzorkovanie nemusí zachytiť krátke pripojenia. Skutočnú záruku dávajú CSP politika a test závislostí.</p>
      <AuditFailuresSection failures={failures} />
      <Button variant="secondary" onClick={onScanNow}>
        Skenovať teraz
      </Button>
    </div>
  )
}

// Large enough that every statement a single-user local install could hold
// comes back in one page, so its length also serves as the total count
// (spec: no dedicated counting command).
const ALL_STATEMENTS_LIMIT = 100000

const BANK_REF = /^\d{4}\/\d{1,6}-\d{1,10}$/

function resolveIban(input: string): string {
  const value = input.trim()
  if (!BANK_REF.test(value)) return value
  const [bank, rest] = value.split('/')
  const [prefix, number] = rest.split('-')
  return fromSkParts(bank, prefix, number)
}

function AccountRow({
  account,
  onForgetPassword,
  onEdit,
  onDelete,
}: {
  account: Account
  onForgetPassword: (id: number) => void
  onEdit: (account: Account) => void
  onDelete: (account: Account) => void
}) {
  return (
    <tr>
      <td>{account.label}</td>
      <td>{account.kind === 'personal' ? 'Osobný' : 'Firemný'}</td>
      <td>{maskIban(account.iban)}</td>
      <td>{account.has_password ? 'heslo uložené' : 'bez hesla'}</td>
      <td>
        <Button variant="ghost" onClick={() => onEdit(account)}>
          Upraviť
        </Button>
        <Button variant="ghost" onClick={() => onDelete(account)}>Zmazať účet</Button>
        {account.has_password ? (
          <Button variant="ghost" onClick={() => onForgetPassword(account.id)}>
            Zabudnúť heslo
          </Button>
        ) : null}
      </td>
    </tr>
  )
}

export function Settings() {
  const action = useAction()
  const accountRequest = useRef(0)
  const netRequest = useRef(0)
  const updateRequest = useRef(0)
  const [deleteTarget, setDeleteTarget] = useState<Account | null>(null)
  const [exportMessage, setExportMessage] = useState('')
  const [accounts, setAccounts] = useState<Account[]>([])
  const [dataDir, setDataDir] = useState('')
  const [statementCount, setStatementCount] = useState(0)
  const [addOpen, setAddOpen] = useState(false)
  const [ibanInput, setIbanInput] = useState('')
  const [label, setLabel] = useState('')
  const [kind, setKind] = useState<AccountKind>('personal')
  const [error, setError] = useState('')
  const [period] = usePeriod()
  const [checkUpdates, setCheckUpdates] = useState(false)
  const [release, setRelease] = useState<Release | null>(null)
  const [netLog, setNetLog] = useState<NetLogRow[]>([])
  const [auditFailures, setAuditFailures] = useState<AuditFailure[]>([])
  const [editTarget, setEditTarget] = useState<Account | null>(null)
  const [editLabel, setEditLabel] = useState('')
  const [editKind, setEditKind] = useState<AccountKind>('personal')
  const [editAcknowledge, setEditAcknowledge] = useState(false)
  const [editError, setEditError] = useState('')

  async function refresh() {
    const request = ++accountRequest.current
    const [nextAccounts, statements] = await Promise.all([api.listAccounts(), api.recentStatements(ALL_STATEMENTS_LIMIT)])
    if (request !== accountRequest.current) return
    setAccounts(nextAccounts)
    setStatementCount(statements.length)
  }

  async function refreshNetLog() {
    const request = ++netRequest.current
    const [rows, failures] = await Promise.all([api.netLog(NET_LOG_LIMIT), api.netAuditFailures()])
    if (request !== netRequest.current) return
    setNetLog(rows)
    setAuditFailures(failures)
  }

  useEffect(() => {
    void refresh().catch((e) => action.setError(String(e)))
    void api.dataDir().then(setDataDir).catch((e) => action.setError(String(e)))
    void refreshNetLog().catch((e) => action.setError(String(e)))
    const request = ++updateRequest.current
    void loadUpdatePreference(request).catch((e) => action.setError(String(e)))
    return () => { accountRequest.current++; netRequest.current++; updateRequest.current++ }
  }, [])

  async function loadUpdatePreference(request: number) {
    const on = await api.getCheckUpdates()
    if (request !== updateRequest.current) return
    setCheckUpdates(on)
    if (!on) return
    const nextRelease = await api.checkUpdateNow()
    if (request === updateRequest.current) setRelease(nextRelease)
    await refreshNetLog()
  }

  async function toggleCheckUpdates(next: boolean) {
    const request = ++updateRequest.current
    await api.setCheckUpdates(next)
    setCheckUpdates(next)
    setRelease(null)
    try {
      const nextRelease = next ? await api.checkUpdateNow() : null
      if (request === updateRequest.current) setRelease(nextRelease)
    } finally { await refreshNetLog() }
  }

  async function scanNow() {
    await api.runNetAudit()
    await refreshNetLog()
  }

  async function saveAccount() {
    setError('')
    try {
      await api.saveAccount(resolveIban(ibanInput), kind, label)
      setAddOpen(false)
      setIbanInput('')
      setLabel('')
      await refresh().catch((e) => action.setError(String(e)))
    } catch (e) {
      setError(String(e))
    }
  }

  async function forgetPassword(accountId: number) {
    await api.clearPassword(accountId)
    await refresh()
  }

  function openEdit(account: Account) {
    setEditTarget(account)
    setEditLabel(account.label)
    setEditKind(account.kind)
    setEditAcknowledge(false)
    setEditError('')
  }

  function changeEditKind(next: AccountKind) {
    setEditKind(next)
    setEditAcknowledge(false)
    setEditError('')
  }

  // A17/F2: the IBAN never travels here, so an edit can only touch the label
  // and the kind of the account it was opened for. The backend refuses a kind
  // change on an account with imports unless acknowledged; the checkbox below
  // only appears once that refusal names what would be recast.
  async function saveEdit() {
    if (!editTarget) return
    setEditError('')
    try {
      await api.updateAccount(editTarget.id, editLabel, editKind, editAcknowledge)
      setEditTarget(null)
      await refresh()
    } catch (e) {
      setEditError(String(e))
    }
  }

  async function exportCsv() {
    if (!validPeriod(period)) throw new Error('Vyberte platné obdobie v Transakciách.')
    setExportMessage('')
    const path = await files.saveCsv()
    if (!path) { setExportMessage('Export zrušený.'); return }
    const { from, to } = periodRange(period.kind, new Date(), period.custom)
    const count = await api.exportCsv({ from, to }, path)
    setExportMessage(`Exportovaných transakcií: ${count}.`)
  }

  return (
    <div className="k-section">
      {action.error ? <p role="alert">{action.error}</p> : null}
      {action.busy ? <p role="status">Prebieha operácia...</p> : null}
      {exportMessage ? <p role="status">{exportMessage}</p> : null}
      <fieldset disabled={action.busy} className="k-section">
        <div className="k-settings-layout">
          <div className="k-section">
            <Card
              title="Účty"
              footer={
                <Button variant="primary" onClick={() => setAddOpen(true)}>
                  Pridať účet
                </Button>
              }
            >
              <div className="k-table-scroll" role="region" aria-label="Účty" tabIndex={0}>
              <table className="k-table k-accounts-table">
                <thead>
                  <tr>
                    <th>Názov</th>
                    <th>Druh</th>
                    <th>IBAN</th>
                    <th>Heslo</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {accounts.map((account) => (
                    <AccountRow key={account.id} account={account} onForgetPassword={(id) => void action.run(() => forgetPassword(id))} onEdit={openEdit} onDelete={setDeleteTarget} />
                  ))}
                </tbody>
              </table>
              </div>
            </Card>
            <Card title="Údaje">
              <p>Priečinok s dátami: {dataDir}</p>
              <Button variant="secondary" onClick={() => void action.run(() => exportCsv())}>
                Exportovať CSV
              </Button>
            </Card>
            <BackupSection />
          </div>
          <div>
            <Card title="Stav">
              <p>Priečinok s dátami: {dataDir}</p>
              <p>
                Účty: <span className="k-num">{accounts.length}</span>
              </p>
              <p>
                Výpisy: <span className="k-num">{statementCount}</span>
              </p>
              <label className="k-checkbox">
                <input type="checkbox" checked={checkUpdates} onChange={(e) => void action.run(() => toggleCheckUpdates(e.target.checked))} />
                Kontrolovať aktualizácie (GitHub)
              </label>
              <p>Toto je jediné sieťové volanie aplikácie. V predvolenom stave je vypnuté.</p>
              {release ? (
                <>
                  <p>Dostupná aktualizácia {release.tag}</p>
                  <p>{release.url}</p>
                </>
              ) : null}
              <NetLogSection rows={netLog} failures={auditFailures} onScanNow={() => void action.run(() => scanNow())} />
            </Card>
          </div>
        </div>
      </fieldset>
      {deleteTarget ? <DeleteAccountDialog account={deleteTarget} onClose={() => setDeleteTarget(null)} onDeleted={refresh} /> : null}
      <Dialog
        open={addOpen}
        title="Pridať účet"
        onClose={() => { if (!action.busy) setAddOpen(false) }}
        actions={
          <>
            <Button variant="secondary" disabled={action.busy} onClick={() => setAddOpen(false)}>
              Zrušiť
            </Button>
            <Button variant="primary" disabled={action.busy} onClick={() => void action.run(() => saveAccount())}>
              Uložiť
            </Button>
          </>
        }
      >
        <Field label="IBAN alebo kód banky/číslo účtu">
          <input className="k-input k-well" value={ibanInput} onChange={(e) => setIbanInput(e.target.value)} />
        </Field>
        <Field label="Názov účtu">
          <input className="k-input k-well" value={label} onChange={(e) => setLabel(e.target.value)} />
        </Field>
        <Field label="Druh účtu">
          <select className="k-select k-well" value={kind} onChange={(e) => setKind(e.target.value as AccountKind)}>
            <option value="personal">Osobný</option>
            <option value="business">Firemný</option>
          </select>
        </Field>
        {error ? <p role="alert" className="k-text-danger">{error}</p> : null}
      </Dialog>
      <Dialog
        open={editTarget !== null}
        title="Upraviť účet"
        onClose={() => { if (!action.busy) setEditTarget(null) }}
        actions={
          <>
            <Button variant="secondary" disabled={action.busy} onClick={() => setEditTarget(null)}>
              Zrušiť
            </Button>
            <Button variant="primary" disabled={action.busy} onClick={() => void action.run(() => saveEdit())}>
              Uložiť
            </Button>
          </>
        }
      >
        <Field label="Názov účtu">
          <input className="k-input k-well" value={editLabel} onChange={(e) => setEditLabel(e.target.value)} />
        </Field>
        <Field label="IBAN (nedá sa zmeniť)">
          <input className="k-input k-well" value={editTarget ? maskIban(editTarget.iban) : ''} disabled readOnly />
        </Field>
        <Field label="Typ účtu">
          <select className="k-select k-well" value={editKind} onChange={(e) => changeEditKind(e.target.value as AccountKind)}>
            <option value="personal">Osobný</option>
            <option value="business">Firemný</option>
          </select>
        </Field>
        {editError ? <p role="alert" className="k-text-danger">{editError}</p> : null}
        {editError && editTarget && editKind !== editTarget.kind ? (
          <label className="k-checkbox">
            <input type="checkbox" checked={editAcknowledge} onChange={(e) => setEditAcknowledge(e.target.checked)} />
            Potvrdzujem zmenu typu účtu a nové zaradenie transakcií.
          </label>
        ) : null}
      </Dialog>
    </div>
  )
}
