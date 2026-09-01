export type Theme = 'light' | 'dark'

function themeFor(matches: boolean): Theme {
  return matches ? 'dark' : 'light'
}

function applyTheme(theme: Theme): void {
  document.documentElement.dataset.theme = theme
}

/**
 * Sets `data-theme` on the document root from the OS colour scheme and keeps
 * it in step with `prefers-color-scheme` changes while the app runs. No
 * in-app toggle: the user asked for a system-following app, not a settings
 * picker. Returns the cleanup that removes the change listener.
 */
export function watchSystemTheme(): () => void {
  const query = window.matchMedia('(prefers-color-scheme: dark)')
  applyTheme(themeFor(query.matches))
  const onChange = (event: MediaQueryListEvent) => applyTheme(themeFor(event.matches))
  query.addEventListener('change', onChange)
  return () => query.removeEventListener('change', onChange)
}
