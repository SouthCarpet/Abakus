// Toasts (Kaliber R12 and R14, port of `Kaliber.toast` in design/web/kaliber-motion.js): the
// result of a user action, at the bottom right. A reversible action acts at once and its toast
// carries the action button (Späť); the button runs it once and removes the toast. A toast
// with an action stays 8 seconds, one without 3.2 seconds (the same constants as
// kaliber-motion.js, TOAST_ACTION_MS and TOAST_MS). Escape removes the newest (layer 2).

import { useSyncExternalStore } from 'react'

export const TOAST_TIME = 3200
export const TOAST_ACTION_TIME = 8000

export interface ToastAction {
  label: string
  run: () => void
}

export interface ToastItem {
  id: number
  text: string
  action?: ToastAction
  leaving: boolean
}

let items: ToastItem[] = []
let nextId = 1
const timers = new Map<number, ReturnType<typeof setTimeout>>()
const listeners = new Set<() => void>()

function emit() {
  listeners.forEach((listen) => listen())
}

export function toast(text: string, action?: ToastAction): number {
  const id = nextId++
  items = [...items, { id, text, action, leaving: false }]
  timers.set(id, setTimeout(() => dismissToast(id), action ? TOAST_ACTION_TIME : TOAST_TIME))
  emit()
  return id
}

/** Starts the leave; the region removes the item when its leave ends. */
export function dismissToast(id: number): void {
  const timer = timers.get(id)
  if (timer) clearTimeout(timer)
  timers.delete(id)
  if (!items.some((item) => item.id === id && !item.leaving)) return
  items = items.map((item) => (item.id === id ? { ...item, leaving: true } : item))
  emit()
}

export function removeToast(id: number): void {
  items = items.filter((item) => item.id !== id)
  emit()
}

/** Removes a toast that offers an action which is no longer valid (another write made the
 *  undo stale). */
export function dropToastsWithAction(): void {
  items.filter((item) => item.action && !item.leaving).forEach((item) => dismissToast(item.id))
}

export function liveToasts(): ToastItem[] {
  return items.filter((item) => !item.leaving)
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function useToasts(): ToastItem[] {
  return useSyncExternalStore(subscribe, () => items)
}

/** Test seam. */
export function resetToasts(): void {
  timers.forEach((timer) => clearTimeout(timer))
  timers.clear()
  items = []
  emit()
}
