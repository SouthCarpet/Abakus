import type { ReactNode } from 'react'

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="ab-field">
      <span className="ab-field-label">{label}</span>
      {children}
    </label>
  )
}
