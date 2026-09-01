import { useEffect, useRef, type ReactNode } from 'react'

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'

function focusableIn(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR))
}

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
  const rootRef = useRef<HTMLDivElement>(null)

  // Escape closes the dialog, and Tab never leaves it while it is open: both
  // are keyboard requirements a mouse-only scrim click and a `disabled`
  // field do not cover on their own.
  useEffect(() => {
    if (!open) return
    focusableIn(rootRef.current ?? document.createElement('div'))[0]?.focus()

    function onKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        onClose()
        return
      }
      if (event.key !== 'Tab') return
      const root = rootRef.current
      if (!root) return
      const focusable = focusableIn(root)
      if (focusable.length === 0) return
      const first = focusable[0]
      const last = focusable[focusable.length - 1]
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault()
        last.focus()
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault()
        first.focus()
      }
    }
    document.addEventListener('keydown', onKeyDown)
    return () => document.removeEventListener('keydown', onKeyDown)
  }, [open, onClose])

  if (!open) return null
  return (
    <div className="k-modal-root">
      <div className="k-scrim" onClick={onClose}>
        <div className="k-milled k-dialog" role="dialog" aria-modal="true" aria-label={title} ref={rootRef} onClick={(event) => event.stopPropagation()}>
          <div className="k-dialog-title">{title}</div>
          <div className="k-dialog-body">{children}</div>
          {actions ? <div className="k-dialog-actions">{actions}</div> : null}
        </div>
      </div>
    </div>
  )
}
