/// <reference types="vitest/config" />
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const host = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [react()],
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
