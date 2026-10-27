import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { RecurringTable } from './RecurringTable'
import { recurringRow } from './oracles'

afterEach(() => cleanup())

describe('RecurringTable Akcie column layout', () => {
  it('stacks the estimate row actions in a column so the wrapped ghost label does not crowd its neighbours', () => {
    render(
      <RecurringTable
        caption="Odhady"
        rows={[recurringRow({ decision: 'estimate' })]}
        busy={false}
        onDetail={() => {}}
        onConfirm={() => {}}
        onEdit={() => {}}
        onIgnore={() => {}}
        onReset={() => {}}
      />
    )
    const ignoreButton = screen.getByRole('button', { name: 'Toto nie je pravidelná platba: Netflix' })
    expect(ignoreButton.closest('.ab-row')).toHaveClass('ab-recurring-actions')
  })

  it('stacks the confirmed row actions in a column too', () => {
    render(
      <RecurringTable
        caption="Výdavky"
        rows={[recurringRow({ decision: 'confirmed' })]}
        busy={false}
        onDetail={() => {}}
        onConfirm={() => {}}
        onEdit={() => {}}
        onIgnore={() => {}}
        onReset={() => {}}
      />
    )
    const ignoreButton = screen.getByRole('button', { name: 'Toto nie je pravidelná platba: Netflix' })
    expect(ignoreButton.closest('.ab-row')).toHaveClass('ab-recurring-actions')
  })

  // Oracle: round-3 finding 1; ignored rows must carry ab-recurring-actions
  // the same way estimate and confirmed rows already do.
  it('stacks the ignored row actions in a column too', () => {
    render(
      <RecurringTable
        caption="Ignorované"
        rows={[recurringRow({ decision: 'ignored' })]}
        busy={false}
        onDetail={() => {}}
        onConfirm={() => {}}
        onEdit={() => {}}
        onIgnore={() => {}}
        onReset={() => {}}
      />
    )
    const restoreButton = screen.getByRole('button', { name: 'Obnoviť odhad Netflix' })
    expect(restoreButton.closest('.ab-row')).toHaveClass('ab-recurring-actions')
  })
})

// Oracle: fix1 badge sweep, a missing payment is a problem, an active payment keeps its dot.
it('fix1 uses danger for missing payments and keeps the active dot', () => {
  render(<RecurringTable caption="Platby" rows={[recurringRow({ state: 'missing' }), recurringRow({ series_key: 'active', state: 'active' })]} busy={false} onDetail={() => {}} onConfirm={() => {}} onEdit={() => {}} onIgnore={() => {}} onReset={() => {}} />)
  expect(screen.getByText('Chýba platba').querySelector('use')).toHaveAttribute('href', '/kaliber-icons.svg#k-icon-danger')
  expect(screen.getByText('Aktívna').querySelector('svg')).toBeNull()
})
