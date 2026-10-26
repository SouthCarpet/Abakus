import { useId } from 'react'
import { MoonGlyph, SunGlyph } from '../kaliber/icons'
import { useTheme, type ThemePreference } from '../kaliber/theme'

// Kaliber R20: two theme controls, one state (`kaliber-theme`, src/kaliber/theme.ts).

/** The quick sun and moon switch in the header tools. Unchecked is the light theme, checked
 *  the dark theme. It shows the theme in use, so in System mode it follows the system; a
 *  click saves Light or Dark. */
export function ThemeSwitch() {
  const { theme, choose } = useTheme()
  return (
    <label className="k-switch k-switch--theme">
      <input
        type="checkbox"
        role="switch"
        className="k-switch-input"
        aria-label="Tmavý vzhľad"
        checked={theme === 'dark'}
        onChange={(event) => choose(event.target.checked ? 'dark' : 'light')}
      />
      <span className="k-switch-track" aria-hidden="true">
        <span className="k-switch-bg k-theme-part" />
        <span className="k-switch-thumb k-theme-part">
          <SunGlyph className="k-sun k-theme-part" />
          <MoonGlyph className="k-moon k-theme-part" />
        </span>
      </span>
    </label>
  )
}

const MODES: { value: ThemePreference; label: string }[] = [
  { value: 'light', label: 'Svetlý' },
  { value: 'dark', label: 'Tmavý' },
  { value: 'system', label: 'Podľa systému' },
]

/** The full control in Settings > Vzhľad: a segmented control of native radios. */
export function ThemeMode() {
  const { preference, choose } = useTheme()
  const name = useId()
  return (
    <fieldset className="k-radio-group ab-theme-mode">
      <legend className="k-group-legend k-sr-only">Vzhľad</legend>
      <div className="k-segmented ab-theme-segmented">
        {MODES.map((mode) => (
          <label key={mode.value} className="k-segmented-item">
            <input
              type="radio"
              className="k-segmented-input"
              name={name}
              value={mode.value}
              checked={preference === mode.value}
              onChange={() => choose(mode.value)}
            />
            <span className="k-segmented-label">{mode.label}</span>
          </label>
        ))}
      </div>
    </fieldset>
  )
}
