import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { useState } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { Dialog } from '../components/Dialog'
import { useDeleteKey } from './deleteKey'

// A list of rows with an irreversible delete behind a named confirm, the shape of the
// accounts, statements and categories lists.
function Harness({ initial }: { initial: string[] }) {
  const [items, setItems] = useState(initial)
  const [target, setTarget] = useState<string | null>(null)
  const keys = useDeleteKey<HTMLUListElement>(items.length)
  return (
    <>
      <ul ref={keys.containerRef}>
        {items.map((item) => (
          <li key={item} {...keys.rowProps(() => setTarget(item))}>
            {item}
            <input aria-label={`Názov ${item}`} defaultValue={item} />
          </li>
        ))}
      </ul>
      <Dialog
        open={target !== null}
        title={`Natrvalo zmazať ${target}?`}
        onClose={() => setTarget(null)}
        actions={<>
          <button type="button" data-k-autofocus="" onClick={() => setTarget(null)}>Zrušiť</button>
          <button type="button" onClick={() => { keys.deleted(); setItems((all) => all.filter((x) => x !== target)); setTarget(null) }}>Natrvalo zmazať</button>
        </>}
      >
        <p>Položky: 1</p>
      </Dialog>
    </>
  )
}

const row = (name: string) => screen.getByText(name).closest('li') as HTMLElement

afterEach(() => cleanup())

// Oracle: Kaliber components.md "Selection model", confirm mode: Del opens the named confirm
// with the focus on Cancel and never deletes by itself; after a delete the focus goes to the
// next row, or the previous row when the last row went. Del in a text field edits the text.
describe('Del on a list row', () => {
  it('opens the named confirm with the focus on Cancel and deletes nothing yet', () => {
    render(<Harness initial={['A', 'B', 'C']} />)
    row('B').focus()
    fireEvent.keyDown(row('B'), { key: 'Delete' })
    expect(screen.getByRole('dialog', { name: 'Natrvalo zmazať B?' })).toBeInTheDocument()
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Zrušiť' }))
    expect(screen.getByText('B')).toBeInTheDocument()
  })

  it('moves the focus to the next row after the confirmed delete', () => {
    render(<Harness initial={['A', 'B', 'C']} />)
    fireEvent.keyDown(row('B'), { key: 'Delete' })
    fireEvent.click(screen.getByRole('button', { name: 'Natrvalo zmazať' }))
    expect(screen.queryByText('B')).not.toBeInTheDocument()
    expect(document.activeElement).toBe(row('C'))
  })

  it('moves the focus to the previous row when the last row went', () => {
    render(<Harness initial={['A', 'B', 'C']} />)
    fireEvent.keyDown(row('C'), { key: 'Delete' })
    fireEvent.click(screen.getByRole('button', { name: 'Natrvalo zmazať' }))
    expect(document.activeElement).toBe(row('B'))
  })

  it('leaves Del inside a text field to the text', () => {
    render(<Harness initial={['A', 'B']} />)
    fireEvent.keyDown(screen.getByRole('textbox', { name: 'Názov A' }), { key: 'Delete' })
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })
})
