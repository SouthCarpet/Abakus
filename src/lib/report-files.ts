import { save } from '@tauri-apps/plugin-dialog'

function pad(value: number): string {
  return value.toString().padStart(2, '0')
}

/** A readable suggestion only. The native renderer refuses an existing file. */
export function defaultReportFilename(date: Date): string {
  return `abakus-prehlad-${date.getFullYear()}-${pad(date.getMonth() + 1)}.pdf`
}

export const reportFiles = {
  save: () => save({
    filters: [{ name: 'PDF', extensions: ['pdf'] }],
    defaultPath: defaultReportFilename(new Date()),
  }),
}
