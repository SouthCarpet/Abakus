import { invoke } from '@tauri-apps/api/core'
import type { AccountKind } from '../api'

export type ReportPeriod =
  | { kind: 'month'; month: string }
  | { kind: 'six_months'; ending_month: string }
  | { kind: 'year'; year: number }
  | { kind: 'all_time' }

export type ReportScope =
  | { kind: 'all' }
  | { kind: 'kind'; account_kind: AccountKind }
  | { kind: 'account'; account_id: number }

/** Wire values of `ReportDocumentScope` in the Rust commands. An absent value means `full`. */
export type ReportDocumentScope = 'full' | 'summary_one_page' | 'summary_three_pages'

export interface ReportRequest { period: ReportPeriod; scope: ReportScope }
export interface ReportDateRange { from: string; to: string }
export interface ReportAccount { id: number; label: string; kind: AccountKind; iban_suffix: string }
export interface ReportPreview {
  range: ReportDateRange | null
  scope_label: string
  accounts: ReportAccount[]
  transaction_count: number
  latest_transaction_date: string | null
  captured_at: string
  unfinished_period: boolean
  accounts_without_statements: number
  accounts_with_gaps: number
  unverified_statement_count: number
  invalid_statement_range_count: number
}
export interface PdfReportOutcome { path: string; bytes: number; pages: number; report: ReportPreview }

function withScope<T extends object>(body: T, scope?: ReportDocumentScope): T | (T & { scope: ReportDocumentScope }) {
  return scope === undefined ? body : { ...body, scope }
}

export const reportApi = {
  preview: (request: ReportRequest, scope?: ReportDocumentScope) =>
    invoke<ReportPreview>('preview_pdf_report', withScope({ request }, scope)),
  export: (request: ReportRequest, path: string, scope?: ReportDocumentScope) =>
    invoke<PdfReportOutcome>('export_pdf_report', withScope({ request, path }, scope)),
}
