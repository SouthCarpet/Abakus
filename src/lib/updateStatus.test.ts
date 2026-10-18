import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import type { Release } from '../api'
import { getLatestRelease, setLatestRelease, useLatestRelease } from './updateStatus'

const release: Release = {
  tag: '0.2.0',
  url: 'https://github.com/SouthCarpet/Abakus/releases/tag/0.2.0',
  notes: '',
  installer_url: null,
  installer_name: null,
  installer_size: null,
  checksums_url: null,
}

afterEach(() => setLatestRelease(null))

describe('updateStatus: shared last-known release', () => {
  it('starts empty: no update ever found in this app run', () => {
    expect(getLatestRelease()).toBeNull()
  })

  it('stores whatever Settings already found, readable from anywhere', () => {
    setLatestRelease(release)
    expect(getLatestRelease()).toEqual(release)
  })

  it('clears back to empty when Settings reports no release', () => {
    setLatestRelease(release)
    setLatestRelease(null)
    expect(getLatestRelease()).toBeNull()
  })

  it('useLatestRelease starts at the current value and updates on change', () => {
    const { result } = renderHook(() => useLatestRelease())
    expect(result.current).toBeNull()
    act(() => setLatestRelease(release))
    expect(result.current).toEqual(release)
  })
})
