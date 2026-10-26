// Kaliber 5 sprite: Phosphor Regular and own Kaliber drawings.
// Licence: THIRD_PARTY_NOTICES.md. Vite serves the vendored sprite from public/.
export type IconName = 'sun' | 'moon' | 'chevron-down' | 'close' | 'archive' | 'delete'
  | 'list' | 'chart-bar' | 'import' | 'folder' | 'settings'
  | 'info' | 'success' | 'warning' | 'danger'

export function Icon({ name, small = false, className = '' }: {
  name: IconName
  small?: boolean
  className?: string
}) {
  return (
    <svg className={`k-icon${small ? ' k-icon--sm' : ''}${className ? ` ${className}` : ''}`} aria-hidden="true" focusable="false">
      <use href={`/kaliber-icons.svg#k-icon-${name}`} />
    </svg>
  )
}
