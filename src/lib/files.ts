import { save } from '@tauri-apps/plugin-dialog'

function pad(value: number, width = 2): string {
  return value.toString().padStart(width, '0')
}

/** A dated, unique-per-call default name so a backup never collides with the last one. */
export function defaultBackupFilename(date: Date): string {
  const stamp = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}-${pad(date.getHours())}${pad(date.getMinutes())}${pad(date.getSeconds())}${pad(date.getMilliseconds(), 3)}`
  return `abakus-zaloha-${stamp}.db`
}

export const files = {
  saveCsv: () => save({ filters: [{ name: 'CSV', extensions: ['csv'] }] }),
  saveBackup: () => save({ filters: [{ name: 'Databáza', extensions: ['db'] }], defaultPath: defaultBackupFilename(new Date()) }),
}
