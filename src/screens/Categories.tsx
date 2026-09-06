import { useEffect, useRef, useState } from 'react'
import type { Category, CategoryKind, RuleView } from '../api'
import { api } from '../api'
import { Button } from '../components/Button'
import { Card } from '../components/Card'
import { Dialog } from '../components/Dialog'
import { Field } from '../components/Field'
import { useAction } from '../lib/useAction'
import { groupCategories } from '../lib/categories'

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
    <table className="k-table">
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
}: {
  category: Category
  indent: boolean
  onRename: (id: number, name: string) => void
  onArchive: (category: Category) => void
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
}: {
  categories: Category[]
  onRename: (id: number, name: string) => void
  onAddSub: (parent: Category) => void
  onArchive: (category: Category) => void
}) {
  const groups = groupCategories(categories)
  return (
    <div className="k-section">
      {groups.map(({ parent, subs }) => (
        <div key={parent.id} className="k-section">
          <CategoryRow category={parent} indent={false} onRename={onRename} onArchive={onArchive} />
          {subs.map((sub) => (
            <CategoryRow key={sub.id} category={sub} indent onRename={onRename} onArchive={onArchive} />
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
              />
            </Card>
          </div>
          <div style={{ flex: 1, minWidth: 320 }}>
            <Card title="Pravidlá">
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
    </div>
  )
}
