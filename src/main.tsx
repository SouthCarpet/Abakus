import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './tokens.css'
import './kaliber.css'
import { App } from './App'

const rootEl = document.getElementById('root')
if (!rootEl) throw new Error('missing #root element')

createRoot(rootEl).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
