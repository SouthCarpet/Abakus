import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ExportPdfAction } from './ExportPdfAction'

vi.mock('./ExportPdfDialog', () => ({ ExportPdfDialog: ({ open, onClose }: { open: boolean; onClose: () => void }) => open ? <button onClick={onClose}>Zavrieť export</button> : null }))

afterEach(() => cleanup())

describe('ExportPdfAction: dialog ownership', () => {
  it('opens the export dialog from its toolbar action and closes it through the dialog callback', () => {
    render(<ExportPdfAction />)
    fireEvent.click(screen.getByRole('button', { name: 'Exportovať PDF' }))
    expect(screen.getByRole('button', { name: 'Zavrieť export' })).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Zavrieť export' }))
    expect(screen.queryByRole('button', { name: 'Zavrieť export' })).not.toBeInTheDocument()
  })
})
