import { save } from '@tauri-apps/plugin-dialog'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defaultReportFilename, reportFiles } from './report-files'

vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }))

beforeEach(() => vi.clearAllMocks())

describe('reportFiles: save destination', () => {
  it('suggests a calendar-month PDF filename for the user-selected native destination', () => {
    expect(defaultReportFilename(new Date(2026, 8, 7))).toBe('abakus-prehlad-2026-09.pdf')
  })

  it('opens the native PDF save dialog without manufacturing a browser file', async () => {
    vi.mocked(save).mockResolvedValue(null)

    await reportFiles.save()

    expect(save).toHaveBeenCalledTimes(1)
    expect(save).toHaveBeenCalledWith({ filters: [{ name: 'PDF', extensions: ['pdf'] }], defaultPath: expect.stringMatching(/^abakus-prehlad-\d{4}-\d{2}\.pdf$/) })
  })

  it('tells a summary apart from the full report by a suffix', () => {
    const date = new Date(2026, 8, 7)
    expect(defaultReportFilename(date, 'full')).toBe('abakus-prehlad-2026-09.pdf')
    expect(defaultReportFilename(date, 'summary_one_page')).toBe('abakus-prehlad-2026-09-sumar-1.pdf')
    expect(defaultReportFilename(date, 'summary_three_pages')).toBe('abakus-prehlad-2026-09-sumar-3.pdf')
  })

  it('suggests the summary file name in the native save dialog', async () => {
    vi.mocked(save).mockResolvedValue(null)

    await reportFiles.save('summary_three_pages')

    expect(save).toHaveBeenCalledWith({ filters: [{ name: 'PDF', extensions: ['pdf'] }], defaultPath: expect.stringMatching(/^abakus-prehlad-\d{4}-\d{2}-sumar-3\.pdf$/) })
  })
})
