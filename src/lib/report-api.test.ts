import { invoke } from '@tauri-apps/api/core'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { reportApi, type ReportRequest } from './report-api'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

beforeEach(() => vi.clearAllMocks())

describe('reportApi: native report boundary', () => {
  it('sends the literal six-month request only to the accepted preview command', async () => {
    const request: ReportRequest = {
      period: { kind: 'six_months', ending_month: '2026-09' },
      scope: { kind: 'kind', account_kind: 'personal' },
    }
    vi.mocked(invoke).mockResolvedValueOnce({ transaction_count: 8 })

    await reportApi.preview(request)

    expect(invoke).toHaveBeenCalledTimes(1)
    expect(invoke).toHaveBeenCalledWith('preview_pdf_report', { request })
  })

  it('sends the selected path and unchanged account request to the accepted export command', async () => {
    const request: ReportRequest = {
      period: { kind: 'year', year: 2024 },
      scope: { kind: 'account', account_id: 42 },
    }
    vi.mocked(invoke).mockResolvedValueOnce({ path: 'C:/reports/2024.pdf', bytes: 80, pages: 1, report: {} })

    await reportApi.export(request, 'C:/reports/2024.pdf')

    expect(invoke).toHaveBeenCalledTimes(1)
    expect(invoke).toHaveBeenCalledWith('export_pdf_report', { request, path: 'C:/reports/2024.pdf' })
  })
})

describe('reportApi: document scope', () => {
  const request: ReportRequest = { period: { kind: 'month', month: '2026-09' }, scope: { kind: 'all' } }

  it('keeps the payload exactly { request } and { request, path } when no scope is given', async () => {
    vi.mocked(invoke).mockResolvedValue({})
    await reportApi.preview(request)
    await reportApi.export(request, 'C:/r.pdf')
    expect(vi.mocked(invoke).mock.calls).toEqual([
      ['preview_pdf_report', { request }],
      ['export_pdf_report', { request, path: 'C:/r.pdf' }],
    ])
  })

  it.each(['full', 'summary_one_page', 'summary_three_pages'] as const)('sends the wire value %s to both commands', async (scope) => {
    vi.mocked(invoke).mockResolvedValue({})
    await reportApi.preview(request, scope)
    await reportApi.export(request, 'C:/r.pdf', scope)
    expect(vi.mocked(invoke).mock.calls).toEqual([
      ['preview_pdf_report', { request, scope }],
      ['export_pdf_report', { request, path: 'C:/r.pdf', scope }],
    ])
  })
})