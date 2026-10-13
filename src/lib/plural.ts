// Slovak plural forms of a counted noun: 1 takes the singular, 2 to 4 the "few" form,
// 0 and 5 or more the genitive plural. The category comes from the CLDR rule of the platform
// (Intl.PluralRules for sk). One helper for every place where a number meets a noun or a verb.

export type PluralForms = readonly [one: string, few: string, many: string]

const rules = new Intl.PluralRules('sk')

export function pluralSk(n: number, forms: PluralForms): string {
  switch (rules.select(n)) {
    case 'one': return forms[0]
    case 'few': return forms[1]
    default: return forms[2]
  }
}

export function countPhrase(n: number, forms: PluralForms): string {
  return `${n} ${pluralSk(n, forms)}`
}

/** The counted nouns of the screens. Add a noun here, never a local ternary. */
export const tx = {
  platba: ['platba', 'platby', 'platieb'],
  transakcia: ['transakcia', 'transakcie', 'transakcií'],
  kategoria: ['kategória', 'kategórie', 'kategórií'],
  pravidlo: ['pravidlo', 'pravidlá', 'pravidiel'],
  odhad: ['odhad', 'odhady', 'odhadov'],
  prevod: ['prevod', 'prevody', 'prevodov'],
  nezaradene: ['nezaradená', 'nezaradené', 'nezaradených'],
  potvrdene: ['potvrdená', 'potvrdené', 'potvrdených'],
} as const satisfies Record<string, PluralForms>

/** The verbs that agree with a counted subject or object. Same rule: add here, never inline. */
export const vb = {
  caka: ['čaká', 'čakajú', 'čaká'],
  cakaVelke: ['Čaká', 'Čakajú', 'Čaká'],
  presunie: ['sa presunie', 'sa presunú', 'sa presunie'],
  presunute: ['presunutá', 'presunuté', 'presunutých'],
  odstrani: ['Odstráni sa', 'Odstránia sa', 'Odstráni sa'],
  zmaze: ['Zmaže sa', 'Zmažú sa', 'Zmaže sa'],
} as const satisfies Record<string, PluralForms>
