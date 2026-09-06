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
  },
})
