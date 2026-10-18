// Entry ink (Kaliber register motion F2, spec R17 and R18; port of the dashboard's lib/ink.ts):
// a register row that a write changes (its category or its state) takes the accent soft tone
// at once, holds it for `ink_hold`, then dries back (kaliber.css `.k-ink`, `.k-ink.is-inked`).
// The many-changes cap: above `ink_cap_per_second` changes in one view in the last second,
// nothing gets ink and the footer says what changed. Numbers never roll or count.

/** `--motion-ink-hold` (design tokens, base.json `motion.ink_hold`). */
export const INK_HOLD_MS = 1600
/** `--motion-ink-cap-per-second` (base.json `motion.ink_cap_per_second`). */
export const INK_CAP_PER_SECOND = 6
const WINDOW_MS = 1000

/** Keys that are new or whose signature changed. `prev === null` is the first load: nothing
 *  moves at page load. */
export function changedKeys<K>(prev: ReadonlyMap<K, string> | null, next: ReadonlyMap<K, string>): K[] {
  if (prev === null) return []
  const out: K[] = []
  for (const [key, signature] of next) {
    const before = prev.get(key)
    if (before !== undefined && before !== signature) out.push(key)
  }
  return out
}

/** Counts the ink changes of one view in the last second. */
export class InkCap {
  private stamps: number[] = []

  /** Records `changes` at `now`; true while the view stays at or under the cap. */
  admit(changes: number, now: number): boolean {
    this.stamps = this.stamps.filter((t) => now - t < WINDOW_MS)
    for (let i = 0; i < changes; i++) this.stamps.push(now)
    return this.stamps.length <= INK_CAP_PER_SECOND
  }
}

/** "Zmenené riadky bez zvýraznenia: 9 (ALZA, PENNY, LIDL, OMV a ďalšie 5)." */
export function capNote(labels: string[]): string {
  const shown = labels.slice(0, 4).join(', ')
  const more = labels.length > 4 ? ` a ďalšie ${labels.length - 4}` : ''
  return `Zmenené riadky bez zvýraznenia: ${labels.length} (${shown}${more}).`
}
