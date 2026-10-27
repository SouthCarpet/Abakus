import type { ReactNode } from 'react'
import { Icon } from '../kaliber/icons'

export function InlineAlert({ children, kind = 'error', announce = true }: {
  children: ReactNode
  announce?: boolean
  kind?: 'info' | 'success' | 'warning' | 'error'
}) {
  return <div className={`k-alert k-alert--${kind}`} role={announce ? (kind === 'error' || kind === 'warning' ? 'alert' : 'status') : undefined}>
    <Icon name={kind === 'error' ? 'danger' : kind} small className="k-alert-icon" />
    <div className="k-alert-body">{children}</div>
  </div>
}
