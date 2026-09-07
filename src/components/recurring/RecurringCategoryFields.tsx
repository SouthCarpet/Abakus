import { useRef, useState, type ComponentType } from 'react'
import type { Category } from '../../api'
import { api } from '../../api'
import { Button } from '../Button'
import { CategoryPicker } from '../CategoryPicker'
import type { RecurringCategoryDialogProps } from './category-dialog-props'

export function RecurringCategoryFields({
  categories,
  transactionId,
  CategoryDialog,
  disabled,
}: {
  categories: Category[]
  transactionId: number
  CategoryDialog?: ComponentType<RecurringCategoryDialogProps>
  disabled: boolean
}) {
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [createdId, setCreatedId] = useState<number | null>(null)
  const pending = useRef(false)
  const lastAssigned = useRef<number | null>(null)
  const locked = disabled || busy

  async function assign() {
    if (selectedId === null || pending.current) return
    pending.current = true
    setBusy(true)
    setError('')
    try {
      if (lastAssigned.current !== selectedId) {
        await api.assign([transactionId], selectedId, false)
        lastAssigned.current = selectedId
      }
    } catch (e) {
      setError(String(e))
    } finally {
      pending.current = false
      setBusy(false)
    }
  }

  return (
    <div>
      <p>Zaradenie kategórie je samostatný krok. Potvrdenie pravidelnosti kategóriu nenastaví.</p>
      <CategoryPicker
        value={selectedId}
        onChange={setSelectedId}
        categories={categories}
        disabled={locked}
      />
      {CategoryDialog ? (
        <Button variant="secondary" disabled={locked} onClick={() => setDialogOpen(true)}>
          Nová kategória...
        </Button>
      ) : null}
      <Button variant="secondary" disabled={locked || selectedId === null} onClick={() => void assign()}>
        Zaradiť kategóriu
      </Button>
      {busy ? <p role="status">Zaraďuje sa kategória...</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      {createdId !== null && error ? (
        <p>Kategória ostala vytvorená. Skúste znova zaradenie, nevytvára sa znova.</p>
      ) : null}
      {CategoryDialog ? (
        <CategoryDialog
          open={dialogOpen}
          onClose={() => setDialogOpen(false)}
          onCreated={(category) => {
            setSelectedId(category.id)
            setCreatedId(category.id)
            lastAssigned.current = null
            setDialogOpen(false)
          }}
        />
      ) : null}
    </div>
  )
}
