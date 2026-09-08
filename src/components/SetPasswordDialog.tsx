import { useState } from 'react'
import { api, type Account } from '../api'
import { useAction } from '../lib/useAction'
import { Button } from './Button'
import { Dialog } from './Dialog'
import { Field } from './Field'

// Sets or changes the PDF-statement password Abakus stores for one account
// (Michal, 2026-09-08: "Nedá sa nastaviť alebo zmeniť heslo pri účte"). On
// success the dialog stays open and shows the status line instead of
// closing, so the confirmation is visible before the user dismisses it;
// `onSaved` refreshes the account row behind it (`has_password`, "heslo
// uložené") without closing the dialog itself.
//
// `account` is nullable and the component stays mounted (the caller renders
// it unconditionally with a `key` that changes on every open, see
// `Settings.tsx`), so a fresh open always starts from an empty, error-free
// draft even when it targets the same account as the previous open.
export function SetPasswordDialog({ account, onClose, onSaved }: {
  account: Account | null
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const action = useAction()
  const [value, setValue] = useState('')
  const [confirm, setConfirm] = useState('')
  const [error, setError] = useState('')
  const [saved, setSaved] = useState(false)

  // Keeps the typed draft on failure so the user is not asked to retype it;
  // clears it only once the keyring write actually succeeded, so a stored
  // password is never left sitting in the fields.
  async function save() {
    if (!account) return
    setError('')
    setSaved(false)
    try {
      await api.setAccountPassword(account.id, value)
      setValue('')
      setConfirm('')
      setSaved(true)
      await onSaved()
    } catch (e) {
      setError(String(e))
    }
  }

  function close() { if (!action.busy) onClose() }

  return <Dialog
    open={account !== null}
    title={account ? `Heslo k výpisom účtu ${account.label}` : ''}
    onClose={close}
    actions={<>
      <Button variant="secondary" disabled={action.busy} onClick={close}>Zrušiť</Button>
      <Button variant="primary" disabled={action.busy || value === '' || value !== confirm} onClick={() => void action.run(save)}>Uložiť</Button>
    </>}
  >
    <Field label="Heslo">
      <input type="password" className="k-input k-well" value={value} onChange={(e) => setValue(e.target.value)} />
    </Field>
    <Field label="Zopakovať heslo">
      <input type="password" className="k-input k-well" value={confirm} onChange={(e) => setConfirm(e.target.value)} />
    </Field>
    {error ? <p role="alert" className="k-text-danger">{error}</p> : null}
    {saved ? <p role="status">Heslo je uložené v Správcovi poverení.</p> : null}
  </Dialog>
}
