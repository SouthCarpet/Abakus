import { useEffect, useId, useRef, useState, type KeyboardEvent } from 'react'
import { useEscapeLayer } from '../kaliber/escape'
import { ChevronGlyph } from '../kaliber/icons'

export interface MenuItem {
  label: string
  onSelect: () => void
  disabled?: boolean
  /** A separator above the item (Kaliber R13: a destructive item is last, after one). */
  separated?: boolean
  /** Key text shown at the end of the item (main menus show key hints, R22). */
  keyHint?: string
}

// The labelled "Ďalšie" menu at the end of the screen header row (Kaliber R12, R13): commands
// only, no navigation. WAI-ARIA APG menu button: arrows, Home and End move, Enter and Space
// run, Escape closes and returns the focus to the trigger (Escape layer 3), Tab closes.
export function MoreMenu({ items, label = 'Ďalšie' }: { items: MenuItem[]; label?: string }) {
  const [open, setOpen] = useState(false)
  const rootRef = useRef<HTMLDivElement>(null)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const listRef = useRef<HTMLDivElement>(null)
  const listId = useId()

  function close(focusTrigger: boolean) {
    setOpen(false)
    if (focusTrigger) triggerRef.current?.focus()
  }

  useEscapeLayer('popup', open, () => close(true), () => rootRef.current)

  useEffect(() => {
    if (!open) return
    listRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')?.focus()
    function onPointerDown(event: PointerEvent) {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false)
    }
    document.addEventListener('pointerdown', onPointerDown)
    return () => document.removeEventListener('pointerdown', onPointerDown)
  }, [open])

  function onListKey(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === 'Tab') { setOpen(false); return }
    const all = Array.from(listRef.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)') ?? [])
    if (all.length === 0) return
    const at = all.indexOf(document.activeElement as HTMLButtonElement)
    const next: Record<string, number> = { ArrowDown: (at + 1) % all.length, ArrowUp: (at - 1 + all.length) % all.length, Home: 0, End: all.length - 1 }
    if (!(event.key in next)) return
    event.preventDefault()
    all[next[event.key]].focus()
  }

  return (
    <div className="k-menu" ref={rootRef}>
      <button
        ref={triggerRef}
        type="button"
        className="k-btn k-btn-secondary k-btn-auto k-btn-menu"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={listId}
        onClick={() => setOpen((value) => !value)}
        onKeyDown={(event) => {
          if (event.key !== 'ArrowDown') return
          event.preventDefault()
          setOpen(true)
        }}
      >
        {label}
        <ChevronGlyph />
      </button>
      <div
        ref={listRef}
        id={listId}
        role="menu"
        aria-label={label}
        className={`k-menu-list k-menu-list--end${open ? ' is-open' : ''}`}
        onKeyDown={onListKey}
      >
        {items.map((item) => (
          <div key={item.label}>
            {item.separated ? <div className="k-menu-sep" role="separator" /> : null}
            <button
              type="button"
              role="menuitem"
              className="k-menu-item ab-menu-item"
              disabled={item.disabled}
              tabIndex={-1}
              onClick={() => { close(true); item.onSelect() }}
            >
              <span>{item.label}</span>
              {item.keyHint ? <span className="ab-menu-key">{item.keyHint}</span> : null}
            </button>
          </div>
        ))}
      </div>
    </div>
  )
}
