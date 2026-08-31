import type { ReactNode } from 'react'

export function Dialog({
  open,
  title,
  onClose,
  children,
  actions,
}: {
  open: boolean
  title: string
  onClose: () => void
  children: ReactNode
  actions?: ReactNode
}) {
  if (!open) return null
  return (
    <div className="k-modal-root">
      <div className="k-scrim" onClick={onClose}>
        <div className="k-milled k-dialog" onClick={(event) => event.stopPropagation()}>
          <div className="k-dialog-title">{title}</div>
          <div className="k-dialog-body">{children}</div>
          {actions ? <div className="k-dialog-actions">{actions}</div> : null}
        </div>
      </div>
    </div>
  )
}
