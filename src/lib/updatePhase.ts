// The three button labels the "Aktualizovať na <tag>" click cycles through
// (spec 0.1.4, Michal 2026-09-08). Kept as plain data, separate from
// Settings.tsx, so the exact Slovak wording is checkable on its own without
// mounting the whole screen or timing a transient render.
export type UpdatePhase = 'idle' | 'downloading' | 'verifying' | 'launching' | 'error'

export const UPDATE_PHASE_LABEL: Record<'downloading' | 'verifying' | 'launching', string> = {
  downloading: 'Sťahujem inštalátor…',
  verifying: 'Overujem podpis súboru…',
  launching: 'Spúšťam inštalátor…',
}

// What the "Aktualizovať na <tag>" button shows for the current phase: one
// of the three labels above while a download/verify/launch step is running,
// or the click prompt itself the rest of the time.
export function updateButtonLabel(phase: UpdatePhase, tag: string): string {
  if (phase === 'downloading' || phase === 'verifying' || phase === 'launching') return UPDATE_PHASE_LABEL[phase]
  return `Aktualizovať na ${tag}`
}
