import { describe, expect, it } from 'vitest'
import { countPhrase, pluralSk, tx } from './plural'

// Slovak: 1 takes the singular, 2 to 4 the "few" form, 0 and 5 or more the genitive plural.
describe('pluralSk', () => {
  const forms = ['platba', 'platby', 'platieb'] as const
  it.each([
    [0, 'platieb'],
    [1, 'platba'],
    [2, 'platby'],
    [3, 'platby'],
    [4, 'platby'],
    [5, 'platieb'],
    [11, 'platieb'],
    [12, 'platieb'],
    [14, 'platieb'],
    [21, 'platieb'],
    [22, 'platieb'],
    [100, 'platieb'],
  ])('%i takes %s', (n, word) => {
    expect(pluralSk(n, forms)).toBe(word)
  })
})

describe('countPhrase and the counted nouns of the app', () => {
  it('writes the number and the right form', () => {
    expect(countPhrase(1, tx.platba)).toBe('1 platba')
    expect(countPhrase(2, tx.platba)).toBe('2 platby')
    expect(countPhrase(15, tx.platba)).toBe('15 platieb')
  })
  it('covers transakcia, odhad, prevod and the participle forms', () => {
    expect(countPhrase(1, tx.transakcia)).toBe('1 transakcia')
    expect(countPhrase(3, tx.transakcia)).toBe('3 transakcie')
    expect(countPhrase(8, tx.transakcia)).toBe('8 transakcií')
    expect(countPhrase(1, tx.odhad)).toBe('1 odhad')
    expect(countPhrase(2, tx.odhad)).toBe('2 odhady')
    expect(countPhrase(7, tx.odhad)).toBe('7 odhadov')
    expect(countPhrase(1, tx.prevod)).toBe('1 prevod')
    expect(countPhrase(2, tx.prevod)).toBe('2 prevody')
    expect(countPhrase(5, tx.prevod)).toBe('5 prevodov')
    expect(countPhrase(1, tx.nezaradene)).toBe('1 nezaradená')
    expect(countPhrase(2, tx.nezaradene)).toBe('2 nezaradené')
    expect(countPhrase(5, tx.nezaradene)).toBe('5 nezaradených')
    expect(countPhrase(1, tx.potvrdene)).toBe('1 potvrdená')
    expect(countPhrase(4, tx.potvrdene)).toBe('4 potvrdené')
    expect(countPhrase(6, tx.potvrdene)).toBe('6 potvrdených')
  })
})
