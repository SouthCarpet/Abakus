import { Icon } from '../kaliber/icons'
import { type ReactNode } from 'react'

export function OperationStatus({ error, busy, children = 'Prebieha operácia...' }: { error: string; busy?: boolean; children?: ReactNode }) {
  return <>
    {error ? <div className="k-state is-error" role="alert"><span className="k-state-icon" aria-hidden="true"><Icon name="danger" small /></span><span>{error}</span></div> : null}
    {busy ? <p role="status">{children}</p> : null}
  </>
}
