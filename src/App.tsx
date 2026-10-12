import { useEffect, useRef, useState } from 'react'
import type { AccountKind, Status } from './api'
import { HelpPanel } from './components/HelpPanel'
import { ThemeSwitch } from './components/ThemeControls'
import { Toasts } from './components/Toasts'
import { UpdateIndicator } from './components/UpdateIndicator'
import { useActivity } from './kaliber/activity'
import { ariaKeys, withKey } from './kaliber/bindings'
import { matchKey } from './kaliber/keys'
import { useTheme } from './kaliber/theme'
import { checkAtStart } from './lib/updateCheck'
import { Categories } from './screens/Categories'
import { Import } from './screens/Import'
import { Overview } from './screens/Overview'
import { Settings } from './screens/Settings'
import { Transactions } from './screens/Transactions'

type Screen = 'transactions' | 'overview' | 'import' | 'categories' | 'settings'

interface TransactionsEntry {
  statementId?: number
  status?: Status
  categoryId?: number
  // Point 17: a merchant click in Overview carries the merchant name here as
  // the Transactions search text (point 9's substring search).
  merchantText?: string
  accountKind?: AccountKind
}

// Kaliber R12: 5 sections, so header tabs; Settings last. The register (Transakcie) is the
// signature screen and opens first (identity lever R16, docs/plan-091-kaliber-r3.md).
const TABS: { id: Screen; label: string }[] = [
  { id: 'transactions', label: 'Transakcie' },
  { id: 'overview', label: 'Prehľad' },
  { id: 'import', label: 'Import' },
  { id: 'categories', label: 'Kategórie' },
  { id: 'settings', label: 'Nastavenia' },
]

function useAppKeys(handlers: { help: () => void; settings: () => void; importFiles: () => void }) {
  const latest = useRef(handlers)
  latest.current = handlers
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (matchKey(event, 'F1')) { event.preventDefault(); latest.current.help(); return }
      if (matchKey(event, 'F5') || matchKey(event, 'primary+r') || matchKey(event, 'primary+shift+r')) {
        // Tauri: a page reload would drop text typed in a dialog (Kaliber map, F5 and Ctrl+R).
        event.preventDefault()
        return
      }
      if (document.querySelector('[aria-modal="true"]')) return
      if (matchKey(event, 'primary+comma')) { event.preventDefault(); latest.current.settings(); return }
      if (matchKey(event, 'primary+o')) { event.preventDefault(); latest.current.importFiles() }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])
}

export function App() {
  useTheme()
  const [screen, setScreen] = useState<Screen>('transactions')
  const [transactionsEntry, setTransactionsEntry] = useState<TransactionsEntry>({})
  const [entryVersion, setEntryVersion] = useState(0)
  const [helpOpen, setHelpOpen] = useState(false)
  const [aboutRequest, setAboutRequest] = useState(0)
  // Ctrl+O: 1 asks Import to open the file chooser once; Import resets it to 0.
  const [pickRequest, setPickRequest] = useState(0)
  const activity = useActivity()

  // 091/B10 p3 fix: bumped once a restore actually lands (RestoreSection's
  // onRestored, bubbled through Settings), so any screen whose data effects
  // depend on it drops its pre-restore DB state and refetches.
  const [dbGeneration, setDbGeneration] = useState(0)

  useEffect(() => {
    // The start path of the update check: quiet, and only while the user has turned it on.
    void checkAtStart()
  }, [])

  useAppKeys({
    help: () => setHelpOpen(true),
    settings: () => setScreen('settings'),
    importFiles: () => { setScreen('import'); setPickRequest(1) },
  })

  function goToTransactions(entry: TransactionsEntry = {}) {
    setTransactionsEntry(entry)
    setEntryVersion((version) => version + 1)
    setScreen('transactions')
  }

  function selectScreen(id: Screen) {
    if (id === 'transactions') goToTransactions()
    else setScreen(id)
  }

  function openAbout() {
    setAboutRequest((value) => value + 1)
    setScreen('settings')
  }

  const busy = activity.running.length > 0
  return (
    <div className="k-appshell ab-app">
      <header className={`k-header ab-app-header${busy ? ' is-busy' : ''}`}>
        <div className="k-shell">
          <span className="k-header-name ab-brand">
            <img src="/icon.svg" alt="" width={24} height={24} />
            Abakus
          </span>
          <nav className="k-tabs" aria-label="Sekcie">
            {TABS.map((tab) => (
              <button
                key={tab.id}
                type="button"
                className={`k-tab${tab.id === screen ? ' is-active' : ''}`}
                aria-current={tab.id === screen ? 'page' : undefined}
                aria-keyshortcuts={tab.id === 'settings' ? ariaKeys('settings') : undefined}
                title={tab.id === 'settings' ? withKey(tab.label, 'settings') : undefined}
                onClick={() => selectScreen(tab.id)}
              >
                {tab.label}
              </button>
            ))}
          </nav>
          <div className="k-header-tools">
            <UpdateIndicator onOpenAbout={openAbout} />
            <ThemeSwitch />
          </div>
        </div>
        <span className="k-activity" aria-hidden="true"><span className="k-activity-bar" /></span>
      </header>
      <div className="k-appshell-body">
        <main className="k-appshell-main" data-k-sheet="">
          {screen === 'transactions' ? (
            <Transactions
              key={entryVersion}
              dbGeneration={dbGeneration}
              statementId={transactionsEntry.statementId}
              initialStatus={transactionsEntry.status ?? null}
              initialCategoryId={transactionsEntry.categoryId ?? null}
              initialAccountKind={transactionsEntry.accountKind ?? null}
              initialText={transactionsEntry.merchantText ?? ''}
              onNavigateToImport={() => setScreen('import')}
            />
          ) : null}
          {screen === 'overview' ? (
            <Overview onNavigateToImport={() => setScreen('import')} onNavigateToTransactions={goToTransactions} />
          ) : null}
          {screen === 'import' ? <Import pickRequest={pickRequest} onPickRequestHandled={() => setPickRequest(0)} onNavigateToTransactions={(statementId) => goToTransactions({ statementId })} /> : null}
          {screen === 'categories' ? <Categories /> : null}
          {screen === 'settings' ? <Settings key={aboutRequest} aboutOpen={aboutRequest > 0} onRestored={() => setDbGeneration((value) => value + 1)} /> : null}
        </main>
      </div>
      <footer className="k-footer">
        <div className="k-shell">
          <span className="ab-footer-status" role="status" aria-live="polite">
            {busy ? activity.running.join(', ') : activity.note}
          </span>
          <div className="k-footer-links">
            <button type="button" className="k-link" aria-keyshortcuts="F1" title={withKey('Pomocník', 'help')} onClick={() => setHelpOpen(true)}>
              Pomocník
            </button>
          </div>
        </div>
      </footer>
      <HelpPanel open={helpOpen} screen={screen} onClose={() => setHelpOpen(false)} />
      <Toasts />
    </div>
  )
}
