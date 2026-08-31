import { useEffect, useState } from 'react'
import { save } from '@tauri-apps/plugin-dialog'
import type { Account, AccountKind, NetLogRow, Release } from '../api'
import { api } from '../api'
import { Button } from '../components/Button'
import { Card } from '../components/Card'
import { Dialog } from '../components/Dialog'
import { Field } from '../components/Field'
import { usePeriod } from '../components/PeriodPicker'
import { fromSkParts, maskIban } from '../lib/iban'
import { periodRange } from '../lib/period'

const NET_LOG_LIMIT = 20

function formatLogTime(startedAt: string): string {
  return startedAt.replace('T', ' ').replace('Z', '').slice(0, 19)
}

function NetLogSection({ rows, onScanNow }: { rows: NetLogRow[]; onScanNow: () => void }) {
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

function AccountRow({ account, onForgetPassword }: { account: Account; onForgetPassword: (id: number) => void }) {
  return (
    <tr>
      <td>{account.label}</td>
      <td>{account.kind === 'personal' ? 'Osobný' : 'Firemný'}</td>
      <td>{maskIban(account.iban)}</td>
      <td>{account.has_password ? 'heslo uložené' : 'bez hesla'}</td>
      <td>
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

  async function refresh() {
    setAccounts(await api.listAccounts())
  }

  async function refreshNetLog() {
    setNetLog(await api.netLog(NET_LOG_LIMIT))
  }

  useEffect(() => {
    void refresh()
    void api.dataDir().then(setDataDir)
    void api.recentStatements(ALL_STATEMENTS_LIMIT).then((list) => setStatementCount(list.length))
    void refreshNetLog()
    // Boot gating (spec A14): the check runs only when the persisted flag is
    // on, at the moment Nastavenia is opened. Nastavenia is the one place
    // its result is ever shown.
    void api.getCheckUpdates().then((on) => {
      setCheckUpdates(on)
      if (on) void api.checkUpdateNow().then(setRelease)
    })
  }, [])

  async function toggleCheckUpdates(next: boolean) {
    await api.setCheckUpdates(next)
    setCheckUpdates(next)
    setRelease(next ? await api.checkUpdateNow() : null)
    await refreshNetLog()
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
      await refresh()
    } catch (e) {
      setError(String(e))
    }
  }

  async function forgetPassword(accountId: number) {
    await api.clearPassword(accountId)
    await refresh()
  }

  async function exportCsv() {
    const path = await save({ filters: [{ name: 'CSV', extensions: ['csv'] }] })
    if (!path) return
    const { from, to } = periodRange(period.kind, new Date(), period.custom)
    await api.exportCsv({ from, to }, path)
  }

  return (
    <div className="k-section">
      <div className="k-row" style={{ alignItems: 'stretch' }}>
        <div className="k-section" style={{ flex: 1, minWidth: 320 }}>
          <Card
            title="Účty"
            footer={
              <Button variant="primary" onClick={() => setAddOpen(true)}>
                Pridať účet
              </Button>
            }
          >
            <table className="k-table">
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
                  <AccountRow key={account.id} account={account} onForgetPassword={(id) => void forgetPassword(id)} />
                ))}
              </tbody>
            </table>
          </Card>
          <Card title="Údaje">
            <p>Priečinok s dátami: {dataDir}</p>
            <Button variant="secondary" onClick={() => void exportCsv()}>
              Exportovať CSV
            </Button>
          </Card>
        </div>
        <div style={{ flex: 1, minWidth: 320 }}>
          <Card title="Stav">
            <p>Priečinok s dátami: {dataDir}</p>
            <p>
              Účty: <span className="k-num">{accounts.length}</span>
            </p>
            <p>
              Výpisy: <span className="k-num">{statementCount}</span>
            </p>
            <label className="k-checkbox">
              <input type="checkbox" checked={checkUpdates} onChange={(e) => void toggleCheckUpdates(e.target.checked)} />
              Kontrolovať aktualizácie (GitHub)
            </label>
            <p>Toto je jediné sieťové volanie aplikácie. V predvolenom stave je vypnuté.</p>
            {release ? (
              <>
                <p>Dostupná aktualizácia {release.tag}</p>
                <p>{release.url}</p>
              </>
            ) : null}
            <NetLogSection rows={netLog} onScanNow={() => void scanNow()} />
          </Card>
        </div>
      </div>
      <Dialog
        open={addOpen}
        title="Pridať účet"
        onClose={() => setAddOpen(false)}
        actions={
          <>
            <Button variant="secondary" onClick={() => setAddOpen(false)}>
              Zrušiť
            </Button>
            <Button variant="primary" onClick={() => void saveAccount()}>
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
        {error ? <p className="k-text-danger">{error}</p> : null}
      </Dialog>
    </div>
  )
}
