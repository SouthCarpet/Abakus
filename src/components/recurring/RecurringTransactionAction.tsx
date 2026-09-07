import { useState } from 'react'
import type { Category, TxRow } from '../../api'
import { isRecurringEligible, localTodayIso } from '../../lib/recurring-labels'
import type { RecurringQuery, TransactionRecurringContext } from '../../lib/recurring-api'
import { recurringApi } from '../../lib/recurring-api'
import { Button } from '../Button'
import { CategoryDialog } from '../CategoryDialog'
import { RecurringEditor } from './RecurringEditor'

export function RecurringTransactionAction({
  row,
  categories,
  onChanged,
}: {
  row: TxRow
  categories: Category[]
  onChanged: () => Promise<void>
}) {
  if (!isRecurringEligible(row)) return null
  return <RecurringTransactionActionReady row={row} categories={categories} onChanged={onChanged} />
}

function RecurringTransactionActionReady({
  row,
  categories,
  onChanged,
}: {
  row: TxRow
  categories: Category[]
  onChanged: () => Promise<void>
}) {
  const [open, setOpen] = useState(false)
  const [context, setContext] = useState<TransactionRecurringContext | null>(null)
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const today = localTodayIso()
  const query: RecurringQuery = { from: null, to: null, account_kind: row.account_kind, today }
  const source = open && context ? { kind: 'transaction' as const, tx: row, context, query } : null

  async function openEditor() {
    if (busy) return
    setBusy(true)
    setError('')
    try {
      const value = await recurringApi.transactionContext(row.id, today)
      setContext(value)
      setOpen(true)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div>
      <Button
        variant="secondary"
        disabled={busy}
        aria-label={`Pravidelná platba ${row.id}`}
        onClick={() => void openEditor()}
      >
        Pravidelná platba
      </Button>
      {busy ? <p role="status">Načítava sa pravidelnosť...</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      <RecurringEditor
        source={source}
        categories={categories}
        CategoryDialog={CategoryDialog}
        onClose={() => { setOpen(false); setContext(null) }}
        onChanged={onChanged}
      />
    </div>
  )
}
