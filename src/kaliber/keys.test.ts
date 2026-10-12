import { describe, expect, it } from 'vitest'
import { keyLabel, matchKey } from './keys'

const key = (init: KeyboardEventInit & { code?: string }) => new KeyboardEvent('keydown', init)

// Oracle: Kaliber components.md "Matching rules" and "Discoverability" (labels per platform).
describe('Kaliber keys', () => {
  it('labels per platform', () => {
    expect(keyLabel('primary+shift+s', 'windows')).toBe('Ctrl+Shift+S')
    expect(keyLabel('primary+shift+s', 'mac')).toBe('⇧⌘S')
    expect(keyLabel('primary+comma', 'windows')).toBe('Ctrl+,')
    expect(keyLabel('F1', 'windows')).toBe('F1')
    expect(keyLabel('PageDown', 'windows')).toBe('PgDn')
    expect(keyLabel('Escape', 'linux')).toBe('Esc')
  })

  it('matches letters by key on QWERTZ, digits by code, and never AltGr or Ctrl+Alt', () => {
    expect(matchKey(key({ key: 'z', code: 'KeyY', ctrlKey: true }), 'primary+z', 'windows')).toBe(true)
    expect(matchKey(key({ key: 'z', code: 'KeyY', metaKey: true }), 'primary+z', 'windows')).toBe(false)
    expect(matchKey(key({ key: 'z', ctrlKey: true, altKey: true }), 'primary+z', 'windows')).toBe(false)
    expect(matchKey(key({ key: 'é', code: 'Digit0', ctrlKey: true }), 'primary+0', 'windows')).toBe(true)
    expect(matchKey(key({ key: ',', ctrlKey: true }), 'primary+comma', 'windows')).toBe(true)
    expect(matchKey(key({ key: 'o', metaKey: true }), 'primary+o', 'mac')).toBe(true)
  })
})
