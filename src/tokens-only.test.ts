import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
function walk(dir: string, out: string[] = []): string[] { for (const f of readdirSync(dir)) { const p = join(dir, f); if (statSync(p).isDirectory()) walk(p, out); else if (/\.(css|tsx)$/.test(f) && !f.endsWith('tokens.css')) out.push(p) } return out }
describe('tokens only', () => {
  it('has no literal colours or durations outside tokens.css', () => {
    const bad = walk('src').flatMap((p) => { const s = readFileSync(p, 'utf8'); const hits = s.match(/#[0-9a-fA-F]{3,6}\b|\brgba?\(|\boklch\(|\b\d+ms\b/g) ?? []; return hits.map((h) => `${p}: ${h}`) })
    expect(bad).toEqual([])
  })
})
