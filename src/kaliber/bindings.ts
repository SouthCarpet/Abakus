// The one binding table of Abakus (Kaliber "Discoverability"): the help panel, the tooltips
// and `aria-keyshortcuts` read the keys from here. The set is chosen from the Kaliber keyboard
// map (a catalog, ledger:9299); docs/plan-091-kaliber-r3.md says why each key is or is not
// bound.

import { keyLabel } from './keys'

export interface Binding {
  id: string
  spec: string
  action: string
  where: string
}

export const BINDINGS: Binding[] = [
  { id: 'help', spec: 'F1', action: 'Pomocník', where: 'Všade' },
  { id: 'settings', spec: 'primary+comma', action: 'Nastavenia', where: 'Všade' },
  { id: 'import', spec: 'primary+o', action: 'Import a výber PDF', where: 'Všade' },
  { id: 'find', spec: 'primary+f', action: 'Pole Hľadať', where: 'Transakcie' },
  { id: 'undo', spec: 'primary+z', action: 'Vrátiť posledné hromadné priradenie', where: 'Transakcie' },
  { id: 'redo', spec: 'primary+y', action: 'Znova vykonať vrátené hromadné priradenie (aj Ctrl+Shift+Z)', where: 'Transakcie' },
  { id: 'delete', spec: 'Delete', action: 'Zmazať zameraný riadok: otvorí potvrdenie s počtom, bez neho nič nezmaže', where: 'Účty, výpisy, kategórie' },
  { id: 'select-all', spec: 'primary+a', action: 'Vybrať všetky viditeľné riadky, druhýkrát zrušiť výber', where: 'Tabuľka Transakcie' },
  { id: 'accept', spec: 'Enter', action: 'Prijať odhad na zameranom riadku', where: 'Tabuľka Transakcie' },
  { id: 'toggle', spec: 'space', action: 'Prepnúť výber zameraného riadku', where: 'Tabuľka Transakcie' },
  { id: 'move', spec: 'ArrowDown', action: 'Ďalší riadok (šípka hore: predchádzajúci)', where: 'Tabuľka Transakcie' },
  { id: 'extend', spec: 'shift+ArrowDown', action: 'Rozšíriť výber (aj Shift+šípka hore, Shift+klik)', where: 'Tabuľka Transakcie' },
  { id: 'first-last', spec: 'Home', action: 'Prvý riadok (End: posledný)', where: 'Tabuľka Transakcie' },
  { id: 'page', spec: 'PageDown', action: 'O 10 riadkov nižšie (PgUp: vyššie)', where: 'Tabuľka Transakcie' },
  { id: 'escape', spec: 'Escape', action: 'Zavrie jednu vrstvu: toast, zoznam, text hľadania, výber, dialóg', where: 'Všade' },
]

export function binding(id: string): Binding {
  const found = BINDINGS.find((b) => b.id === id)
  if (!found) throw new Error(`unknown binding ${id}`)
  return found
}

/** "Pomocník (F1)": the tooltip text of a control with a key. */
export function withKey(text: string, id: string): string {
  return `${text} (${keyLabel(binding(id).spec)})`
}

/** The `aria-keyshortcuts` value: "Control+O", "F1". */
export function ariaKeys(id: string): string {
  return keyLabel(binding(id).spec).replace(/^Ctrl\+/, 'Control+')
}
