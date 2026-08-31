import type { ReactNode } from 'react'

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="k-field">
      <span className="k-field-label">{label}</span>
      {children}
    </label>
  )
}
