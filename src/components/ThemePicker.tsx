import { useTheme, type ThemePreference } from '../lib/theme'
import { Field } from './Field'

export function ThemePicker() {
  const [theme, choose] = useTheme()
  return <Field label="Vzhľad">
    <select className="k-select k-well" value={theme} onChange={(e) => choose(e.target.value as ThemePreference)}>
      <option value="light">Svetlý</option>
      <option value="dark">Tmavý</option>
      <option value="system">Podľa systému</option>
    </select>
  </Field>
}
