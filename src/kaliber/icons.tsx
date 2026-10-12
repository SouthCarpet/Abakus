// The Abakus glyph family (identity lever R16 point 4): Lucide outlines at a 1.5 px stroke,
// drawn inline so the app needs no icon package. Paths from Lucide (ISC licence, see
// THIRD_PARTY_NOTICES.md): sun, moon, chevron-down, x, archive, trash-2.

import type { ReactNode } from 'react'

function Glyph({ size = 16, className, children }: { size?: number; className?: string; children: ReactNode }) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {children}
    </svg>
  )
}

export function SunGlyph({ className }: { className?: string }) {
  return (
    <Glyph size={14} className={className}>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2" /><path d="M12 20v2" /><path d="m4.93 4.93 1.41 1.41" /><path d="m17.66 17.66 1.41 1.41" />
      <path d="M2 12h2" /><path d="M20 12h2" /><path d="m6.34 17.66-1.41 1.41" /><path d="m19.07 4.93-1.41 1.41" />
    </Glyph>
  )
}

export function MoonGlyph({ className }: { className?: string }) {
  return <Glyph size={14} className={className}><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z" /></Glyph>
}

export function ChevronGlyph({ className = 'k-chevron' }: { className?: string }) {
  return <Glyph className={className}><path d="m6 9 6 6 6-6" /></Glyph>
}

export function ArchiveGlyph() {
  return <Glyph><rect width="20" height="5" x="2" y="3" rx="1" /><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8" /><path d="M10 12h4" /></Glyph>
}

export function TrashGlyph() {
  return (
    <Glyph>
      <path d="M3 6h18" /><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" /><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
      <path d="M10 11v6" /><path d="M14 11v6" />
    </Glyph>
  )
}

export function CloseGlyph() {
  return <Glyph><path d="M18 6 6 18" /><path d="m6 6 12 12" /></Glyph>
}
