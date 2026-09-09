import { useState } from 'react'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { PasswordInput } from './PasswordInput'

afterEach(() => cleanup())

// PasswordInput is controlled: a small stateful wrapper stands in for the
// caller (`SetPasswordDialog`, `Import`'s locked-file prompt) so typing and
// re-renders behave the same way they do inside those dialogs.
function Wrapper({ initial = '' }: { initial?: string }) {
  const [value, setValue] = useState(initial)
  return <PasswordInput value={value} onChange={setValue} />
}

describe('PasswordInput', () => {
  it('starts hidden', () => {
    render(<Wrapper />)
    expect(screen.getByRole('button', { name: 'Zobraziť' })).toBeInTheDocument()
    expect((document.querySelector('input') as HTMLInputElement).type).toBe('password')
  })

  it('toggles to text and back on click', () => {
    render(<Wrapper />)
    const toggle = screen.getByRole('button', { name: 'Zobraziť' })
    fireEvent.click(toggle)
    expect((document.querySelector('input') as HTMLInputElement).type).toBe('text')
    expect(screen.getByRole('button', { name: 'Skryť' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Skryť' }))
    expect((document.querySelector('input') as HTMLInputElement).type).toBe('password')
    expect(screen.getByRole('button', { name: 'Zobraziť' })).toBeInTheDocument()
  })

  it('reflects shown state in aria-pressed', () => {
    render(<Wrapper />)
    const toggle = screen.getByRole('button', { name: 'Zobraziť' })
    expect(toggle).toHaveAttribute('aria-pressed', 'false')
    fireEvent.click(toggle)
    expect(screen.getByRole('button', { name: 'Skryť' })).toHaveAttribute('aria-pressed', 'true')
  })

  it('is a type="button" so it never submits a form', () => {
    render(<Wrapper />)
    expect(screen.getByRole('button', { name: 'Zobraziť' })).toHaveAttribute('type', 'button')
  })

  it('stays shown across blur, a click elsewhere and a re-render with a new value', () => {
    render(<Wrapper initial="a" />)
    fireEvent.click(screen.getByRole('button', { name: 'Zobraziť' }))
    const input = document.querySelector('input') as HTMLInputElement
    expect(input.type).toBe('text')

    fireEvent.blur(input)
    document.body.click()
    expect(input.type).toBe('text')

    fireEvent.change(input, { target: { value: 'ab' } })
    expect((document.querySelector('input') as HTMLInputElement).type).toBe('text')
    expect(screen.getByRole('button', { name: 'Skryť' })).toBeInTheDocument()
  })

  it('keeps focus on the toggle button through the click (not stolen by the input)', () => {
    render(<Wrapper />)
    const toggle = screen.getByRole('button', { name: 'Zobraziť' })
    toggle.focus()
    fireEvent.click(toggle)
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Skryť' }))
  })
})
