import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Release } from '../api'

const { checkUpdateNow, getCheckUpdates } = vi.hoisted(() => ({ checkUpdateNow: vi.fn(), getCheckUpdates: vi.fn() }))
vi.mock('../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../api')>()
  return { ...actual, api: { ...actual.api, checkUpdateNow, getCheckUpdates } }
})

import { checkAtStart, checkNow, describeLastCheck, readLastCheck } from './updateCheck'
import { getLatestRelease, setLatestRelease } from './updateStatus'

const release: Release = {
  tag: 'v0.2.1', url: 'https://github.com/SouthCarpet/Abakus/releases/tag/v0.2.1', notes: '',
  installer_url: null, installer_name: null, installer_size: null, checksums_url: null,
}
const at = new Date(2026, 9, 12, 14, 5)

beforeEach(() => { localStorage.clear(); checkUpdateNow.mockReset(); getCheckUpdates.mockReset() })
afterEach(() => setLatestRelease(null))

// Oracle: Michal 2026-10-12 (coordinator relay): with checking off, zero requests on all three
// paths; with checking on, check at start (quiet), on About open and on the button; a
// connection error is not "you have the latest version".
describe('update check paths', () => {
  it('makes no request at start, on About open or on the button while checking is off', async () => {
    getCheckUpdates.mockResolvedValue(false)
    await checkAtStart(at)
    expect(await checkNow(false, at)).toBeNull()
    expect(checkUpdateNow).not.toHaveBeenCalled()
    expect(readLastCheck()).toBeNull()
  })

  it('checks exactly once at start while checking is on and marks a found update', async () => {
    getCheckUpdates.mockResolvedValue(true)
    checkUpdateNow.mockResolvedValue(release)
    await checkAtStart(at)
    expect(checkUpdateNow).toHaveBeenCalledTimes(1)
    expect(getLatestRelease()).toEqual(release)
    expect(readLastCheck()).toEqual({ at: at.toISOString(), result: 'found', tag: 'v0.2.1' })
  })

  it('stays silent when the start check is offline, and records the error', async () => {
    getCheckUpdates.mockResolvedValue(true)
    checkUpdateNow.mockRejectedValue('update check request failed: offline')
    await expect(checkAtStart(at)).resolves.toBeUndefined()
    expect(getLatestRelease()).toBeNull()
    expect(readLastCheck()).toEqual({ at: at.toISOString(), result: 'error', detail: 'update check request failed: offline' })
  })

  it('tells a connection error from the latest version on the button path', async () => {
    checkUpdateNow.mockResolvedValueOnce(null).mockRejectedValueOnce('update check request failed: dns')
    expect(await checkNow(true, at)).toEqual({ at: at.toISOString(), result: 'latest' })
    expect(describeLastCheck(readLastCheck())).toBe('Posledná kontrola 12. 10. 2026 14:05: máte najnovšiu verziu.')
    expect(await checkNow(true, at)).toEqual({ at: at.toISOString(), result: 'error', detail: 'update check request failed: dns' })
    expect(describeLastCheck(readLastCheck())).toBe('Posledná kontrola 12. 10. 2026 14:05: bez spojenia so serverom GitHub. Skúste to neskôr.')
  })

  it('names an HTTP error and a found version', () => {
    expect(describeLastCheck({ at: at.toISOString(), result: 'error', detail: 'update check got HTTP 403' }))
      .toBe('Posledná kontrola 12. 10. 2026 14:05: GitHub odpovedal chybou (HTTP 403).')
    expect(describeLastCheck({ at: at.toISOString(), result: 'found', tag: 'v0.2.1' }))
      .toBe('Posledná kontrola 12. 10. 2026 14:05: k dispozícii je verzia v0.2.1.')
    expect(describeLastCheck(null)).toBe('Kontrola ešte neprebehla.')
  })
})
