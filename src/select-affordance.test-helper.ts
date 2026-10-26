import { expect } from 'vitest'

// Check all native selects on the rendered route, including future additions.
export function expectSelectChevrons(root: ParentNode = document) {
  const selects = root.querySelectorAll('select')
  expect(selects.length).toBeGreaterThan(0)
  for (const select of selects) {
    const wrap = select.closest('.k-select-wrap')
    expect(wrap?.querySelector('use')).toHaveAttribute('href', '/kaliber-icons.svg#k-icon-chevron-down')
    expect(wrap?.querySelector('svg')).toHaveAttribute('aria-hidden', 'true')
  }
}
