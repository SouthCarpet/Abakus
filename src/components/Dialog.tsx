import { useEffect, useRef, useState, type ReactNode } from 'react'
import { useEscapeLayer } from '../kaliber/escape'

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'

function focusableIn(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR))
}

// A Kaliber dialog (`.k-modal-root[data-k-modal]`): it opens from the centre with the dialog
// motion (kaliber.css, `is-open` one frame after mount). Escape closes it through the Escape
// layer stack (layer 8, src/kaliber/escape.ts), so a popup or a toast above it closes first.
// Tab never leaves it while it is open. `wide` gives the report dialog its width.
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
    focusableIn(rootRef.current ?? document.createElement('div'))[0]?.focus()

    function onKeyDown(event: KeyboardEvent) {
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
