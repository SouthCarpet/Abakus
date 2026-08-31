import type { Checksum, ImportStatus } from '../api'
export { formatEur } from '../api'
import { formatEur } from '../api'
export function formatDate(iso: string): string { const [y, m, d] = iso.split('-').map(Number); return `${d}. ${m}. ${y}` }
export function checksumLabel(c: Checksum | null): string {
  if (!c) return ''
  if (c.status === 'ok') return 'Kontrolný súčet sedí'
  if (c.status === 'off_by') return `Kontrolný súčet nesedí o ${formatEur(Math.abs(c.off_by))}`
  return 'Kontrolný súčet sa nedá overiť'
}
const IMPORT_LABELS: Record<ImportStatus, string> = { imported: 'Importované', already_imported: 'Už importované', locked: 'Zamknuté PDF', unknown_account: 'Neznámy účet', error: 'Chyba' }
export function importStatusLabel(s: ImportStatus): string { return IMPORT_LABELS[s] }
