import { useState } from 'react'
import { api, type Account, type AccountKind, type ImportReport } from '../api'
import { useAction } from '../lib/useAction'
import { Button } from './Button'
import { Dialog } from './Dialog'
import { Field } from './Field'
import { PasswordInput } from './PasswordInput'

// The rest of a statement's unknown-account result the dialog needs.
// `password` is the draft to prefill: the one typed for this same file when
// it was locked, or '' when the file needed no password.
export interface SetupAccountTarget {
  path: string
  iban: string
  ibanMasked: string | null
  kind: AccountKind
  password: string
}

function defaultLabel(kind: AccountKind): string {
  return kind === 'business' ? 'Firemný účet' : 'Osobný účet'
}

// Michal, 2026-09-09: "IBAN si už načíta sám po vložení hesla" - so an
// unknown account can be set up right where the statement lands, not only
// through Nastavenia. Three backend calls run in turn (save the account,
// then its password if one was typed, then the retry); any failure stops
// here and surfaces in the dialog, so the account is never left half-set-up
// without the user seeing why.
async function setupAccount(target: SetupAccountTarget, label: string, kind: AccountKind, password: string): Promise<ImportReport> {
  const account: Account = await api.saveAccount(target.iban, kind, label)
  if (password !== '') await api.setAccountPassword(account.id, password)
  const report = password !== ''
    ? await api.importWithPassword(target.path, password, false)
    : (await api.importStatements([target.path]))[0]
  return report
}

export function SetupAccountDialog({
  target,
  onSetup,
  onSkip,
}: {
  target: SetupAccountTarget | null
  onSetup: (report: ImportReport) => void
  onSkip: () => void
}) {
  const action = useAction()
  const [label, setLabel] = useState(() => defaultLabel(target?.kind ?? 'personal'))
  const [kind, setKind] = useState<AccountKind>(target?.kind ?? 'personal')
  const [password, setPassword] = useState(target?.password ?? '')

  async function submit() {
    if (!target) return
    const report = await setupAccount(target, label, kind, password)
    onSetup(report)
  }

  function skip() {
    if (!action.busy) onSkip()
  }

  return (
    <Dialog
      open={target !== null}
      title="Účet nie je nastavený"
      onClose={skip}
      actions={
        <>
          <Button variant="secondary" disabled={action.busy} onClick={skip}>
            Neskôr
          </Button>
          <Button variant="primary" disabled={action.busy} onClick={() => void action.run(submit)}>
            Nastaviť účet
          </Button>
        </>
      }
    >
      <p>
        Výpis patrí účtu {target?.ibanMasked ?? ''}, ktorý v aplikácii ešte nie je. Chcete ho nastaviť teraz?
      </p>
      <p>{target?.iban ?? ''}</p>
      <Field label="Názov účtu">
        <input className="k-input k-well" value={label} onChange={(e) => setLabel(e.target.value)} />
      </Field>
      <Field label="Druh účtu">
        <select className="k-select k-well" value={kind} onChange={(e) => setKind(e.target.value as AccountKind)}>
          <option value="personal">Osobný</option>
          <option value="business">Firemný</option>
        </select>
      </Field>
      <Field label="Heslo k výpisom">
        <PasswordInput value={password} onChange={setPassword} />
      </Field>
      <p>Heslo sa uloží do Správcu poverení systému Windows. Nechajte prázdne, ak výpisy nemajú heslo.</p>
      {action.error ? <p role="alert" className="k-text-danger">{action.error}</p> : null}
    </Dialog>
  )
}
