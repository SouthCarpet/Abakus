import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { resetEscapeLayers } from '../kaliber/escape'
import { HelpPanel } from './HelpPanel'
import { MoreMenu } from './MoreMenu'

beforeEach(() => resetEscapeLayers())
afterEach(() => cleanup())

const trigger = () => screen.getByRole('button', { name: 'Ďalšie' })
const item = (name: string) => screen.getByRole('menuitem', { name })

// Oracle: Kaliber R13 and the WAI-ARIA APG menu button (arrows, Home, End move; Enter runs;
// Escape closes and returns the focus to the trigger; Tab closes; disabled items are skipped).
describe('MoreMenu', () => {
  function setup() {
    const refresh = vi.fn()
    const csv = vi.fn()
    render(<MoreMenu items={[
      { label: 'Obnoviť', onSelect: refresh },
      { label: 'Exportovať filtrované CSV', onSelect: csv, disabled: true },
      { label: 'Exportovať PDF', onSelect: vi.fn() },
    ]} />)
    return { refresh, csv }
  }

  it('opens from its labelled trigger with the focus on the first item', () => {
    setup()
    expect(trigger()).toHaveAttribute('aria-expanded', 'false')
    fireEvent.click(trigger())
    expect(trigger()).toHaveAttribute('aria-expanded', 'true')
    expect(screen.getByRole('menu', { name: 'Ďalšie' })).toHaveClass('is-open')
    expect(document.activeElement).toBe(item('Obnoviť'))
  })

  it('moves with the arrows, Home and End and skips a disabled item', () => {
    setup()
    fireEvent.click(trigger())
    const menu = screen.getByRole('menu', { name: 'Ďalšie' })
    fireEvent.keyDown(menu, { key: 'ArrowDown' })
    expect(document.activeElement).toBe(item('Exportovať PDF'))
    fireEvent.keyDown(menu, { key: 'ArrowDown' })
    expect(document.activeElement).toBe(item('Obnoviť'))
    fireEvent.keyDown(menu, { key: 'End' })
    expect(document.activeElement).toBe(item('Exportovať PDF'))
    fireEvent.keyDown(menu, { key: 'Home' })
    expect(document.activeElement).toBe(item('Obnoviť'))
    fireEvent.keyDown(menu, { key: 'ArrowUp' })
    expect(document.activeElement).toBe(item('Exportovať PDF'))
  })

  it('closes on Escape and returns the focus to the trigger', () => {
    setup()
    fireEvent.click(trigger())
    fireEvent.keyDown(item('Obnoviť'), { key: 'Escape' })
    expect(trigger()).toHaveAttribute('aria-expanded', 'false')
    expect(document.activeElement).toBe(trigger())
  })

  it('runs an item once, closes, and does not run a disabled item', () => {
    const { refresh, csv } = setup()
    fireEvent.click(trigger())
    fireEvent.click(item('Obnoviť'))
    expect(refresh).toHaveBeenCalledTimes(1)
    expect(trigger()).toHaveAttribute('aria-expanded', 'false')
    fireEvent.click(item('Exportovať filtrované CSV'))
    expect(csv).not.toHaveBeenCalled()
  })

  it('closes on Tab', () => {
    setup()
    fireEvent.click(trigger())
    fireEvent.keyDown(screen.getByRole('menu', { name: 'Ďalšie' }), { key: 'Tab' })
    expect(trigger()).toHaveAttribute('aria-expanded', 'false')
  })
})

// Oracle: Kaliber components.md "Side sheet and drawer": the panel traps the Tab key.
describe('HelpPanel focus trap', () => {
  it('wraps Tab from the last control to the first and Shift+Tab back', () => {
    render(<HelpPanel open screen="transactions" onClose={vi.fn()} />)
    const panel = screen.getByRole('dialog', { name: 'Pomocník' })
    const focusable = Array.from(panel.querySelectorAll<HTMLElement>('button, input, summary'))
    const first = focusable[0]
    const last = focusable[focusable.length - 1]
    last.focus()
    fireEvent.keyDown(last, { key: 'Tab' })
    expect(document.activeElement).toBe(first)
    fireEvent.keyDown(first, { key: 'Tab', shiftKey: true })
    expect(document.activeElement).toBe(last)
  })
})
