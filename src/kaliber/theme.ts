// The one saved theme every Kaliber app shares (spec R20): `localStorage["kaliber-theme"]`
// holds `light` or `dark`, and an absent key means System. Abakus used `abakus.theme` before
// 0.2.0; migrateThemeKey() moves that saved choice over once, so the user's choice never
// changes. Two controls read and write this one state: the sun and moon switch in the header
// tools and the Light, Dark, System control in Settings > Appearance. The first paint is set
// by public/kaliber-theme-boot.js (the CSP allows no inline script).
//
// Motion (Kaliber S1): a user click switches the theme in one cross-fade of the whole page
// (View Transitions); a change of the system setting in System mode switches at once.

import { useEffect, useSyncExternalStore } from 'react'

export type ThemePreference = 'light' | 'dark' | 'system'
export type Theme = 'light' | 'dark'

export const THEME_KEY = 'kaliber-theme'
/** The Abakus key before 0.2.0. Read once by the migration. */
export const LEGACY_THEME_KEY = 'abakus.theme'

type ThemeStorage = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>

const isSaved = (value: string | null): value is Theme => value === 'light' || value === 'dark'

function storage(): ThemeStorage | null {
  try { return window.localStorage } catch { return null }
}

/** Copies a legacy choice to the shared key when the shared key is absent, then removes the
 *  legacy key. `system` needs no copy. A shared key that is present always wins. */
export function migrateThemeKey(store: ThemeStorage): void {
  const legacy = store.getItem(LEGACY_THEME_KEY)
  if (legacy === null) return
  if (store.getItem(THEME_KEY) === null && isSaved(legacy)) store.setItem(THEME_KEY, legacy)
  store.removeItem(LEGACY_THEME_KEY)
}

export function readTheme(store: ThemeStorage | null): ThemePreference {
  try {
    const saved = store?.getItem(THEME_KEY) ?? null
    return isSaved(saved) ? saved : 'system'
  } catch { return 'system' }
}

export function writeTheme(store: ThemeStorage | null, theme: ThemePreference): void {
  try {
    if (theme === 'system') store?.removeItem(THEME_KEY)
    else store?.setItem(THEME_KEY, theme)
  } catch { /* The session stays usable without persistence. */ }
}

export function systemTheme(): Theme {
  return typeof window.matchMedia === 'function' && window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

export function prefersReducedMotion(): boolean {
  return typeof window.matchMedia === 'function' && window.matchMedia('(prefers-reduced-motion: reduce)').matches
}

export function resolveTheme(preference: ThemePreference): Theme {
  return preference === 'system' ? systemTheme() : preference
}

export function applyTheme(theme: Theme): void {
  document.documentElement.dataset.theme = theme
}

/** Runs `commit` inside one cross-fade where the View Transitions API exists and reduced
 *  motion is off. `k-theme-flip` turns the per-element tone transitions off for the one style
 *  recalculation, so only the cross-fade and the parts of the switch move (kaliber.css). */
export function runThemeSwitch(commit: () => void): void {
  const root = document.documentElement
  const flip = () => {
    root.classList.add('k-theme-flip')
    commit()
    void root.offsetWidth
    root.classList.remove('k-theme-flip')
  }
  const doc = document as Document & { startViewTransition?: (callback: () => void) => unknown }
  if (typeof doc.startViewTransition === 'function' && !prefersReducedMotion()) doc.startViewTransition(flip)
  else flip()
}

let preference: ThemePreference | null = null
const listeners = new Set<() => void>()

function current(): ThemePreference {
  if (preference === null) {
    const store = storage()
    if (store) {
      try { migrateThemeKey(store) } catch { /* storage blocked: System */ }
    }
    preference = readTheme(store)
  }
  return preference
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

/** Saves the choice and switches the page. A click is the only caller, so it cross-fades. */
export function setThemePreference(next: ThemePreference): void {
  const before = resolveTheme(current())
  writeTheme(storage(), next)
  preference = next
  const after = resolveTheme(next)
  const notify = () => { applyTheme(after); listeners.forEach((listen) => listen()) }
  if (before === after) notify()
  else runThemeSwitch(notify)
}

/** Test seam: forget the cached preference. */
export function resetThemeState(): void {
  preference = null
  listeners.clear()
}

/** The saved preference and the theme in use. Follows the system setting at once (no motion)
 *  while the preference is System. */
export function useTheme(): { preference: ThemePreference; theme: Theme; choose: (next: ThemePreference) => void } {
  const pref = useSyncExternalStore(subscribe, current)
  const theme = useSyncExternalStore(subscribe, () => resolveTheme(current()))
  useEffect(() => {
    applyTheme(resolveTheme(pref))
    if (pref !== 'system' || typeof window.matchMedia !== 'function') return
    const query = window.matchMedia('(prefers-color-scheme: dark)')
    const onChange = (event: MediaQueryListEvent) => {
      if (current() !== 'system') return
      applyTheme(event.matches ? 'dark' : 'light')
      listeners.forEach((listen) => listen())
    }
    query.addEventListener('change', onChange)
    return () => query.removeEventListener('change', onChange)
  }, [pref])
  return { preference: pref, theme, choose: setThemePreference }
}
