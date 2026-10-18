/// <reference types="vitest/config" />
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { execSync } from 'node:child_process'
import { readFileSync } from 'node:fs'

// About (Kaliber R12): the version and the build the user runs. The build is the short git
// commit at build time; without git it says so.
const version = (JSON.parse(readFileSync('package.json', 'utf8')) as { version: string }).version
function gitBuild(): string {
  try { return execSync('git rev-parse --short HEAD', { stdio: ['ignore', 'pipe', 'ignore'] }).toString().trim() } catch { return 'neznáme' }
}

const host = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [react()],
  define: { __APP_VERSION__: JSON.stringify(version), __APP_BUILD__: JSON.stringify(gitBuild()) },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host ?? false,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  test: {
    environment: 'jsdom',
    // Node 25's global storage shadows jsdom unless disabled in test workers.
    execArgv: Number(process.versions.node.split('.')[0]) >= 25 ? ['--no-experimental-webstorage'] : [],
    setupFiles: ['./src/test-setup.ts'],
    // A nested git worktree (.worktrees/<name>) carries its own copy of every
    // test file; without this exclude a plain `vitest run` walks it and does
    // not finish (plan 094 G1 verifier finding, 2026-09-20).
    exclude: ['**/node_modules/**', '**/dist/**', '**/.worktrees/**'],
  },
})
