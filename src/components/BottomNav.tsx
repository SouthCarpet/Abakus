import type { Screen } from '../App'
import { usePhone } from '../kaliber/usePhone'

// Kaliber R31: on a phone the sections are the bottom navigation, icon and text always
// visible. The current target has the fill step, bold text and the square mark.
const ICONS: Record<Screen, string> = {
  transactions: 'M4 6h16M4 12h16M4 18h10',
  overview: 'M6 20V11M12 20V4M18 20v-6',
  import: 'M12 4v11M7.5 10.5L12 15l4.5-4.5M5 20h14',
  categories: 'M4 4h8l8 8-8 8-8-8z',
  settings: 'M4 8h16M4 16h16M9 5v6M15 13v6',
}

export function BottomNav({ tabs, current, onSelect }: {
  tabs: { id: Screen; label: string }[]
  current: Screen
  onSelect: (id: Screen) => void
}) {
  const phone = usePhone()
  if (!phone) return null
  return (
    <nav className="k-bottom-nav ab-bottom-nav" aria-label="Sekcie">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          type="button"
          className="k-bottom-nav-item"
          aria-current={tab.id === current ? 'page' : undefined}
          onClick={() => onSelect(tab.id)}
        >
          <svg className="ab-nav-icon" viewBox="0 0 24 24" aria-hidden="true"><path d={ICONS[tab.id]} /></svg>
          <span className="k-bottom-nav-label">{tab.label}</span>
        </button>
      ))}
    </nav>
  )
}
