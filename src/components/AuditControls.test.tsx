import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ThemePicker } from './ThemePicker'
import { PeriodPicker, usePeriod } from './PeriodPicker'
import { Dialog } from './Dialog'
import { useState } from 'react'

let osChange: (event: MediaQueryListEvent) => void
beforeEach(() => {
  localStorage.clear()
  vi.stubGlobal('matchMedia', () => ({ matches: false, addEventListener: (_: string, callback: typeof osChange) => { osChange = callback }, removeEventListener: () => {} }))
})
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.unstubAllGlobals(); localStorage.clear() })

// Oracle: contracts.md Theme, three validated persistent modes and safe storage fallback.
describe('theme controls', () => {
  it('keeps explicit dark through OS changes and restart, then follows system when selected', () => {
    const view = render(<ThemePicker />)
    fireEvent.change(screen.getByRole('combobox', { name: 'Vzhľad' }), { target: { value: 'dark' } })
    expect(document.documentElement.dataset.theme).toBe('dark')
    act(() => osChange({ matches: false } as MediaQueryListEvent))
    expect(document.documentElement.dataset.theme).toBe('dark')
    view.unmount(); render(<ThemePicker />)
    expect(screen.getByRole('combobox', { name: 'Vzhľad' })).toHaveValue('dark')
    fireEvent.change(screen.getByRole('combobox', { name: 'Vzhľad' }), { target: { value: 'system' } })
    expect(document.documentElement.dataset.theme).toBe('light')
    act(() => osChange({ matches: true } as MediaQueryListEvent))
    expect(document.documentElement.dataset.theme).toBe('dark')
    fireEvent.change(screen.getByRole('combobox', { name: 'Vzhľad' }), { target: { value: 'light' } })
    act(() => osChange({ matches: true } as MediaQueryListEvent))
    expect(document.documentElement.dataset.theme).toBe('light')
  })
  it.each(['garbage', '{', 'null'])('falls back to system for corrupt preference %s', (stored) => {
    localStorage.setItem('abakus.theme', stored)
    render(<ThemePicker />)
    expect(screen.getByRole('combobox', { name: 'Vzhľad' })).toHaveValue('system')
  })
  it('keeps controls usable when reading and writing storage fail', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('denied') })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('full') })
    render(<ThemePicker />)
    expect(screen.getByRole('combobox', { name: 'Vzhľad' })).toHaveValue('system')
    fireEvent.change(screen.getByRole('combobox', { name: 'Vzhľad' }), { target: { value: 'dark' } })
    expect(document.documentElement.dataset.theme).toBe('dark')
  })
})
function PeriodControl() {
  const [value, choose] = usePeriod()
  return <PeriodPicker value={value} onChange={choose} />
}
// Oracle: audit brief requires validated period storage; default is existing this_month.
it.each(['null', '{"kind":"unknown"}', '{"kind":"custom","custom":{"from":"2026-02-30","to":"2026-03-01"}}'])('ignores invalid stored period %s and keeps period controls working', (stored) => {
  localStorage.setItem('abakus.period', stored)
  render(<PeriodControl />)
  expect(screen.queryByLabelText('Od dátumu')).not.toBeInTheDocument()
  fireEvent.click(screen.getByRole('button', { name: 'Vlastné' }))
  expect(screen.getByRole('alert')).toHaveTextContent('Zadajte platný')
})
function DialogControl() {
  const [open, setOpen] = useState(false)
  const [text, setText] = useState('')
  return <><button onClick={() => setOpen(true)}>Otvoriť</button><Dialog open={open} title="Test" onClose={() => setOpen(false)}>
    <button onClick={() => setOpen(false)}>Zavrieť</button>
    <input aria-label="Text" value={text} onChange={(e) => setText(e.target.value)} />
  </Dialog></>
}
// Oracle: keyboard operation audit: rerenders cannot steal typing focus; close restores trigger.
it('retains input focus across dialog rerenders and restores the trigger after Escape', () => {
  render(<DialogControl />)
  const trigger = screen.getByRole('button', { name: 'Otvoriť' })
  trigger.focus(); fireEvent.click(trigger)
  const input = screen.getByRole('textbox', { name: 'Text' })
  input.focus(); fireEvent.change(input, { target: { value: 'abc' } })
  expect(input).toHaveFocus()
  fireEvent.keyDown(input, { key: 'Escape' })
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  expect(trigger).toHaveFocus()
})
