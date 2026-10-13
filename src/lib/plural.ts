// Slovak plural forms of a counted noun: 1 takes the singular, 2 to 4 the "few" form,
// 0 and 5 or more the genitive plural. One helper for every place where a number meets a noun.

export type PluralForms = readonly [one: string, few: string, many: string]

export function pluralSk(n: number, forms: PluralForms): string {
  if (n === 1) return forms[0]
  if (n >= 2 && n <= 4) return forms[1]
  return forms[2]
}

export function countPhrase(n: number, forms: PluralForms): string {
  return `${n} ${pluralSk(n, forms)}`
}

/** The counted nouns of the screens. Add a noun here, never a local ternary. */
export const tx = {
  platba: ['platba', 'platby', 'platieb'],
  transakcia: ['transakcia', 'transakcie', 'transakcií'],
  kategoria: ['kategória', 'kategórie', 'kategórií'],
  odhad: ['odhad', 'odhady', 'odhadov'],
  prevod: ['prevod', 'prevody', 'prevodov'],
  nezaradene: ['nezaradená', 'nezaradené', 'nezaradených'],
  potvrdene: ['potvrdená', 'potvrdené', 'potvrdených'],
} as const satisfies Record<string, PluralForms>
