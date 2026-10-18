// The Kaliber Escape layer stack (design/web/kaliber-keys.js, wiki components.md,
// "Escape layering"). One press of Escape acts on the top layer only, then the event
// stops. Rank, from the top: toast 2, popup 3, edit 4, place 5, search 6, selection 7,
// dialog 8. While a dialog is open, only the layers opened after it or inside it count,
// so a table selection behind a dialog stays when Escape closes the dialog. Composition
// and a pending dead key belong to the system. Escape never closes the window and never
// stops a running task.

import { useEffect, useRef } from 'react'

export type LayerKind = 'toast' | 'popup' | 'edit' | 'place' | 'search' | 'selection' | 'dialog'

const RANK: Record<LayerKind, number> = { toast: 2, popup: 3, edit: 4, place: 5, search: 6, selection: 7, dialog: 8 }

export interface Layer {
  kind: LayerKind
  el?: HTMLElement | null
  active?: () => boolean
  escape: (event: KeyboardEvent) => void
  order?: number
}

const layers: Layer[] = []
let serial = 0
let deadPending = false
let installed = false

export function popLayer(layer: Layer): void {
  const index = layers.indexOf(layer)
  if (index > -1) layers.splice(index, 1)
}

export function pushLayer(layer: Layer): Layer {
  install()
  popLayer(layer)
  serial += 1
  layer.order = serial
  layers.push(layer)
  return layer
}

const live = (layer: Layer) => !layer.active || layer.active()
const inside = (modal: Layer, layer: Layer) =>
  layer === modal || (layer.order ?? 0) > (modal.order ?? 0) || !!(modal.el && layer.el && modal.el.contains(layer.el))
const above = (a: Layer, b: Layer) =>
  RANK[a.kind] < RANK[b.kind] || (RANK[a.kind] === RANK[b.kind] && (a.order ?? 0) > (b.order ?? 0))

export function topLayer(): Layer | null {
  const on = layers.filter(live)
  let modal: Layer | null = null
  for (const layer of on) {
    if (layer.kind === 'dialog' && (!modal || (layer.order ?? 0) > (modal.order ?? 0))) modal = layer
  }
  const scope = modal ? on.filter((layer) => inside(modal as Layer, layer)) : on
  return scope.reduce<Layer | null>((best, layer) => (!best || above(layer, best) ? layer : best), null)
}

function onKeyDown(event: KeyboardEvent) {
  if (event.key !== 'Escape' && event.key !== 'Esc') {
    deadPending = event.key === 'Dead'
    return
  }
  const composing = event.isComposing || event.keyCode === 229 || deadPending
  deadPending = false
  if (composing) return
  const layer = topLayer()
  if (!layer) return
  event.preventDefault()
  event.stopPropagation()
  layer.escape(event)
}

function install() {
  if (installed) return
  installed = true
  window.addEventListener('keydown', onKeyDown, true)
}

/** Kaliber layer 6: a search or filter field with text clears and keeps the focus. The
 *  value is set through the native setter and an `input` event, so React's onChange sees
 *  the change. */
export function clearSearchField(active: Element | null): boolean {
  if (!(active instanceof HTMLInputElement) || active.value === '') return false
  if (active.type !== 'search' && !active.matches('[data-k-search]')) return false
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(active, '')
  active.dispatchEvent(new Event('input', { bubbles: true }))
  return true
}

/** The one search layer of the app: active while the focused field is a search field
 *  with text. */
export const searchLayer: Layer = {
  kind: 'search',
  active: () => {
    const el = document.activeElement
    return el instanceof HTMLInputElement && el.value !== '' && (el.type === 'search' || el.matches('[data-k-search]'))
  },
  escape: () => { clearSearchField(document.activeElement) },
}

/** Registers a layer while `on` is true. `escape` may change between renders. */
export function useEscapeLayer(kind: LayerKind, on: boolean, escape: () => void, el?: () => HTMLElement | null): void {
  const run = useRef(escape)
  run.current = escape
  useEffect(() => {
    if (!on) return
    const layer: Layer = { kind, el: el?.() ?? null, escape: () => run.current() }
    pushLayer(layer)
    return () => popLayer(layer)
    // `el` is read once when the layer opens.
  }, [kind, on])
}

/** Test seam: forget every layer. */
export function resetEscapeLayers(): void {
  layers.length = 0
}
