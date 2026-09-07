import { useEffect, useState } from 'react'
import type { Category, CategoryKind } from '../api'
import { api } from '../api'
import { Button } from './Button'
import { Dialog } from './Dialog'
import { Field } from './Field'
import { groupCategories } from '../lib/categories'

const CATEGORY_KIND_LABELS: Record<CategoryKind, string> = { expense: 'Výdavok', income: 'Príjem' }

// Section 9 shared seam: creation only. Owns its own category list and
// creates through the existing, now-validated `api.saveCategory`; the
// reparent/kind-change flow lives in `categoryApi` and Categories.tsx.
export function CategoryDialog({
  open,
  initialParentId = null,
  initialKind = 'expense',
  onClose,
  onCreated,
}: {
  open: boolean
  initialParentId?: number | null
  initialKind?: CategoryKind
  onClose: () => void
  onCreated: (category: Category) => void
}) {
  const [categories, setCategories] = useState<Category[]>([])
  const [name, setName] = useState('')
  const [parentId, setParentId] = useState<number | null>(initialParentId)
  const [kind, setKind] = useState<CategoryKind>(initialKind)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  // Draft (name/parent/kind) resets only when the dialog reopens, so a
  // failed create leaves the user's typed values in place for a retry.
  useEffect(() => {
    if (!open) return
    setName('')
    setParentId(initialParentId)
    setKind(initialKind)
    setError('')
    void api.listCategories().then(setCategories).catch((e) => setError(String(e)))
  }, [open, initialParentId, initialKind])

  const parents = groupCategories(categories).map((g) => g.parent)
  const selectedParent = parents.find((p) => p.id === parentId) ?? null
  const effectiveKind = selectedParent ? selectedParent.kind : kind

  async function create() {
    setBusy(true)
    setError('')
    try {
      const category = await api.saveCategory(null, parentId, name, effectiveKind)
      onCreated(category)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog
      open={open}
      title="Nová kategória"
      onClose={() => {
        if (!busy) onClose()
      }}
      actions={
        <>
          <Button variant="secondary" disabled={busy} onClick={onClose}>
            Zrušiť
          </Button>
          <Button variant="primary" disabled={busy || !name.trim()} onClick={() => void create()}>
            Uložiť
          </Button>
        </>
      }
    >
      {error ? <p role="alert">{error}</p> : null}
      <Field label="Názov">
        <input className="k-input k-well" aria-label="Názov kategórie" value={name} onChange={(e) => setName(e.target.value)} />
      </Field>
      <Field label="Nadradená kategória">
        <select
          aria-label="Nadradená kategória"
          className="k-select k-well"
          value={parentId ?? ''}
          onChange={(e) => setParentId(e.target.value ? Number(e.target.value) : null)}
        >
          <option value="">Nová hlavná kategória</option>
          {parents.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </Field>
      <Field label="Druh">
        <select
          aria-label="Druh kategórie"
          className="k-select k-well"
          value={effectiveKind}
          disabled={selectedParent !== null}
          onChange={(e) => setKind(e.target.value as CategoryKind)}
        >
          <option value="expense">{CATEGORY_KIND_LABELS.expense}</option>
          <option value="income">{CATEGORY_KIND_LABELS.income}</option>
        </select>
      </Field>
    </Dialog>
  )
}
