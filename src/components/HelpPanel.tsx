import { useEffect, useRef, useState } from 'react'
import { useEscapeLayer } from '../kaliber/escape'
import { BINDINGS } from '../kaliber/bindings'
import { keyLabel } from '../kaliber/keys'
import { CloseGlyph } from '../kaliber/icons'
import { trapTab } from '../kaliber/focusTrap'

export type HelpScreen = 'transactions' | 'overview' | 'import' | 'categories' | 'settings'

interface Topic {
  id: string
  screen: HelpScreen | 'keys'
  title: string
  lines: string[]
}

// Task help per screen (Kaliber R15): short steps, grouped by task, searchable. Abakus has no
// onboarding tour, so there is no "Show tour again" item.
export const HELP_TOPICS: Topic[] = [
  {
    id: 'transactions', screen: 'transactions', title: 'Transakcie: zaradenie platieb', lines: [
      'Stav Odhad znamená, že Abakus navrhol kategóriu podľa pravidla. Potvrďte ho tlačidlom alebo klávesom Enter na zameranom riadku.',
      'Nezaradené platby: nad zoznamom je riadok „Čakajú na zaradenie“. Tlačidlo Zaradiť podľa obchodníka otvorí skupiny platieb podľa obchodníka a miesta. Kliknutím na skupinu vyberiete jej platby, potom vyberiete kategóriu a stlačíte Priradiť.',      'Vytvorenie kategórie nevytvorí pravidlo. Priradenie kategórie alebo potvrdenie návrhu pri rozpoznateľnom obchodníkovi vytvorí pravidlo pre ďalšie platby.',
      'Použiť aj na podobné zaradí aj už importované nezaradené alebo navrhnuté platby rovnakého obchodníka. Potvrdený riadok sa nikdy neprepíše.',
      'Hromadné priradenie sa dá vrátiť tlačidlom Späť v hlásení vpravo dole alebo klávesmi Ctrl+Z. Ctrl+Y ho znova vykoná.',
      'Hľadanie spája obchodníka s miestom: Penny Neuss nenájde Penny Berlin.',
    ],
  },
  {
    id: 'overview', screen: 'overview', title: 'Prehľad a export PDF', lines: [
      'Prehľad sčíta príjmy, výdavky a poplatky za vybrané obdobie. Prevody medzi vlastnými účtami sú mimo súčtov.',
      'Exportovať PDF vytvorí výpis transakcií alebo sumár na 1 či 3 strany s vlastným výberom účtov a obdobia.',
    ],
  },
  {
    id: 'import', screen: 'import', title: 'Import výpisov', lines: [
      'Presuňte PDF výpisy Tatra banky do okna alebo stlačte Vybrať PDF (Ctrl+O).',
      'Zamknutý výpis si vypýta heslo. Heslo sa uloží iba vtedy, keď to zaškrtnete.',
      'Zmazanie výpisu je natrvalo. Dialóg najprv ukáže, koľko transakcií odstráni. Pôvodné PDF súbory zostanú. Del na riadku výpisu otvorí ten istý dialóg.',
    ],
  },
  {
    id: 'categories', screen: 'categories', title: 'Kategórie a pravidlá', lines: [
      'Pravidlo zaradí ďalšie platby rovnakého obchodníka. Cieľ pravidla zmeníte v detaile ním zaradenej transakcie.',
      'Kategóriu s transakciami možno archivovať. Zmazanie najprv ukáže, čo sa zmení. Del na riadku kategórie otvorí ten istý dialóg.',
    ],
  },
  {
    id: 'settings', screen: 'settings', title: 'Nastavenia, záloha a aktualizácie', lines: [
      'Záloha je nešifrovaný súbor SQLite. Obnova nahradí celú databázu naraz.',
      'Kontrola aktualizácií je predvolene vypnutá. Keď ju zapnete, Abakus sa pozrie na GitHub pri spustení (ticho), pri otvorení sekcie O aplikácii a na tlačidlo Skontrolovať aktualizácie. Samotné zapnutie prepínača nič neposiela. Pri vypnutej kontrole sa nič neposiela a tlačidlo je neaktívne. Každé volanie je v zozname Sieťová aktivita.',
      'Vzhľad: Svetlý, Tmavý alebo Podľa systému. Prepínač so slnkom a mesiacom v hlavičke mení ten istý stav. Voľba sa ukladá pod kľúčom kaliber-theme. Voľba zo staršej verzie (kľúč abakus.theme) sa pri prvom spustení prenesie.',
    ],
  },
]

