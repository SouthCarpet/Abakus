import { useState } from 'react'
import { Card } from './components/Card'
import { Rail } from './components/Rail'
import { Import } from './screens/Import'
import { Settings } from './screens/Settings'

type Screen = 'overview' | 'transactions' | 'import' | 'categories' | 'settings'

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
  return (
    <div className="k-shell">
      <Rail items={RAIL_ITEMS} active={screen} onSelect={(id) => setScreen(id as Screen)} />
      <section className="k-page">
        {screen === 'overview' ? <Placeholder title="Prehľad" /> : null}
        {screen === 'transactions' ? <Placeholder title="Transakcie" /> : null}
        {screen === 'import' ? <Import onNavigateToTransactions={() => setScreen('transactions')} /> : null}
        {screen === 'categories' ? <Placeholder title="Kategórie" /> : null}
        {screen === 'settings' ? <Settings /> : null}
      </section>
    </div>
  )
}
