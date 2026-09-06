import { useEffect, useState } from 'react'
import { api, type Account, type AccountDeletePreview } from '../api'
import { useAction } from '../lib/useAction'
import { Button } from './Button'
import { Dialog } from './Dialog'
import { OperationStatus } from './OperationStatus'
import { Field } from './Field'

export function DeleteAccountDialog({ account, onClose, onDeleted }: {
  account: Account
  onClose: () => void
  onDeleted: () => Promise<void>
}) {
  const [preview, setPreview] = useState<AccountDeletePreview | null>(null)
  const [previewError, setPreviewError] = useState('')
  const [attempt, setAttempt] = useState(0)
  const [confirmation, setConfirmation] = useState('')
  const [deleted, setDeleted] = useState(false)
  const action = useAction()
  useEffect(() => {
    let active = true
    setPreview(null); setPreviewError('')
    void api.accountDeletePreview(account.id).then((value) => {
      if (active) setPreview(value)
    }).catch((e) => { if (active) setPreviewError(String(e)) })
    return () => { active = false }
  }, [account.id, attempt])

  async function remove() {
    if (!preview || confirmation !== preview.label) return
    await api.deleteAccount(account.id)
    setDeleted(true)
    await onDeleted()
    onClose()
  }

  async function refreshDeleted() {
    await onDeleted()
    onClose()
  }

  function close() { if (!action.busy) onClose() }

  return <Dialog open title={`Zmazať účet ${account.label}?`} onClose={close} actions={<>
    <Button variant="secondary" disabled={action.busy} onClick={close}>Zrušiť</Button>
    {deleted
      ? <Button onClick={() => void action.run(refreshDeleted)} disabled={action.busy}>Obnoviť údaje</Button>
      : <Button variant="danger" disabled={action.busy || !preview || confirmation !== preview.label} onClick={() => void action.run(remove)}>Natrvalo zmazať účet</Button>}
  </>}>
    {previewError ? <><p role="alert">{previewError}</p><Button onClick={() => setAttempt((v) => v + 1)}>Skúsiť znova</Button></> : null}
    {!preview && !previewError ? <p role="status">Načítava sa náhľad odstránenia...</p> : null}
    {preview ? <>
      <p>Účet: {preview.label}. Výpisy: {preview.statement_count}. Transakcie: {preview.transaction_count}, potvrdené: {preview.confirmed_count}. Odstránené pravidlá: {preview.rules_deleted}.</p>
      <p>Ide o nezvratné lokálne odstránenie účtu a jeho importovaných údajov. Pôvodné bankové PDF súbory sa neodstránia.</p>
      {preview.has_password ? <p>Odstráni sa aj uložené heslo tohto účtu.</p> : null}
      <Field label={`Na potvrdenie napíšte názov účtu: ${preview.label}`}>
        <input className="k-input k-well" value={confirmation} disabled={action.busy || deleted} onChange={(e) => setConfirmation(e.target.value)} />
      </Field>
    </> : null}
    {deleted ? <p role="status">Účet bol odstránený. Obnovte zobrazené údaje.</p> : null}
    <OperationStatus busy={action.busy} error={action.error} />
  </Dialog>
}
