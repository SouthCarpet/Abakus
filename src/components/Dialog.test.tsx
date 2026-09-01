import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { Dialog } from './Dialog'

afterEach(() => cleanup())

describe('Dialog: accessibility', () => {
  it('closes on Escape', () => {
    const onClose = vi.fn()
    render(
      <Dialog open title="Test" onClose={onClose}>
        <input aria-label="pole" />
      </Dialog>,
    )
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(onClose).toHaveBeenCalled()
  })

  it('focuses the first control when it opens', () => {
    render(
      <Dialog open title="Test" onClose={() => {}}>
        <input aria-label="prve" />
      </Dialog>,
    )
    expect(document.activeElement).toBe(screen.getByLabelText('prve'))
  })

  it('traps Tab inside the dialog, wrapping from the last control back to the first and back', () => {
    render(
      <Dialog open title="Test" onClose={() => {}} actions={<button>Uložiť</button>}>
        <input aria-label="prve" />
        <input aria-label="druhe" />
      </Dialog>,
    )
    const first = screen.getByLabelText('prve')
    const save = screen.getByRole('button', { name: 'Uložiť' })

    first.focus()
    fireEvent.keyDown(document, { key: 'Tab', shiftKey: true })
    expect(document.activeElement).toBe(save)

    save.focus()
    fireEvent.keyDown(document, { key: 'Tab' })
    expect(document.activeElement).toBe(first)
  })

  it('renders nothing, and stops listening, once closed', () => {
    const onClose = vi.fn()
    const { rerender } = render(
      <Dialog open title="Test" onClose={onClose}>
        <input aria-label="pole" />
      </Dialog>,
    )
    rerender(
      <Dialog open={false} title="Test" onClose={onClose}>
        <input aria-label="pole" />
      </Dialog>,
    )
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    fireEvent.keyDown(document, { key: 'Escape' })
    expect(onClose).not.toHaveBeenCalled()
  })
})
