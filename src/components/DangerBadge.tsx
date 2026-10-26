import type { ReactNode } from 'react'
import { Icon } from '../kaliber/icons'

export function DangerBadge({ children }: { children: ReactNode }) {
  return <span className="k-card-badge ab-badge-danger"><Icon name="danger" small />{children}</span>
}
