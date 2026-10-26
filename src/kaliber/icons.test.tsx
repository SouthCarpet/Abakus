import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { readFileSync } from 'node:fs'
import { afterEach, describe, expect, it } from 'vitest'
import { Icon, type IconName } from './icons'
import { InlineAlert } from '../components/InlineAlert'
import { OperationStatus } from '../components/OperationStatus'
import { HelpPanel } from '../components/HelpPanel'
import { Toasts } from '../components/Toasts'
import { resetToasts, toast } from './toast'

// Oracle: A2 brief items 1, 2 and 5 and design/assets/icons/README.md.
// Cases: every used symbol exists; labelled icon-only control; four alert meanings;
// error row; error toast versus the unchanged success dot. No progress API reports a percent.
const names: IconName[] = ['sun', 'moon', 'chevron-down', 'close', 'archive', 'delete',
  'list', 'chart-bar', 'import', 'folder', 'settings', 'info', 'success', 'warning', 'danger']
const sprite = readFileSync('public/kaliber-icons.svg', 'utf8')

afterEach(() => { cleanup(); resetToasts() })

describe('Kaliber 5 icons and states', () => {
  it.each(names)('resolves %s to a vendored symbol, decorative beside text', (name) => {
    const { container } = render(<span><Icon name={name} small />Text</span>)
    expect(container.querySelector('svg')).toHaveClass('k-icon', 'k-icon--sm')
    expect(container.querySelector('svg')).toHaveAttribute('aria-hidden', 'true')
    expect(container.querySelector('use')).toHaveAttribute('href', `/kaliber-icons.svg#k-icon-${name}`)
    expect(sprite).toContain(`id="k-icon-${name}"`)
  })

  it('keeps the icon-only Help close button named and usable', () => {
    let closed = false
    render(<HelpPanel open screen="transactions" onClose={() => { closed = true }} />)
    const close = screen.getByRole('button', { name: 'Zavrieť pomocníka' })
    expect(close).toHaveAccessibleName('Zavrieť pomocníka')
    expect(close.querySelector('svg')).toHaveAttribute('aria-hidden', 'true')
    expect(close.querySelector('use')).toHaveAttribute('href', '/kaliber-icons.svg#k-icon-close')
    expect(close.querySelector('svg')).not.toHaveClass('k-icon--sm')
    fireEvent.click(close)
    expect(closed).toBe(true)
  })

  it.each([
    ['info', 'info', 'status'], ['success', 'success', 'status'],
    ['warning', 'warning', 'alert'], ['error', 'danger', 'alert'],
  ] as const)('gives the %s inline alert its meaning and readable text', (kind, icon, role) => {
    render(<InlineAlert kind={kind}>Správa</InlineAlert>)
    const alert = screen.getByRole(role)
    expect(alert).toHaveTextContent('Správa')
    expect(alert.querySelector('svg')).toHaveClass('k-icon', 'k-icon--sm', 'k-alert-icon')
    expect(alert.querySelector('use')).toHaveAttribute('href', `/kaliber-icons.svg#k-icon-${icon}`)
  })

  it('keeps a static coverage warning out of live alert announcements', () => {
    // Oracle: existing Overview error recovery clears the sole live error (Audit078).
    render(<InlineAlert kind="warning" announce={false}>Chýba výpis.</InlineAlert>)
    expect(screen.getByText('Chýba výpis.')).toBeVisible()
    expect(screen.queryByRole('alert')).toBeNull()
    expect(screen.queryByRole('status')).toBeNull()
  })

  it('gives an operation error the danger shape while retaining its error text', () => {
    render(<OperationStatus error="Záloha zlyhala." />)
    const alert = screen.getByRole('alert')
    expect(alert).toHaveClass('k-state', 'is-error')
    expect(alert).toHaveTextContent('Záloha zlyhala.')
    expect(alert.querySelector('.k-state-icon use')).toHaveAttribute('href', '/kaliber-icons.svg#k-icon-danger')
  })

  it('uses danger for an error toast and keeps a dot for a confirmation', () => {
    render(<Toasts />)
    act(() => { toast('Zlyhalo.', undefined, 'error'); toast('Uložené.') })
    const error = screen.getByText('Zlyhalo.').closest('.k-toast')!
    const success = screen.getByText('Uložené.').closest('.k-toast')!
    expect(error).toHaveClass('k-toast--error')
    expect(error.querySelector('.k-toast-dot use')).toHaveAttribute('href', '/kaliber-icons.svg#k-icon-danger')
    expect(success).not.toHaveClass('k-toast--error')
    expect(success.querySelector('.k-toast-dot')).not.toBeNull()
    expect(success.querySelector('svg')).toBeNull()
  })
})
