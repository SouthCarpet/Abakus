import { useEffect, useRef, useState, type ReactNode } from 'react'
import { useEscapeLayer } from '../kaliber/escape'
import { focusableIn, trapTab } from '../kaliber/focusTrap'

// A Kaliber dialog (`.k-modal-root[data-k-modal]`): it opens from the centre with the dialog
// motion (kaliber.css, `is-open` one frame after mount). Escape closes it through the Escape
// layer stack (layer 8, src/kaliber/escape.ts), so a popup or a toast above it closes first.
// Tab never leaves it while it is open. `wide` gives the report dialog its width. The element
// with `data-k-autofocus` takes the focus on open; without it the first focusable one does.
export function Dialog({
  open,
  title,
  onClose,
  children,
  actions,
  wide = false,
}: {
  open: boolean
  title: string
  onClose: () => void
  children: ReactNode
  actions?: ReactNode
  wide?: boolean
}) {
  const rootRef = useRef<HTMLDivElement>(null)
  const [shown, setShown] = useState(false)

  useEscapeLayer('dialog', open, onClose, () => rootRef.current)

  useEffect(() => {
    if (!open) { setShown(false); return }
    const frame = requestAnimationFrame(() => setShown(true))
    const previousFocus = document.activeElement as HTMLElement | null
    const root = rootRef.current ?? document.createElement('div')
    // `data-k-autofocus` names the element that takes the focus (Cancel in a named confirm, R14).
    const first = root.querySelector<HTMLElement>('[data-k-autofocus]:not(:disabled)') ?? focusableIn(root)[0]
    first?.focus()

    const onKeyDown = (event: KeyboardEvent) => trapTab(event, rootRef.current)
    document.addEventListener('keydown', onKeyDown)
    return () => {
      cancelAnimationFrame(frame)
      document.removeEventListener('keydown', onKeyDown)
      previousFocus?.focus()
    }
  }, [open])

  if (!open) return null
  return (
    <div className={`k-modal-root${shown ? ' is-open' : ''}`} data-k-modal="">
      <div className="k-scrim" onClick={onClose} />
      <div
        className={`k-dialog ${wide ? 'ab-dialog-wide' : 'ab-dialog'}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        ref={rootRef}
      >
        <div className="k-dialog-title">{title}</div>
        <div className="k-dialog-body ab-dialog-body">{children}</div>
        {actions ? <div className="k-dialog-actions">{actions}</div> : null}
      </div>
    </div>
  )
}
