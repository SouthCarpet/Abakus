import { useState } from 'react'
import type { Status } from './api'
import { Card } from './components/Card'
import { Rail } from './components/Rail'
import { Import } from './screens/Import'
import { Overview } from './screens/Overview'
import { Settings } from './screens/Settings'
import { Transactions } from './screens/Transactions'

type Screen = 'overview' | 'transactions' | 'import' | 'categories' | 'settings'

interface TransactionsEntry {
  statementId?: number
  status?: Status
}

const RAIL_ITEMS: { id: Screen; label: string }[] = [
  { id: 'overview', label: 'Prehľad' },
  { id: 'import', label: 'Import' },
  { id: 'transactions', label: 'Transakcie' },
  { id: 'categories', label: 'Kategórie' },
  { id: 'settings', label: 'Nastavenia' },
]

function Placeholder({ title }: { title: string }) {
  return <Card title={title}>Táto obrazovka príde v ďalšej úlohe.</Card>
}

export function App() {
  const [screen, setScreen] = useState<Screen>('overview')
  const [transactionsEntry, setTransactionsEntry] = useState<TransactionsEntry>({})

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
          <Transactions statementId={transactionsEntry.statementId} initialStatus={transactionsEntry.status ?? null} />
        ) : null}
        {screen === 'import' ? <Import onNavigateToTransactions={(statementId) => goToTransactions({ statementId })} /> : null}
        {screen === 'categories' ? <Placeholder title="Kategórie" /> : null}
        {screen === 'settings' ? <Settings /> : null}
      </section>
    </div>
  )
}
