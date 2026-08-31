import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { TransactionRow } from './Transactions'

afterEach(() => cleanup())

const row = {
  id: 7,
  account_kind: 'personal',
  tx_date: '2026-05-29',
  merchant_raw: 'ALDI SUED',
  place: 'Neuss',
  amount_cents: -199,
  category_id: 2,
  category_name: 'potraviny',
  parent_name: 'Jedlo',
  status: 'suggested',
  raw_block: 'raw',
  kind: 'card',
} as never

describe('TransactionRow', () => {
  it('shows an Odhad badge with a confirm button and calls confirm', () => {
    const onConfirm = vi.fn()
    render(
      <table>
        <tbody>
          <TransactionRow row={row} categories={[]} selected={false} onSelect={vi.fn()} onAssign={vi.fn()} onConfirm={onConfirm} />
        </tbody>
      </table>,
    )
    expect(screen.getByText('Odhad')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Potvrdiť' }))
    expect(onConfirm).toHaveBeenCalledWith(7)
  })
  it('renders the amount with tabular numerals class and negative sign', () => {
    render(
      <table>
        <tbody>
          <TransactionRow row={row} categories={[]} selected={false} onSelect={vi.fn()} onAssign={vi.fn()} onConfirm={vi.fn()} />
        </tbody>
      </table>,
    )
    expect(screen.getByText('-1,99 €')).toHaveClass('k-num')
  })
})
