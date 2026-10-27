import type { ReactNode } from 'react'
import { DangerBadge } from './DangerBadge'

export function Card({
  title,
  badge,
  badgeDanger = false,
  children,
  footer,
}: {
  title: string
  badge?: string
  badgeDanger?: boolean
  children: ReactNode
  footer?: ReactNode
}) {
  return (
    <section className="ab-section k-milled ab-section-body">
      <div className="k-card-header">
        <span className="k-card-title">{title}</span>
        {badge ? (badgeDanger ? <DangerBadge>{badge}</DangerBadge> : <span className="k-card-badge">{badge}</span>) : null}
      </div>
      <div className="k-card-body ab-card-body">{children}</div>
      {footer ? <div className="k-card-footer">{footer}</div> : null}
    </section>
  )
}
