import { useEffect, useRef, useState, type ComponentType } from 'react'
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
  onChanged,
}: {
  categories: Category[]
  transactionId: number
  CategoryDialog?: ComponentType<RecurringCategoryDialogProps>
  disabled: boolean
  onChanged: () => Promise<void>
}) {
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [error, setError] = useState('')
  const [refreshError, setRefreshError] = useState('')
  const [busy, setBusy] = useState(false)
  const [createdCategory, setCreatedCategory] = useState<Category | null>(null)
  const pending = useRef(false)
  const lastAssigned = useRef<number | null>(null)
  const mounted = useRef(true)
  const locked = disabled || busy
  const availableCategories = createdCategory && !categories.some((category) => category.id === createdCategory.id)
    ? [...categories, createdCategory]
    : categories

  useEffect(() => () => { mounted.current = false }, [])

  async function assign() {
    if (selectedId === null || pending.current) return
    const targetTransactionId = transactionId
    pending.current = true
    setBusy(true)
    setError('')
    setRefreshError('')
    try {
      if (lastAssigned.current !== selectedId) {
        await api.assign([targetTransactionId], selectedId, false)
        if (!mounted.current) return
        lastAssigned.current = selectedId
      }
      await onChanged()
    } catch (e) {
      if (!mounted.current) return
      if (lastAssigned.current === selectedId) setRefreshError(String(e))
      else setError(String(e))
    } finally {
      if (mounted.current) {
        pending.current = false
        setBusy(false)
      }
    }
  }

  return (
    <div>
      <p>Zaradenie kategórie je samostatný krok. Potvrdenie pravidelnosti kategóriu nenastaví.</p>
      <CategoryPicker
        value={selectedId}
        onChange={(id) => {
          setSelectedId(id)
          setError('')
          setRefreshError('')
        }}
        categories={availableCategories}
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
      {refreshError ? <p role="alert">Kategória je zaradená, obnovenie zlyhalo: {refreshError}</p> : null}
      {createdCategory !== null && error ? (
        <p>Kategória ostala vytvorená. Skúste znova zaradenie, nevytvára sa znova.</p>
      ) : null}
      {CategoryDialog ? (
        <CategoryDialog
          open={dialogOpen}
          onClose={() => setDialogOpen(false)}
          onCreated={(category) => {
            setSelectedId(category.id)
            setCreatedCategory(category)
            lastAssigned.current = null
            setError('')
            setRefreshError('')
            setDialogOpen(false)
          }}
        />
      ) : null}
    </div>
  )
}
