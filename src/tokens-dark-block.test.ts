import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

// Only colour and shadow tokens are theme-dependent. Typography, spacing,
// radius and motion tokens are the same in both themes on purpose and stay
// defined once in :root, inherited by [data-theme="dark"]; requiring them
// there too would fail well-formed CSS for no reason.
const THEME_DEPENDENT = /^--(color|shadow)-/

function themeDependentProperties(block: string): Set<string> {
  const names = block.match(/--[\w-]+(?=:)/g) ?? []
  return new Set(names.filter((name) => THEME_DEPENDENT.test(name)))
}

// A17: a missing dark value is a hole in the theme that only shows up when
// the app actually switches to dark, which no other test exercises. This
// reads tokens.css directly so it fails the moment the two blocks drift.
describe('dark theme block reachability', () => {
  it('defines a dark value for every colour/shadow token the light theme defines', () => {
    const css = readFileSync('src/tokens.css', 'utf8')
    const lightBlock = css.match(/:root\s*\{([\s\S]*?)\n\}/)?.[1] ?? ''
    const darkBlock = css.match(/\[data-theme="dark"\]\s*\{([\s\S]*?)\n\}/)?.[1] ?? ''
    const light = themeDependentProperties(lightBlock)
    const dark = themeDependentProperties(darkBlock)
    expect(light.size).toBeGreaterThan(0)
    const missing = [...light].filter((name) => !dark.has(name))
    expect(missing).toEqual([])
  })
})
