import { useState } from 'react'
import { api } from '../api'
import type { BackupPreview } from '../api'
import { files } from '../lib/files'
import { useAction } from '../lib/useAction'
import { Button } from './Button'
import { Card } from './Card'
import { Dialog } from './Dialog'
import { OperationStatus } from './OperationStatus'

// Shown both before any pick (so the limit is known up front) and again
// inside the confirmation dialog, right before the irreversible click.
const CREDENTIALS_NOTE = 'Heslá k účtom zo Správcu poverení systému Windows nie sú súčasťou zálohy a obnovou sa nezmenia.'

// Same rule as CREDENTIALS_NOTE: the confirmation dialog itself must say
// this, not just the card underneath it. A user confirming from the dialog
// alone must already know a safety copy exists and where appka puts it.
const SAFETY_COPY_NOTE = 'Pred obnovou appka najprv vytvorí bezpečnostnú kópiu súčasnej databázy v priečinku s dátami appky a cestu k nej ukáže.'

// 091/B10 p3 fix: `onRestored` fires only after `restoreDatabase` itself
// resolves, never after a cancelled pick or a failed restore, so a caller
// (Settings, and through it App's cross-screen generation bump) only ever
// reacts to a restore that actually landed.
export function RestoreSection({ onRestored }: { onRestored?: () => void } = {}) {
  const pickAction = useAction()
  const confirmAction = useAction()
  const [pickedPath, setPickedPath] = useState('')
  const [preview, setPreview] = useState<BackupPreview | null>(null)
  const [safetyCopyPath, setSafetyCopyPath] = useState('')

  async function pickAndPreview() {
    setSafetyCopyPath('')
    const path = await files.openBackup()
    if (!path) return
    // A corrupt or foreign file is refused here, in the pick step: the
    // confirmation dialog below only ever opens once the backend has
    // already proven the file is a valid, supported Abakus database.
    const result = await api.restorePreview(path)
    setPickedPath(path)
    setPreview(result)
  }

  async function confirmRestore() {
    const result = await api.restoreDatabase(pickedPath)
    setPreview(null)
    setPickedPath('')
    setSafetyCopyPath(result.safety_copy_path)
    onRestored?.()
  }

  function cancel() {
    setPreview(null)
    setPickedPath('')
  }

  const busy = pickAction.busy || confirmAction.busy

  return (
    <Card
      title="Obnova zo zálohy"
      footer={
        <Button variant="secondary" disabled={busy} onClick={() => void pickAction.run(pickAndPreview)}>
          Vybrať zálohu
        </Button>
      }
    >
      <p>Obnova nahradí všetky súčasné dáta obsahom vybranej zálohy. Pred obnovou appka vždy najprv vytvorí bezpečnostnú kópiu súčasnej databázy.</p>
      <p>{CREDENTIALS_NOTE}</p>
      <OperationStatus error={pickAction.error} busy={pickAction.busy}>Načítava sa náhľad zálohy...</OperationStatus>
      {safetyCopyPath ? (
        <p role="status">
          Obnova prebehla. Skontrolujte dáta. Bezpečnostná kópia databázy spred obnovy je uložená tu: {safetyCopyPath}
        </p>
      ) : null}
      <Dialog
        open={preview !== null}
        title="Náhľad zálohy"
        onClose={() => { if (!busy) cancel() }}
        actions={
          <>
            <Button variant="secondary" disabled={busy} onClick={cancel}>
              Zrušiť
            </Button>
            <Button variant="primary" disabled={busy} onClick={() => void confirmAction.run(confirmRestore)}>
              Obnoviť databázu
            </Button>
          </>
        }
      >
        {preview ? (
          <>
            <p>Účty: {preview.accounts}</p>
            <p>Výpisy: {preview.statements}</p>
            <p>Transakcie: {preview.transactions}</p>
            <p>{SAFETY_COPY_NOTE}</p>
            <p>{CREDENTIALS_NOTE}</p>
          </>
        ) : null}
        <OperationStatus error={confirmAction.error} busy={confirmAction.busy}>Obnovuje sa...</OperationStatus>
      </Dialog>
    </Card>
  )
}
