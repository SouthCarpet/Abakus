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

export const reportApi = {
  preview: (request: ReportRequest) => invoke<ReportPreview>('preview_pdf_report', { request }),
  export: (request: ReportRequest, path: string) => invoke<PdfReportOutcome>('export_pdf_report', { request, path }),
}
