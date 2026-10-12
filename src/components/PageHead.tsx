import type { ReactNode } from 'react'

// The screen header row (Kaliber R12, `.k-pagehead`): the title, then at most one primary
// action, then the "Ďalšie" menu at the end of the same row.
export function PageHead({ title, primary, more }: { title: string; primary?: ReactNode; more?: ReactNode }) {
  return (
    <div className="k-pagehead">
      <h2 className="k-pagehead-title">{title}</h2>
      {primary}
      {more}
    </div>
  )
}
