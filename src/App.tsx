import { useEffect, useState } from 'react'
import type { AccountKind, Status } from './api'
import { Rail } from './components/Rail'
import { watchSystemTheme } from './lib/theme'
import { Categories } from './screens/Categories'
import { Import } from './screens/Import'
import { Overview } from './screens/Overview'
import { Settings } from './screens/Settings'
import { Transactions } from './screens/Transactions'

type Screen = 'overview' | 'transactions' | 'import' | 'categories' | 'settings'

interface TransactionsEntry {
  statementId?: number
  status?: Status
  categoryId?: number
  accountKind?: AccountKind
}

const RAIL_ITEMS: { id: Screen; label: string }[] = [
  { id: 'overview', label: 'Prehľad' },
  { id: 'import', label: 'Import' },
  { id: 'transactions', label: 'Transakcie' },
  { id: 'categories', label: 'Kategórie' },
  { id: 'settings', label: 'Nastavenia' },
]

export function App() {
  const [screen, setScreen] = useState<Screen>('overview')
  const [transactionsEntry, setTransactionsEntry] = useState<TransactionsEntry>({})

  useEffect(() => watchSystemTheme(), [])

  function goToTransactions(entry: TransactionsEntry = {}) {
    setTransactionsEntry(entry)
    setScreen('transactions')
  }

  function selectScreen(id: string) {
    if (id === 'transactions') goToTransactions()
    else setScreen(id as Screen)
  }

  return (
    <div className="k-shell">
      <Rail items={RAIL_ITEMS} active={screen} onSelect={selectScreen} />
      <section className="k-page">
        {screen === 'overview' ? (
          <Overview onNavigateToImport={() => setScreen('import')} onNavigateToTransactions={goToTransactions} />
        ) : null}
        {screen === 'transactions' ? (
          <Transactions
            statementId={transactionsEntry.statementId}
            initialStatus={transactionsEntry.status ?? null}
            initialCategoryId={transactionsEntry.categoryId ?? null}
            initialAccountKind={transactionsEntry.accountKind ?? null}
          />
        ) : null}
        {screen === 'import' ? <Import onNavigateToTransactions={(statementId) => goToTransactions({ statementId })} /> : null}
        {screen === 'categories' ? <Categories /> : null}
        {screen === 'settings' ? <Settings /> : null}
      </section>
    </div>
  )
}
