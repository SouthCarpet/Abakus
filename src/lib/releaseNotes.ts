// Renders a GitHub release body as readable blocks, never as HTML: a
// release's notes are third-party text (spec 0.1.4, Michal 2026-09-08), so
// Settings.tsx must never feed them to dangerouslySetInnerHTML. This parses
// only the handful of Markdown shapes GitHub release notes actually use
// (headings, bullet lists, plain paragraphs) into a small, pure block list
// that a component renders as ordinary React text nodes, which React itself
// escapes.
export type ReleaseNoteBlock = { kind: 'heading'; text: string } | { kind: 'bullets'; items: string[] } | { kind: 'paragraph'; text: string }

const HEADING = /^#{1,6}\s+(.*)$/
const BULLET = /^[-*]\s+(.*)$/

export function parseReleaseNotes(markdown: string): ReleaseNoteBlock[] {
  const blocks: ReleaseNoteBlock[] = []
  let bulletBuffer: string[] = []

  function flushBullets() {
    if (bulletBuffer.length === 0) return
    blocks.push({ kind: 'bullets', items: bulletBuffer })
    bulletBuffer = []
  }

  for (const rawLine of markdown.replace(/\r\n/g, '\n').split('\n')) {
    const line = rawLine.trim()
    if (line === '') { flushBullets(); continue }

    const heading = HEADING.exec(line)
    if (heading) { flushBullets(); blocks.push({ kind: 'heading', text: heading[1].trim() }); continue }

    const bullet = BULLET.exec(line)
    if (bullet) { bulletBuffer.push(bullet[1].trim()); continue }

    flushBullets()
    blocks.push({ kind: 'paragraph', text: line })
  }
  flushBullets()
  return blocks
}
