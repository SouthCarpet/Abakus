import { save } from '@tauri-apps/plugin-dialog'
import type { ReportDocumentScope } from './report-api'

function pad(value: number): string {
  return value.toString().padStart(2, '0')
}

const SCOPE_SUFFIX: Record<ReportDocumentScope, string> = {
  full: '',
  summary_one_page: '-sumar-1',
  summary_three_pages: '-sumar-3',
}

/** A readable suggestion only. The native renderer refuses an existing file. A summary gets a suffix so it differs from the full report. */
export function defaultReportFilename(date: Date, scope: ReportDocumentScope = 'full'): string {
  return `abakus-prehlad-${date.getFullYear()}-${pad(date.getMonth() + 1)}${SCOPE_SUFFIX[scope]}.pdf`
}

export const reportFiles = {
  save: (scope: ReportDocumentScope = 'full') => save({
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
    defaultPath: defaultReportFilename(new Date(), scope),
  }),
}
