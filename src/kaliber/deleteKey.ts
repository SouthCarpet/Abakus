// Del on a list row (Kaliber keyboard map: "standard where a delete exists"; selection model,
// `confirm` mode). Every delete in Abakus is irreversible, so Del only opens the existing named
// confirm dialog (it names the item and the count, the focus is on Cancel). It never deletes
// by itself. After a delete the focus goes to the next row, or to the previous row when the
// last row went. Del typed in a text field edits the text.

import { useEffect, useRef, type KeyboardEvent } from 'react'
import { matchKey } from './keys'

function isField(target: EventTarget | null): boolean {
  const tag = (target as HTMLElement | null)?.tagName
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT'
}

function rowsOf(container: HTMLElement | null): HTMLElement[] {
  return Array.from(container?.querySelectorAll<HTMLElement>('[data-k-item]') ?? [])
}

/** `count` is the number of rows; the focus moves when it changes after a delete. */
export function useDeleteKey<E extends HTMLElement>(count: number) {
  const containerRef = useRef<E>(null)
  const rowIndex = useRef(-1)
  const pending = useRef<number | null>(null)

  useEffect(() => {
    if (pending.current === null) return
    const rows = rowsOf(containerRef.current)
    const at = Math.min(pending.current, rows.length - 1)
    pending.current = null
    rows[at]?.focus()
  }, [count])

  function remember(row: HTMLElement) {
    rowIndex.current = rowsOf(containerRef.current).indexOf(row)
  }

  /** Props for one row. `open` opens the named confirm; omit it for a row with no delete. */
  function rowProps(open?: () => void) {
    return {
      tabIndex: 0,
      'data-k-item': '',
      onClickCapture: (event: { currentTarget: HTMLElement }) => remember(event.currentTarget),
      onKeyDown: (event: KeyboardEvent<HTMLElement>) => {
        if (!open || isField(event.target) || !matchKey(event.nativeEvent, 'Delete')) return
        event.preventDefault()
        remember(event.currentTarget)
        open()
      },
    }
  }

  /** Call when the confirmed delete succeeded. */
  function deleted() {
    pending.current = rowIndex.current
  }

  return { containerRef, rowProps, deleted }
}
