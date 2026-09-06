import { save } from '@tauri-apps/plugin-dialog'

export const files = {
  saveCsv: () => save({ filters: [{ name: 'CSV', extensions: ['csv'] }] }),
}
