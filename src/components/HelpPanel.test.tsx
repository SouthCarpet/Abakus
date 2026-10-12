import { describe, expect, it } from 'vitest'
import { HELP_TOPICS } from './HelpPanel'

// Oracle: README "Aktualizácia" and "Vzhľad", KNOWN_ISSUES (Kaliber r3) and the update contract:
// three paths, an off switch that sends nothing, and one theme key with its migration. A reader
// who only has the in-app help must be able to learn each of them.
describe('Help: settings topic', () => {
  const lines = HELP_TOPICS.find((topic) => topic.id === 'settings')?.lines.join('\n') ?? ''

  it('names the three update-check paths and the off rule', () => {
    expect(lines).toContain('pri spustení')
    expect(lines).toContain('pri otvorení sekcie O aplikácii')
    expect(lines).toContain('na tlačidlo Skontrolovať aktualizácie')
    expect(lines).toContain('Pri vypnutej kontrole sa nič neposiela')
  })

  it('names the theme key and the abakus.theme migration', () => {
    expect(lines).toContain('kaliber-theme')
    expect(lines).toContain('abakus.theme')
  })
})
