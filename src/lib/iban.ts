// Mirrors parser::iban::from_sk_parts (crates/parser/src/iban.rs) so the
// "Pridať účet" dialog can accept a bank-code/account-number pair and
// convert it to an IBAN before it ever reaches the Rust command.
function mod97(digits: string): number {
  let acc = 0
  for (const ch of digits) acc = (acc * 10 + Number(ch)) % 97
  return acc
}

function toDigits(s: string): string {
  return [...s].map((c) => (c >= '0' && c <= '9' ? c : String(c.charCodeAt(0) - 55))).join('')
}

export function fromSkParts(bank: string, prefix: string, number: string): string {
  const bban = `${bank.padStart(4, '0')}${prefix.padStart(6, '0')}${number.padStart(10, '0')}`
  const check = 98 - mod97(toDigits(`${bban}SK00`))
  return `SK${check.toString().padStart(2, '0')}${bban}`
}

export function maskIban(iban: string): string {
  if (iban.length < 8) return iban
  return `${iban.slice(0, 4)}...${iban.slice(-4)}`
}
