import { useEffect, useRef, useState } from 'react'
import type { Category, CategoryKind, RuleView } from '../api'
import { api } from '../api'
import { Button } from '../components/Button'
import { Card } from '../components/Card'
import { Dialog } from '../components/Dialog'
import { Field } from '../components/Field'
import { useAction } from '../lib/useAction'
import { groupCategories } from '../lib/categories'
import { categoryApi, type CategoryUpdatePreview } from '../lib/category-api'

const RULE_KIND_LABELS: Record<RuleView['kind'], string> = {
  exact: 'presné',
  merchant: 'obchodník',
  counterparty_account: 'účet',
  seed: 'slovník',
}

const CATEGORY_KIND_LABELS: Record<CategoryKind, string> = { expense: 'Výdavok', income: 'Príjem' }

const ARCHIVE_CONFIRM_TEXT = 'Kategória sa skryje z výberu. Historické priradenia ostávajú.'

export function RulesTable({ rules, onDelete }: { rules: RuleView[]; onDelete: (id: number) => void }) {
  return (
    <table className="k-table k-rules-table">
      <thead>
        <tr>
          <th>Kľúč</th>
          <th>Miesto</th>
          <th>Kategória</th>
          <th>Druh</th>
          <th className="k-num">Zásahy</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {rules.map((rule) => (
          <tr key={rule.id}>
            <td>{rule.key}</td>
            <td>{rule.place ?? ''}</td>
            <td>{rule.parent_name ? `${rule.parent_name} / ${rule.category_name}` : rule.category_name}</td>
            <td>
              <span className="k-card-badge">{RULE_KIND_LABELS[rule.kind]}</span>
            </td>
            <td className="k-num">{rule.hit_count}</td>
            <td>
              {rule.kind !== 'seed' ? (
                <Button variant="danger" onClick={() => onDelete(rule.id)}>
                  Zmazať
                </Button>
              ) : null}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

function CategoryRow({
  category,
  indent,
  onRename,
  onArchive,
  onEdit,
}: {
  category: Category
  indent: boolean
  onRename: (id: number, name: string) => void
  onArchive: (category: Category) => void
  onEdit: (category: Category) => void
}) {
  const [name, setName] = useState(category.name)
  useEffect(() => setName(category.name), [category.name])

  function commit() {
    const trimmed = name.trim()
    if (trimmed && trimmed !== category.name) onRename(category.id, trimmed)
    else setName(category.name)
  }

  return (
    <div className="k-row" style={indent ? { marginLeft: 'var(--space-6)' } : undefined}>
      <input
        className="k-input k-well"
        aria-label={`Názov kategórie ${category.id}`}
        value={name}
        disabled={category.system}
        onChange={(e) => setName(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
        }}
      />
      {!category.system ? (
        <Button variant="secondary" onClick={() => onEdit(category)}>
          Upraviť
        </Button>
      ) : null}
      {!category.system ? (
        <Button variant="ghost" onClick={() => onArchive(category)}>
          Archivovať
        </Button>
      ) : null}
    </div>
  )
}

function CategoryTree({
  categories,
  onRename,
  onAddSub,
  onArchive,
  onEdit,
}: {
  categories: Category[]
  onRename: (id: number, name: string) => void
  onAddSub: (parent: Category) => void
  onArchive: (category: Category) => void
  onEdit: (category: Category) => void
}) {
  const groups = groupCategories(categories)
  return (
    <div className="k-section">
      {groups.map(({ parent, subs }) => (
        <div key={parent.id} className="k-section">
          <CategoryRow category={parent} indent={false} onRename={onRename} onArchive={onArchive} onEdit={onEdit} />
          {subs.map((sub) => (
            <CategoryRow key={sub.id} category={sub} indent onRename={onRename} onArchive={onArchive} onEdit={onEdit} />
          ))}
          <div className="k-row" style={{ marginLeft: 'var(--space-6)' }}>
            <Button variant="ghost" onClick={() => onAddSub(parent)}>
              + podkategória
            </Button>
          </div>
        </div>
      ))}
    </div>
  )
}

const CATEGORY_KIND_LABELS_SHORT: Record<CategoryKind, string> = { expense: 'Výdavok', income: 'Príjem' }

// Section 9 C Categories flow: edit -> preview -> show affected/confirmed
// counts if effective kind changes -> explicit confirm -> atomic update ->
// caller refreshes tree and rules.
function CategoryEditDialog({
  category,
  categories,
  onClose,
  onSaved,
}: {
  category: Category | null
  categories: Category[]
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [name, setName] = useState('')
  const [parentId, setParentId] = useState<number | null>(null)
  const [kind, setKind] = useState<CategoryKind>('expense')
  const [preview, setPreview] = useState<CategoryUpdatePreview | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!category) return
    setName(category.name)
    setParentId(category.parent_id)
    setKind(category.kind)
    setPreview(null)
    setError('')
  }, [category])

  if (!category) return null

  // A root already carrying children cannot move under another root (the
  // backend refuses it too, third-level protection); an archived or system
  // category, or the edited category itself, is never a valid target.
  const hasChildren = categories.some((c) => c.parent_id === category.id)
  const parentOptions = categories.filter((c) => c.parent_id === null && c.id !== category.id && !c.system && !c.archived && !(hasChildren && category.parent_id === null))
  const selectedParent = parentOptions.find((p) => p.id === parentId) ?? null
  const effectiveKind = selectedParent ? selectedParent.kind : kind

  function request(acknowledge: boolean) {
    if (!category) throw new Error('no category selected')
    return { id: category.id, parent_id: parentId, name, kind, acknowledge_kind_change: acknowledge }
  }

  async function save() {
    setBusy(true)
    setError('')
    try {
      const p = await categoryApi.preview(request(false))
      if (p.requires_confirmation) {
        setPreview(p)
        return
      }
      await categoryApi.update(request(false))
      await onSaved()
      onClose()
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  async function confirmKindChange() {
    setBusy(true)
    setError('')
    try {
      await categoryApi.update(request(true))
      await onSaved()
      onClose()
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog
      open
      title={`Upraviť kategóriu ${category.name}`}
      onClose={() => {
        if (!busy) onClose()
      }}
      actions={
        preview ? (
          <>
            <Button variant="secondary" disabled={busy} onClick={() => setPreview(null)}>
              Zrušiť
            </Button>
            <Button variant="primary" disabled={busy} onClick={() => void confirmKindChange()}>
              Potvrdiť zmenu druhu
            </Button>
          </>
        ) : (
          <>
            <Button variant="secondary" disabled={busy} onClick={onClose}>
              Zrušiť
            </Button>
            <Button variant="primary" disabled={busy || !name.trim()} onClick={() => void save()}>
              Uložiť
            </Button>
          </>
        )
      }
    >
      {error ? <p role="alert">{error}</p> : null}
      {preview ? (
        <div className="k-section">
          <p>Zmena druhu ovplyvní {preview.affected_categories} kategórií a {preview.transaction_count} transakcií (z toho {preview.confirmed_count} potvrdených).</p>
          <p>Kategórie a súvisiace transakcie zostanú priradené, mení sa iba význam výdavok/príjem.</p>
        </div>
      ) : (
        <>
          <Field label="Názov">
            <input className="k-input k-well" aria-label="Upraviť názov kategórie" value={name} onChange={(e) => setName(e.target.value)} />
          </Field>
          <Field label="Nadradená kategória">
            <select
              aria-label="Upraviť nadradenú kategóriu"
              className="k-select k-well"
              value={parentId ?? ''}
              onChange={(e) => setParentId(e.target.value ? Number(e.target.value) : null)}
            >
              <option value="">Hlavná kategória</option>
              {parentOptions.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Druh">
            <select
              aria-label="Upraviť druh kategórie"
              className="k-select k-well"
              value={effectiveKind}
              disabled={selectedParent !== null}
              onChange={(e) => setKind(e.target.value as CategoryKind)}
            >
              <option value="expense">{CATEGORY_KIND_LABELS_SHORT.expense}</option>
              <option value="income">{CATEGORY_KIND_LABELS_SHORT.income}</option>
            </select>
          </Field>
        </>
      )}
    </Dialog>
  )
}

export function Categories() {
  const [categories, setCategories] = useState<Category[]>([])
  const [rules, setRules] = useState<RuleView[]>([])
  const action = useAction()
  const refreshRequest = useRef(0)
  const [addOpen, setAddOpen] = useState(false)
  const [newName, setNewName] = useState('')
  const [newKind, setNewKind] = useState<CategoryKind>('expense')
  const [subParent, setSubParent] = useState<Category | null>(null)
  const [subName, setSubName] = useState('')
  const [archiveTarget, setArchiveTarget] = useState<Category | null>(null)
  const [editTarget, setEditTarget] = useState<Category | null>(null)

  async function refresh() {
    const request = ++refreshRequest.current
    const [nextCategories, nextRules] = await Promise.all([api.listCategories(), api.listRules()])
    if (request !== refreshRequest.current) return
    setCategories(nextCategories)
    setRules(nextRules)
  }

  useEffect(() => {
    void refresh().catch((e) => action.setError(String(e)))
    return () => { refreshRequest.current++ }
  }, [])

  const withToast = action.run

  function rename(id: number, name: string) {
    void withToast(async () => {
      const cat = categories.find((c) => c.id === id)
      if (!cat) return
      await api.saveCategory(id, cat.parent_id, name, cat.kind)
      await refresh()
    })
  }

  function addTop() {
    void withToast(async () => {
      await api.saveCategory(null, null, newName, newKind)
      setAddOpen(false)
      setNewName('')
      setNewKind('expense')
      await refresh()
    })
  }

  function addSub() {
    if (!subParent) return
    const parent = subParent
    void withToast(async () => {
      await api.saveCategory(null, parent.id, subName, parent.kind)
      setSubParent(null)
      setSubName('')
      await refresh()
    })
  }

  function confirmArchive() {
    if (!archiveTarget) return
    const target = archiveTarget
    void withToast(async () => {
      await api.archiveCategory(target.id)
      setArchiveTarget(null)
      await refresh()
    })
  }

  function deleteRule(id: number) {
    void withToast(async () => {
      await api.deleteRule(id)
      await refresh()
    })
  }

  return (
    <div className="k-section">
      {action.error ? <p role="alert">{action.error}</p> : null}
      {action.busy ? <p role="status">Prebieha operácia...</p> : null}
      <fieldset disabled={action.busy} className="k-section">
        <div className="k-row" style={{ alignItems: 'stretch' }}>
          <div style={{ flex: 1, minWidth: 320 }}>
            <Card
              title="Kategórie"
              footer={
                <Button variant="primary" onClick={() => setAddOpen(true)}>
                  + kategória
                </Button>
              }
            >
              <CategoryTree
                categories={categories}
                onRename={rename}
                onAddSub={(parent) => setSubParent(parent)}
                onArchive={setArchiveTarget}
                onEdit={setEditTarget}
              />
            </Card>
          </div>
          <div className="k-rules-card-col" style={{ flex: 1, minWidth: 560 }}>
            <Card title="Pravidlá">
              <p className="k-field-label">
                Pravidlá vzniknú po priradení kategórie alebo potvrdení návrhu pri rozpoznateľnom obchodníkovi. Samotné vytvorenie kategórie pravidlo nevytvorí.
              </p>
              <div className="k-table-scroll" role="region" aria-label="Pravidlá" tabIndex={0}>
                <RulesTable rules={rules} onDelete={deleteRule} />
              </div>
            </Card>
          </div>
        </div>

      </fieldset>
      <Dialog
        open={addOpen}
        title="Pridať kategóriu"
        onClose={() => { if (!action.busy) setAddOpen(false) }}
        actions={
          <>
            <Button variant="secondary" disabled={action.busy} onClick={() => setAddOpen(false)}>
              Zrušiť
            </Button>
            <Button variant="primary" disabled={action.busy} onClick={addTop}>
              Uložiť
            </Button>
          </>
        }
      >
        {action.error ? <p role="alert">{action.error}</p> : null}
        <Field label="Názov">
          <input className="k-input k-well" value={newName} onChange={(e) => setNewName(e.target.value)} />
        </Field>
        <Field label="Druh">
          <select className="k-select k-well" value={newKind} onChange={(e) => setNewKind(e.target.value as CategoryKind)}>
            <option value="expense">{CATEGORY_KIND_LABELS.expense}</option>
            <option value="income">{CATEGORY_KIND_LABELS.income}</option>
          </select>
        </Field>
      </Dialog>

      <Dialog
        open={subParent !== null}
        title={`Pridať podkategóriu do ${subParent?.name ?? ''}`}
        onClose={() => { if (!action.busy) setSubParent(null) }}
        actions={
          <>
            <Button variant="secondary" disabled={action.busy} onClick={() => setSubParent(null)}>
              Zrušiť
            </Button>
            <Button variant="primary" disabled={action.busy} onClick={addSub}>
              Uložiť
            </Button>
          </>
        }
      >
        {action.error ? <p role="alert">{action.error}</p> : null}
        <Field label="Názov">
          <input className="k-input k-well" value={subName} onChange={(e) => setSubName(e.target.value)} />
        </Field>
      </Dialog>

      <Dialog
        open={archiveTarget !== null}
        title="Archivovať kategóriu"
        onClose={() => { if (!action.busy) setArchiveTarget(null) }}
        actions={
          <>
            <Button variant="secondary" disabled={action.busy} onClick={() => setArchiveTarget(null)}>
              Zrušiť
            </Button>
            <Button variant="primary" disabled={action.busy} onClick={confirmArchive}>
              Archivovať
            </Button>
          </>
        }
      >
        {action.error ? <p role="alert">{action.error}</p> : null}
        <p>{archiveTarget?.name}</p>
        <p>{ARCHIVE_CONFIRM_TEXT}</p>
      </Dialog>

      <CategoryEditDialog category={editTarget} categories={categories} onClose={() => setEditTarget(null)} onSaved={refresh} />
    </div>
  )
}
