import type { ReactNode } from 'react'

export function Card({
  title,
  badge,
  children,
  footer,
}: {
  title: string
  badge?: string
  children: ReactNode
  footer?: ReactNode
}) {
  return (
    <section className="k-section k-milled k-section-body">
      <div className="k-card-header">
        <span className="k-card-title">{title}</span>
        {badge ? <span className="k-card-badge">{badge}</span> : null}
      </div>
      <div className="k-card-body">{children}</div>
      {footer ? <div className="k-card-footer">{footer}</div> : null}
    </section>
  )
}
