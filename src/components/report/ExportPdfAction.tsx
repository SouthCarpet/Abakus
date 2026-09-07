import { useState } from 'react'
import { Button } from '../Button'
import { ExportPdfDialog } from './ExportPdfDialog'
import './report.css'

export function ExportPdfAction() {
  const [open, setOpen] = useState(false)

  return <>
    <Button variant="secondary" onClick={() => setOpen(true)}>Exportovať PDF</Button>
    <ExportPdfDialog open={open} onClose={() => setOpen(false)} />
  </>
}
