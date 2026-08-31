import { cleanup, render } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { Kpi } from './Kpi'

afterEach(() => cleanup())

describe('Kpi', () => {
  it('fills the gauge track for a normal tile', () => {
    const { container } = render(<Kpi label="Príjem" cents={500} min={0} max={1000} />)
    expect(container.querySelector('.k-gauge-fill')).not.toBeNull()
  })

  it('baselineOnly renders an empty track: no fill, no tick', () => {
    const { container } = render(<Kpi label="Čisté" cents={500} baselineOnly />)
    expect(container.querySelector('.k-gauge-fill')).toBeNull()
    expect(container.querySelector('.k-gauge-tick')).toBeNull()
    expect(container.querySelector('.k-gauge')).not.toBeNull()
  })
})
