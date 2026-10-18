import { describe, expect, it, vi } from 'vitest'
import { open } from '@tauri-apps/plugin-dialog'
import { defaultBackupFilename, files } from './files'

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(), save: vi.fn() }))

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

describe('files.openBackup', () => {
  it('opens a single-file picker filtered to .db, no multi-select', () => {
    void files.openBackup()
    expect(open).toHaveBeenCalledExactlyOnceWith({ filters: [{ name: 'Databáza', extensions: ['db'] }] })
  })
})
