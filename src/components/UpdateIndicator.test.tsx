import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Release } from '../api'
import { setLatestRelease } from '../lib/updateStatus'
import { UpdateIndicator } from './UpdateIndicator'

const release: Release = {
  tag: '0.2.0',
  url: 'https://github.com/SouthCarpet/Abakus/releases/tag/0.2.0',
  notes: '',
  installer_url: null,
  installer_name: null,
  installer_size: null,
  checksums_url: null,
}

afterEach(() => { cleanup(); setLatestRelease(null) })

describe('UpdateIndicator', () => {
  it('shows nothing when no release is known yet', () => {
    render(<UpdateIndicator onOpenAbout={vi.fn()} />)
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
  })

  it('names the found version and leads to About on click, as a quiet line', () => {
    setLatestRelease(release)
    const onOpenAbout = vi.fn()
    render(<UpdateIndicator onOpenAbout={onOpenAbout} />)
    const button = screen.getByRole('button', { name: /0\.2\.0/ })
    fireEvent.click(button)
    expect(onOpenAbout).toHaveBeenCalledTimes(1)
    expect(button).toHaveClass('k-update-line')
  })
})
