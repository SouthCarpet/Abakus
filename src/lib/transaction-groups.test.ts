import { describe, expect, it } from 'vitest'
import type { TxRow } from '../api'
import { groupUnassignedTransactions } from './transaction-groups'

function tx(id: number, merchant: string, place: string | null, status: TxRow['status'] = 'unassigned'): TxRow {
  return {
    id,
    account_id: 1,
    account_kind: 'personal',
    statement_number: 1,
    posted_date: '2026-09-01',
    tx_date: '2026-09-01',
    kind: 'card',
    amount_cents: -100,
    orig_amount_cents: null,
    orig_currency: null,
    merchant_raw: merchant,
    place,
    counterparty_name: null,
    counterparty_iban: null,
    category_id: null,
    category_name: null,
    parent_name: null,
    status,
    source: 'synthetic-test',
    raw_block: '',
    note: '',
  }
}

describe('groupUnassignedTransactions', () => {
  it('groups source-derived Slovak fold cases and preserves first-seen display and row order', () => {
    // Oracle: parser::fold removes decomposed marks, lowercases, and collapses whitespace.
    const rows = [
      tx(11, '  ŽLTÝ   Obchod ', 'BRATISLAVA'),
      tx(12, 'žltý\tobchod', ' bratislava '),
      tx(13, 'Iný obchod', null),
      tx(14, 'žltý obchod', 'Bratislava'),
    ]

    expect(groupUnassignedTransactions(rows)).toEqual([
      { merchant: '  ŽLTÝ   Obchod ', place: 'BRATISLAVA', count: 3, ids: [11, 12, 14] },
      { merchant: 'Iný obchod', place: null, count: 1, ids: [13] },
    ])
  })

  it('keeps punctuation, identifier tokens, and missing versus named places distinct', () => {
    // Oracle: parser::fold does not remove punctuation or synthesize a place for null.
    const rows = [
      tx(21, 'Shop c123', null),
      tx(22, 'shop c124', null),
      tx(23, 'SHOP c123', 'Košice'),
      tx(24, 'shop c123', null),
    ]

    expect(groupUnassignedTransactions(rows)).toEqual([
      { merchant: 'Shop c123', place: null, count: 2, ids: [21, 24] },
      { merchant: 'shop c124', place: null, count: 1, ids: [22] },
      { merchant: 'SHOP c123', place: 'Košice', count: 1, ids: [23] },
    ])
  })

  it('filters non-unassigned statuses and excludes every occurrence of a duplicate ID', () => {
    // Oracle: approved point 2 includes only unassigned rows with unambiguous explicit IDs.
    const rows = [
      tx(31, 'Alpha', null),
      tx(32, 'Beta', null, 'suggested'),
      tx(33, 'Gamma', null, 'confirmed'),
      tx(34, 'Delta', null, 'transfer'),
      tx(31, 'Alpha', null),
      tx(35, 'Epsilon', null),
    ]

    expect(groupUnassignedTransactions(rows)).toEqual([
      { merchant: 'Epsilon', place: null, count: 1, ids: [35] },
    ])
  })

  it('keeps each empty folded merchant as a separate row without mutating the caller input', () => {
    // Oracle: unknown merchant labels must never collapse into a synthetic shared group.
    const rows = [tx(41, '', null), tx(42, ' \t\n ', 'Nitra'), tx(43, '\u0301', null)]
    const original = structuredClone(rows)

    expect(groupUnassignedTransactions(rows)).toEqual([
      { merchant: '', place: null, count: 1, ids: [41] },
      { merchant: ' \t\n ', place: 'Nitra', count: 1, ids: [42] },
      { merchant: '\u0301', place: null, count: 1, ids: [43] },
    ])
    expect(rows).toEqual(original)
  })

  it('returns no IDs that were not explicitly supplied', () => {
    // Oracle: selection is limited to visible input and performs no backend expansion.
    expect(groupUnassignedTransactions([tx(51, 'Visible', 'Žilina')])).toEqual([
      { merchant: 'Visible', place: 'Žilina', count: 1, ids: [51] },
    ])
  })
})
