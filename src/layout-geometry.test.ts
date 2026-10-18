import { execFileSync } from 'node:child_process'
import { describe, expect, it } from 'vitest'

// Real geometry of the Transakcie screen (plan 091 r4, fix round a4). jsdom has no layout, so
// scripts/layout-probe.mjs opens the Vite dev server in Chromium and measures. It reports
// "skipped" when no Playwright or Chromium is at hand (CI has none); this machine has both.
interface Probe {
  skipped?: string
  firstRowTop: Record<string, number>
  overflowX: Record<string, number>
  emptyTop: number
  search: Record<string, { need: number; room: number }>
  hit: {
    rowCheckbox: { width: number; height: number }
    transferCheckbox: { width: number; height: number }
    transferDisabled: boolean
    similar: { width: number; height: number }
  }
}

function runProbe(): Probe {
  const out = execFileSync(process.execPath, ['scripts/layout-probe.mjs'], { encoding: 'utf8', timeout: 290_000 })
  return JSON.parse(out.trim().split('\n').pop() ?? '{}') as Probe
}

const probe = runProbe()
const run = probe.skipped ? it.skip : it

// Limits are declared from the r4 gallery, not from the result: its register rows start at about
// y503 at 1440 px, and this screen has one more line (the waiting row), so 520. At 1024 px the
// filter fields wrap once more, so 560. Baseline before the fix: 601 and 720.
const LIMIT_TOP = { 1440: 520, 1024: 560 }
const TOUCH = 44

describe('Transakcie geometry in a real browser', () => {
  run('starts the register high enough at 1440 px', () => {
    expect(probe.firstRowTop['1440']).toBeLessThanOrEqual(LIMIT_TOP[1440])
  })
  run('starts the register high enough at 1024 px', () => {
    expect(probe.firstRowTop['1024']).toBeLessThanOrEqual(LIMIT_TOP[1024])
  })
  run('shows the whole placeholder of the search field at 1440 and 1024 px', () => {
    for (const width of ['1440', '1024']) expect(probe.search[width].room, `room at ${width}`).toBeGreaterThanOrEqual(probe.search[width].need)
  })
  run('keeps the page free of sideways scroll, also with a long category name and an empty register', () => {
    expect(probe.overflowX).toEqual({ '390': 0, '1024': 0, '1440': 0, long: 0, empty: 0 })
    expect(probe.emptyTop).toBeLessThanOrEqual(LIMIT_TOP[1440])
  })
  run('gives the row checkbox and the confirm-similar control a 44 px click area on a phone', () => {
    for (const box of [probe.hit.rowCheckbox, probe.hit.similar]) {
      expect(box.width).toBeGreaterThanOrEqual(TOUCH)
      expect(box.height).toBeGreaterThanOrEqual(TOUCH)
    }
  })
  run('keeps the same click area on the disabled checkbox of a transfer', () => {
    expect(probe.hit.transferDisabled).toBe(true)
    expect(probe.hit.transferCheckbox.width).toBeGreaterThanOrEqual(TOUCH)
    expect(probe.hit.transferCheckbox.height).toBeGreaterThanOrEqual(TOUCH)
  })
})
