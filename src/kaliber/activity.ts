// Background activity (Kaliber motion set row d and register motion P1): while a process
// runs, the header shows the 2px activity line and the footer says what runs; idle, the line
// is gone. At the end the footer keeps a dated receipt ("Import dokončený 14:05"). The line is
// the one loop of the screen: <html data-k-loop="header"> names its owner (kaliber.css).
// The footer status is the app's one polite live region.

import { useSyncExternalStore } from 'react'

interface ActivityState {
  running: string[]
  note: string
}

let state: ActivityState = { running: [], note: '' }
const listeners = new Set<() => void>()

function set(next: ActivityState) {
  state = next
  const root = document.documentElement
  if (state.running.length > 0) root.dataset.kLoop = 'header'
  else if (root.dataset.kLoop === 'header') delete root.dataset.kLoop
  listeners.forEach((listen) => listen())
}

export function clockText(at: Date): string {
  return `${String(at.getHours()).padStart(2, '0')}:${String(at.getMinutes()).padStart(2, '0')}`
}

/** Starts one running process. The returned function ends it; with a receipt the footer
 *  keeps "<receipt> <HH:MM>". */
export function startActivity(label: string): (receipt?: string) => void {
  set({ running: [...state.running, label], note: state.note })
  let done = false
  return (receipt?: string) => {
    if (done) return
    done = true
    const index = state.running.indexOf(label)
    const running = index > -1 ? [...state.running.slice(0, index), ...state.running.slice(index + 1)] : state.running
    set({ running, note: receipt ? `${receipt} ${clockText(new Date())}` : state.note })
  }
}

/** A note in the footer without a running process (the ink cap note, F2). */
export function footerNote(text: string): void {
  set({ running: state.running, note: text })
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function useActivity(): ActivityState {
  return useSyncExternalStore(subscribe, () => state)
}

/** Test seam. */
export function resetActivity(): void {
  set({ running: [], note: '' })
}
