import { useEffect, useState } from 'react'

export type Theme = 'light' | 'dark'
export type ThemePreference = Theme | 'system'
const STORAGE_KEY = 'abakus.theme'

function loadPreference(): ThemePreference {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored === 'light' || stored === 'dark') return stored
  } catch { /* Inaccessible storage uses system mode. */ }
  return 'system'
}

export function useTheme() {
  const [preference, setPreference] = useState<ThemePreference>(loadPreference)
  useEffect(() => {
    const query = window.matchMedia('(prefers-color-scheme: dark)')
    const apply = (dark: boolean) => {
      document.documentElement.dataset.theme = preference === 'system' ? (dark ? 'dark' : 'light') : preference
    }
    apply(query.matches)
    const onChange = (event: MediaQueryListEvent) => apply(event.matches)
    query.addEventListener('change', onChange)
    return () => query.removeEventListener('change', onChange)
  }, [preference])
  function choose(next: ThemePreference) {
    setPreference(next)
    try { localStorage.setItem(STORAGE_KEY, next) }
    catch { /* The current session remains usable without persistence. */ }
  }
  return [preference, choose] as const
}
