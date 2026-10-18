import { useState } from 'react'
import { Button, type ButtonVariant } from '../Button'
import { ExportPdfDialog } from './ExportPdfDialog'
import './report.css'

// Kaliber R12: a report command, so it sits in the screen header (Prehľad: the primary action).
export function ExportPdfAction({ variant = 'secondary' }: { variant?: ButtonVariant }) {
  const [open, setOpen] = useState(false)

  return <>
    <Button variant={variant} onClick={() => setOpen(true)}>Exportovať PDF</Button>
    <ExportPdfDialog open={open} onClose={() => setOpen(false)} />
  </>
}
