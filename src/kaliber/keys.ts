// Kaliber keyboard rules (design/web/kaliber-keys.js; wiki components.md, "Keyboard"),
// ported to TypeScript the same way the ANIMUS dashboard did (src/lib/keys.ts there).
// One spec string per shortcut: '+'-joined modifiers (primary, ctrl, alt, shift, meta) and
// one key ('a', '0', ',', 'F1', 'Escape'). Letters match `key` (lower case) with a `code`
// fallback for a non-Latin layout; digits match `code`, so the Slovak top row works;
// punctuation matches `key` with Shift ignored; named keys match `key`. Composition, dead
// keys, AltGr and Ctrl+Alt never match. The primary modifier is Command on macOS and Ctrl
// elsewhere, never ctrlKey || metaKey.

export type Platform = 'mac' | 'windows' | 'linux'

interface Spec {
  key: string
  kind: 'letter' | 'digit' | 'punct' | 'named'
  primary: boolean
  ctrl: boolean
  alt: boolean
  shift: boolean
  meta: boolean
}

const NAMES: Record<string, string> = {
  esc: 'Escape', escape: 'Escape', del: 'Delete', delete: 'Delete', space: ' ', enter: 'Enter', tab: 'Tab',
  comma: ',', plus: '+', minus: '-', slash: '/', home: 'Home', end: 'End', pageup: 'PageUp', pagedown: 'PageDown',
}

export function platform(): Platform {
  const nav = window.navigator as Navigator & { userAgentData?: { platform?: string } }
  const name = nav.userAgentData?.platform || nav.platform || ''
  if (/mac|iphone|ipad|ipod/i.test(name)) return 'mac'
  return /linux|x11|cros/i.test(name) ? 'linux' : 'windows'
}

export function primary(e: KeyboardEvent, where: Platform = platform()): boolean {
  return where === 'mac' ? e.metaKey : e.ctrlKey
}

function keyOf(text: string): string {
  const lower = text.toLowerCase()
  if (lower in NAMES) return NAMES[lower]
  if (/^f\d{1,2}$/.test(lower)) return `F${lower.slice(1)}`
  return text.length === 1 ? lower : text
}

function kindOf(key: string): Spec['kind'] {
  if (/^[a-z]$/.test(key)) return 'letter'
  if (/^[0-9]$/.test(key)) return 'digit'
  return key.length === 1 && key !== ' ' ? 'punct' : 'named'
}

export function parseSpec(spec: string): Spec {
  const plusKey = spec === '+' || spec.endsWith('++')
  const body = plusKey ? spec.slice(0, -1).replace(/\+$/, '') : spec
  const parts = body ? body.split('+') : []
  const key = plusKey ? '+' : keyOf(parts.pop() ?? '')
  const mods = new Set(parts.map((p) => p.toLowerCase()))
  return {
    key,
    kind: kindOf(key),
    primary: mods.has('primary'),
    ctrl: mods.has('ctrl'),
    alt: mods.has('alt'),
    shift: mods.has('shift'),
    meta: mods.has('meta'),
  }
}

/** A press the system owns as typing: composition, a dead key, AltGr or Ctrl+Alt. */
export function ignoredKey(e: KeyboardEvent): boolean {
  if (e.isComposing || e.keyCode === 229 || e.key === 'Dead') return true
  if (typeof e.getModifierState === 'function' && e.getModifierState('AltGraph')) return true
  return e.ctrlKey && e.altKey
}

function keyMatches(e: KeyboardEvent, s: Spec): boolean {
  const key = e.key ?? ''
  if (s.kind === 'letter') {
    const lower = key.toLowerCase()
    return /^[a-z]$/.test(lower) ? lower === s.key : e.code === `Key${s.key.toUpperCase()}`
  }
  if (s.kind === 'digit') return e.code === `Digit${s.key}` || (e.code === `Numpad${s.key}` && /^[0-9]$/.test(key))
  return key === s.key
}

export function matchKey(e: KeyboardEvent, spec: string, where: Platform = platform()): boolean {
  const s = parseSpec(spec)
  if (ignoredKey(e)) return false
  const mac = where === 'mac'
  const ctrl = s.ctrl || (s.primary && !mac)
  const meta = s.meta || (s.primary && mac)
  if (!!e.ctrlKey !== ctrl || !!e.metaKey !== meta || !!e.altKey !== s.alt) return false
  if (s.kind !== 'punct' && !!e.shiftKey !== s.shift) return false
  return keyMatches(e, s)
}

const PC_NAMES: Record<string, string> = { ' ': 'Space', Escape: 'Esc', Delete: 'Del', '+': 'Plus', '-': 'Minus', ArrowDown: '↓', ArrowUp: '↑', PageDown: 'PgDn', PageUp: 'PgUp' }
const MAC_NAMES: Record<string, string> = { ' ': 'Space', Escape: '⎋', Delete: '⌦', Enter: '↩', Tab: '⇥' }

interface Mods { ctrl: boolean; alt: boolean; shift: boolean; meta: boolean }

function modsOf(s: Spec, mac: boolean): Mods {
  return {
    ctrl: s.ctrl || (s.primary && !mac),
    alt: s.alt,
    shift: s.shift && s.kind !== 'punct',
    meta: s.meta || (s.primary && mac),
  }
}

function keyText(s: Spec, names: Record<string, string>): string {
  return names[s.key] ?? (s.key.length === 1 ? s.key.toUpperCase() : s.key)
}

/** `Ctrl+Shift+S` on Windows and Linux, `⇧⌘S` on macOS (Control, Option, Shift, Command). */
export function keyLabel(spec: string, where: Platform = platform()): string {
  const s = parseSpec(spec)
  const mac = where === 'mac'
  const m = modsOf(s, mac)
  if (mac) {
    const glyphs: [boolean, string][] = [[m.ctrl, '⌃'], [m.alt, '⌥'], [m.shift, '⇧'], [m.meta, '⌘']]
    return glyphs.filter(([on]) => on).map(([, g]) => g).join('') + keyText(s, MAC_NAMES)
  }
  const words: [boolean, string][] = [[m.ctrl, 'Ctrl'], [m.alt, 'Alt'], [m.shift, 'Shift'], [m.meta, where === 'linux' ? 'Super' : 'Win']]
  return [...words.filter(([on]) => on).map(([, w]) => w), keyText(s, PC_NAMES)].join('+')
}

/** Kaliber Escape layer 6: a search or filter field with text clears and keeps the focus;
 *  the next Escape goes to the next layer. True when it cleared a field. The value is set
 *  through the native setter and an `input` event, so React's onChange sees the change. */
export function clearSearchField(active: Element | null): boolean {
  if (!(active instanceof HTMLInputElement) || active.value === '') return false
  if (active.type !== 'search' && !active.matches('[data-k-search], [cmdk-input]')) return false
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(active, '')
  active.dispatchEvent(new Event('input', { bubbles: true }))
  return true
}
