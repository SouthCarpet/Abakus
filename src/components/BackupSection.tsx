import { useState } from 'react'
import { api } from '../api'
import { files } from '../lib/files'
import { useAction } from '../lib/useAction'
import { Button } from './Button'
import { Card } from './Card'
import { OperationStatus } from './OperationStatus'

export function BackupSection() {
  const action = useAction()
  const [savedPath, setSavedPath] = useState('')

  async function backup() {
    setSavedPath('')
    const path = await files.saveBackup()
    if (!path) return
    const outcome = await api.backupDatabase(path)
    setSavedPath(outcome.path)
  }

  return (
    <Card
      title="Záloha databázy"
      footer={
        <Button variant="secondary" disabled={action.busy} onClick={() => void action.run(backup)}>
          Zálohovať databázu
        </Button>
      }
    >
      <p>Záloha obsahuje nezašifrované bankové údaje. Heslá k účtom a pôvodné PDF výpisy sa do zálohy neukladajú. Pri každej zálohe zvoľte nový názov súboru.</p>
      <OperationStatus error={action.error} busy={action.busy}>Zálohuje sa...</OperationStatus>
      {savedPath ? <p role="status">Záloha uložená: {savedPath}</p> : null}
    </Card>
  )
}
