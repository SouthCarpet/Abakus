import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '../../api'
import { reportApi } from '../../lib/report-api'
import { reportFiles } from '../../lib/report-files'
import { ExportPdfDialog } from './ExportPdfDialog'

vi.mock('../../api', () => ({ api: { listAccounts: vi.fn() } }))
vi.mock('../../lib/report-api', () => ({ reportApi: { preview: vi.fn(), export: vi.fn() } }))
vi.mock('../../lib/report-files', () => ({ reportFiles: { save: vi.fn() } }))

afterEach(() => cleanup())
beforeEach(() => {
  vi.clearAllMocks()
  vi.mocked(api.listAccounts).mockResolvedValue([
    { id: 11, label: 'Bežný účet', kind: 'personal', iban: 'SK001', has_password: false },
    { id: 22, label: 'Firma', kind: 'business', iban: 'SK002', has_password: false },
  ])
})

function preview(overrides: Partial<Awaited<ReturnType<typeof reportApi.preview>>> = {}) {
  return {
    range: { from: '2026-09-01', to: '2026-09-30' },
    scope_label: 'Všetky účty',
    accounts: [{ id: 11, label: 'Bežný účet', kind: 'personal' as const, iban_suffix: '0001' }],
    transaction_count: 8,
    latest_transaction_date: '2026-09-07',
    captured_at: '2026-09-07T10:30:00+02:00',
    unfinished_period: true,
    accounts_without_statements: 0,
    accounts_with_gaps: 0,
    unverified_statement_count: 0,
    invalid_statement_range_count: 0,
    ...overrides,
  }
}

async function renderReady() {
  vi.mocked(reportApi.preview).mockResolvedValue(preview())
  render(<ExportPdfDialog open onClose={() => {}} />)
  await screen.findByRole('heading', { name: 'Náhľad' })
}

describe('ExportPdfDialog: P02 exact report period', () => {
  it('keeps the selected current calendar month as a preview request and marks the unfinished report', async () => {
    await renderReady()

    expect(reportApi.preview).toHaveBeenLastCalledWith({ period: { kind: 'month', month: expect.stringMatching(/^\d{4}-\d{2}$/) }, scope: { kind: 'all' } })
    expect(screen.getByText('Obdobie ešte neskončilo.')).toBeInTheDocument()
  })

  it('uses the ending month for six consecutive months instead of an approximate day count', async () => {
    await renderReady()
    fireEvent.change(screen.getByLabelText('Obdobie'), { target: { value: 'six_months' } })
    fireEvent.change(await screen.findByLabelText('Koncový mesiac'), { target: { value: '2026-02' } })

    await vi.waitFor(() => expect(reportApi.preview).toHaveBeenLastCalledWith({ period: { kind: 'six_months', ending_month: '2026-02' }, scope: { kind: 'all' } }))
  })
})

describe('ExportPdfDialog: P03 scope does not follow table filters', () => {
  it('sends an exact selected account ID and tells the user that table filters are unused', async () => {
    await renderReady()
    fireEvent.change(screen.getByLabelText('Účty'), { target: { value: 'account:22' } })

    await vi.waitFor(() => expect(reportApi.preview).toHaveBeenLastCalledWith({ period: { kind: 'month', month: expect.any(String) }, scope: { kind: 'account', account_id: 22 } }))
    expect(await screen.findByText(/Filtre tabuľky sa nepoužijú/)).toBeInTheDocument()
  })
})

describe('ExportPdfDialog: P16 native save and stale state', () => {
  it('does nothing after cancelling the native save dialog', async () => {
    await renderReady()
    vi.mocked(reportFiles.save).mockResolvedValue(null)
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť PDF' }))

    await vi.waitFor(() => expect(reportFiles.save).toHaveBeenCalledTimes(1))
    expect(reportApi.export).not.toHaveBeenCalled()
    expect(screen.queryByText(/PDF uložené/)).not.toBeInTheDocument()
  })

  it('retains the draft and displays a native launch error', async () => {
    await renderReady()
    vi.mocked(reportFiles.save).mockRejectedValueOnce(new Error('Dialóg sa nedal otvoriť.'))
    const monthBeforeSave = (screen.getByLabelText('Mesiac') as HTMLInputElement).value
    fireEvent.click(screen.getByRole('button', { name: 'Uložiť PDF' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('Dialóg sa nedal otvoriť.')
    expect(screen.getByLabelText('Mesiac')).toHaveValue(monthBeforeSave)
  })

  it('uses the completed capture count, freezes controls, and ignores a second export click', async () => {
    await renderReady()
    let resolveExport!: (value: Awaited<ReturnType<typeof reportApi.export>>) => void
    vi.mocked(reportFiles.save).mockResolvedValue('C:/reports/september.pdf')
    vi.mocked(reportApi.export).mockReturnValue(new Promise((resolve) => { resolveExport = resolve }))
    const saveButton = screen.getByRole('button', { name: 'Uložiť PDF' })
    fireEvent.click(saveButton)
    fireEvent.click(saveButton)

    await vi.waitFor(() => expect(reportApi.export).toHaveBeenCalledTimes(1))
    expect(screen.getByLabelText('Účty')).toBeDisabled()
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(screen.getByRole('dialog')).toBeInTheDocument()
    resolveExport({ path: 'C:/reports/september.pdf', bytes: 1200, pages: 2, report: preview({ transaction_count: 9 }) })

    expect(await screen.findByText(/Transakcie zachytené pri uložení: 9/)).toBeInTheDocument()
  })

  it('does not let a late first preview replace the latest selected scope', async () => {
    let resolveFirst!: (value: Awaited<ReturnType<typeof reportApi.preview>>) => void
    vi.mocked(reportApi.preview)
      .mockReturnValueOnce(new Promise((resolve) => { resolveFirst = resolve }))
      .mockResolvedValueOnce(preview({ scope_label: 'Firemné účty', transaction_count: 3 }))
    render(<ExportPdfDialog open onClose={() => {}} />)
    await screen.findByLabelText('Účty')
    fireEvent.change(screen.getByLabelText('Účty'), { target: { value: 'business' } })

    await screen.findByText('Firemné účty (1)')
    await act(async () => {
      resolveFirst(preview({ scope_label: 'Všetky účty', transaction_count: 99 }))
      await Promise.resolve()
    })
    expect(screen.queryByText('Všetky účty (1)')).not.toBeInTheDocument()
    expect(screen.getByText('Firemné účty (1)')).toBeInTheDocument()
  })
})