function matches(topic: Topic, query: string): boolean {
  const q = query.trim().toLocaleLowerCase('sk')
  if (!q) return true
  return [topic.title, ...topic.lines].some((text) => text.toLocaleLowerCase('sk').includes(q))
}

// The help panel (Kaliber R12, R15): a drawer from the right with a search field and the
// topics as an accordion (several open, R19), the current screen's topic open. F1 and the
// footer link open it. Escape clears the search text first, then closes the panel; the focus
// returns to the element that opened it.
export function HelpPanel({ open, screen, onClose }: { open: boolean; screen: HelpScreen; onClose: () => void }) {
  const [query, setQuery] = useState('')
  const [shown, setShown] = useState(false)
  const panelRef = useRef<HTMLDivElement>(null)
  const searchRef = useRef<HTMLInputElement>(null)

  useEscapeLayer('dialog', open, onClose, () => panelRef.current)
  useEscapeLayer('search', open && query !== '', () => setQuery(''), () => searchRef.current)

  useEffect(() => {
    if (!open) { setShown(false); setQuery(''); return }
    const previous = document.activeElement as HTMLElement | null
    const frame = requestAnimationFrame(() => setShown(true))
    searchRef.current?.focus()
    const onKeyDown = (event: KeyboardEvent) => trapTab(event, panelRef.current)
    document.addEventListener('keydown', onKeyDown)
    return () => { cancelAnimationFrame(frame); document.removeEventListener('keydown', onKeyDown); previous?.focus() }
  }, [open])

  if (!open) return null
  const topics = HELP_TOPICS.filter((topic) => matches(topic, query))
  const keys = BINDINGS.filter((b) => matches({ id: b.id, screen: 'keys', title: b.action, lines: [b.where] }, query))
  return (
    <div className={`k-drawer-root${shown ? ' is-open' : ''}`} data-k-drawer="">
      <div className="k-scrim" onClick={onClose} />
      <div className="k-drawer" role="dialog" aria-modal="true" aria-labelledby="ab-help-title" ref={panelRef}>
        <div className="k-drawer-head">
          <h2 className="k-dialog-title" id="ab-help-title">Pomocník</h2>
          <button type="button" className="k-x-btn" aria-label="Zavrieť pomocníka" onClick={onClose}><CloseGlyph /></button>
        </div>
        <div className="k-drawer-body">
          <input
            ref={searchRef}
            type="search"
            className="k-input ab-help-search"
            aria-label="Hľadať v pomocníkovi"
            placeholder="Hľadať v pomocníkovi"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          {topics.length === 0 && keys.length === 0 ? <p role="status">Žiadna téma nezodpovedá hľadaniu.</p> : null}
          <div className="k-accordion">
            {topics.map((topic) => (
              <details key={topic.id} className="k-disclosure" open={topic.screen === screen || query !== ''}>
                <summary className="k-disclosure-summary">{topic.title}</summary>
                <div className="k-disclosure-body ab-section">
                  {topic.lines.map((line) => <p key={line}>{line}</p>)}
                </div>
              </details>
            ))}
            {keys.length > 0 ? (
              <details className="k-disclosure" open>
                <summary className="k-disclosure-summary">Klávesové skratky</summary>
                <div className="k-disclosure-body">
                  <dl className="ab-help-keys">
                    {keys.map((b) => (
                      <div key={b.id} style={{ display: 'contents' }}>
                        <dt>{keyLabel(b.spec)}</dt>
                        <dd>{b.action} ({b.where})</dd>
                      </div>
                    ))}
                  </dl>
                </div>
              </details>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  )
}
