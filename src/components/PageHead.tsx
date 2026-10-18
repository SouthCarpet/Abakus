import type { ReactNode } from 'react'

// The screen head (Kaliber R12, R24, R34 a): one large title, one muted metadata line, then
// at most one primary action and the "Ďalšie" menu in the action group at the end of the row.
export function PageHead({ title, meta, primary, more }: { title: string; meta?: string; primary?: ReactNode; more?: ReactNode }) {
  return (
    <div className="ab-head">
      <div className="k-page-head">
        <h2 className="k-page-title">{title}</h2>
        {meta ? <p className="k-page-meta">{meta}</p> : null}
      </div>
      {primary || more ? <div className="ab-head-actions">{primary}{more}</div> : null}
    </div>
  )
}
