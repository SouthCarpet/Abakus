import { describe, expect, it } from 'vitest'
import { parseReleaseNotes } from './releaseNotes'

describe('parseReleaseNotes', () => {
  it('reads a heading line into a heading block, stripped of the leading #', () => {
    expect(parseReleaseNotes('## Novinky')).toEqual([{ kind: 'heading', text: 'Novinky' }])
  })

  it('groups consecutive bullet lines into one bullets block', () => {
    expect(parseReleaseNotes('- prvá vec\n- druhá vec\n* tretia vec')).toEqual([
      { kind: 'bullets', items: ['prvá vec', 'druhá vec', 'tretia vec'] },
    ])
  })

  it('reads a plain line as a paragraph', () => {
    expect(parseReleaseNotes('Obyčajný text.')).toEqual([{ kind: 'paragraph', text: 'Obyčajný text.' }])
  })

  it('keeps heading, bullets and paragraph blocks in their original order', () => {
    const notes = '## Novinky\n- oprava A\n- oprava B\n\nĎalší text.'
    expect(parseReleaseNotes(notes)).toEqual([
      { kind: 'heading', text: 'Novinky' },
      { kind: 'bullets', items: ['oprava A', 'oprava B'] },
      { kind: 'paragraph', text: 'Ďalší text.' },
    ])
  })

  it('starts a new bullets block when a heading or blank line interrupts a list', () => {
    const notes = '- prvá\n\n- druhá'
    expect(parseReleaseNotes(notes)).toEqual([
      { kind: 'bullets', items: ['prvá'] },
      { kind: 'bullets', items: ['druhá'] },
    ])
  })

  it('returns an empty list for empty or whitespace-only notes', () => {
    expect(parseReleaseNotes('')).toEqual([])
    expect(parseReleaseNotes('   \n\n  ')).toEqual([])
  })
})
