import { describe, expect, it } from 'vitest'
import { UPDATE_PHASE_LABEL, updateButtonLabel } from './updatePhase'

describe('UPDATE_PHASE_LABEL', () => {
  it('states the three Slovak labels the update button cycles through, in click order', () => {
    expect(UPDATE_PHASE_LABEL.downloading).toBe('Sťahujem inštalátor…')
    expect(UPDATE_PHASE_LABEL.verifying).toBe('Overujem podpis súboru…')
    expect(UPDATE_PHASE_LABEL.launching).toBe('Spúšťam inštalátor…')
  })
})

describe('updateButtonLabel', () => {
  it('prompts for the click with the tag when idle', () => {
    expect(updateButtonLabel('idle', 'v0.1.4')).toBe('Aktualizovať na v0.1.4')
  })

  it('shows the same click prompt on an error, so the user can retry', () => {
    expect(updateButtonLabel('error', 'v0.1.4')).toBe('Aktualizovať na v0.1.4')
  })

  it('shows each in-progress label while that phase is running', () => {
    expect(updateButtonLabel('downloading', 'v0.1.4')).toBe('Sťahujem inštalátor…')
    expect(updateButtonLabel('verifying', 'v0.1.4')).toBe('Overujem podpis súboru…')
    expect(updateButtonLabel('launching', 'v0.1.4')).toBe('Spúšťam inštalátor…')
  })
})
