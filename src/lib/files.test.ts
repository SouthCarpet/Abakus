import { describe, expect, it } from 'vitest'
import { defaultBackupFilename } from './files'

describe('defaultBackupFilename', () => {
  it('names the file with a zero-padded, second-precision, dated stamp', () => {
    expect(defaultBackupFilename(new Date(2026, 8, 7, 9, 4, 22, 5))).toBe('abakus-zaloha-2026-09-07-090422005.db')
  })

  it('gives two calls a second apart different names, so a real second click never collides', () => {
    const first = defaultBackupFilename(new Date(2026, 8, 7, 9, 4, 22, 0))
    const second = defaultBackupFilename(new Date(2026, 8, 7, 9, 4, 23, 0))
    expect(first).not.toBe(second)
  })
})
