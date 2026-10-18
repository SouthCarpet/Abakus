import { useSyncExternalStore } from 'react'
import { api } from '../api'
import { readUpdatePreference } from './updatePreference'
import { setLatestRelease } from './updateStatus'

// The three update-check paths (Kaliber R20; Michal 2026-10-12): at app start, when the
// About section opens, and on the "Skontrolovať aktualizácie" button. Each runs only while the
// user has turned checking on; off means zero requests on all three. The backend gates the
// same setting again (`check_update_now_inner`). The start check is quiet: no dialog and no
// toast; it only marks a found update (header line and the About badge). A connection error
// is kept apart from "you have the latest version".

export type CheckResult =
  | { at: string; result: 'latest' }
  | { at: string; result: 'found'; tag: string }
  | { at: string; result: 'error'; detail: string }

const LAST_CHECK_KEY = 'abakus.update.lastCheck'
const listeners = new Set<() => void>()
let cachedRaw: string | null | undefined

function isCheckResult(value: unknown): value is CheckResult {
  const v = value as Partial<Record<string, unknown>> | null
  return !!v && typeof v.at === 'string' && (v.result === 'latest' || v.result === 'found' || v.result === 'error')
}

export function readLastCheck(): CheckResult | null {
  try {
    const raw = localStorage.getItem(LAST_CHECK_KEY)
    if (!raw) return null
    const parsed: unknown = JSON.parse(raw)
    return isCheckResult(parsed) ? parsed : null
  } catch { return null }
}

function saveLastCheck(result: CheckResult) {
  try { localStorage.setItem(LAST_CHECK_KEY, JSON.stringify(result)) } catch { /* shown for this run only */ }
  cachedRaw = undefined
  listeners.forEach((listen) => listen())
}

let generation = 0

/** Turning checking off drops every check still in flight: its answer is not shown. */
export function invalidateChecks(): void {
  generation += 1
}

async function runCheck(at: Date): Promise<CheckResult | null> {
  const mine = generation
  let result: CheckResult
  try {
    const release = await api.checkUpdateNow()
    if (mine !== generation) return null
    setLatestRelease(release)
    result = release ? { at: at.toISOString(), result: 'found', tag: release.tag } : { at: at.toISOString(), result: 'latest' }
  } catch (e) {
    if (mine !== generation) return null
    result = { at: at.toISOString(), result: 'error', detail: String(e) }
  }
  saveLastCheck(result)
  return result
}

/** The start path: quiet. Reads the consent itself; any failure stays in the last-check line. */
export async function checkAtStart(at: Date = new Date()): Promise<void> {
  const on = await readUpdatePreference().catch(() => false)
  if (!on) return
  await runCheck(at)
}

/** The About-open and button paths. `enabled` is the consent the screen has loaded. */
export async function checkNow(enabled: boolean, at: Date = new Date()): Promise<CheckResult | null> {
  if (!enabled) return null
  return runCheck(at)
}

function stamp(iso: string): string {
  const d = new Date(iso)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getDate()}. ${d.getMonth() + 1}. ${d.getFullYear()} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

function errorText(detail: string): string {
  const http = /update check got HTTP (\d+)/.exec(detail)
  if (http) return `GitHub odpovedal chybou (HTTP ${http[1]}).`
  if (detail.includes('update check request failed')) return 'bez spojenia so serverom GitHub. Skúste to neskôr.'
  return 'odpoveď GitHubu sa nedala prečítať.'
}

export function describeLastCheck(last: CheckResult | null): string {
  if (!last) return 'Kontrola ešte neprebehla.'
  const head = `Posledná kontrola ${stamp(last.at)}: `
  if (last.result === 'latest') return `${head}máte najnovšiu verziu.`
  if (last.result === 'found') return `${head}k dispozícii je verzia ${last.tag}.`
  return `${head}${errorText(last.detail)}`
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

let cached: CheckResult | null = null
function snapshot(): CheckResult | null {
  let raw: string | null = null
  try { raw = localStorage.getItem(LAST_CHECK_KEY) } catch { /* none */ }
  if (raw !== cachedRaw) { cachedRaw = raw; cached = readLastCheck() }
  return cached
}

export function useLastCheck(): CheckResult | null {
  return useSyncExternalStore(subscribe, snapshot)
}
