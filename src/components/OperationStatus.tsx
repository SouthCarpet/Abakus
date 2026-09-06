import { type ReactNode } from 'react'

export function OperationStatus({ error, busy, children = 'Prebieha operácia...' }: { error: string; busy?: boolean; children?: ReactNode }) {
  return <>
    {error ? <p role="alert">{error}</p> : null}
    {busy ? <p role="status">{children}</p> : null}
  </>
}
