import { Icon, type IconName } from '../kaliber/icons'
import type { Screen } from '../App'
import { usePhone } from '../kaliber/usePhone'

// Kaliber R31: on a phone the sections are the bottom navigation, icon and text always
// visible. The current target has the fill step, bold text and the dot.
const ICONS: Record<Screen, IconName> = {
  transactions: 'list',
  overview: 'chart-bar',
  import: 'import',
  categories: 'folder',
  settings: 'settings',
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
          className="k-bottom-nav-item ab-bottom-nav-item"
          aria-current={tab.id === current ? 'page' : undefined}
          onClick={() => onSelect(tab.id)}
        >
          <Icon name={ICONS[tab.id]} />
          <span className="k-bottom-nav-label">{tab.label}</span>
        </button>
      ))}
    </nav>
  )
}
