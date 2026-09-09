import { useId, useState, type MouseEvent } from 'react'
import { Button } from './Button'

// The reveal toggle Abakus draws itself, so it stays after the field loses
// focus (WebView2's own `::-ms-reveal` icon hides for good once the field is
// blurred; `kaliber.css` turns that icon off). `shown` lives only in this
// component: a dialog that unmounts on close (`Dialog` returns `null` while
// `!open`) throws this state away with it, so a fresh open always starts
// hidden.
export function PasswordInput({
  id,
  value,
  onChange,
  maxLength,
  autoFocus,
  ariaLabel,
}: {
  id?: string
  value: string
  onChange: (value: string) => void
  maxLength?: number
  autoFocus?: boolean
  ariaLabel?: string
}) {
  const [shown, setShown] = useState(false)
  const generatedId = useId()
  const inputId = id ?? generatedId

  // Stops the click before it reaches the surrounding `<label>` (see
  // `Field`): a label with two labelable descendants forwards an
  // unclicked-on click to the first one, the input, which would steal focus
  // back from this button right after the toggle.
  function toggle(event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation()
    setShown((was) => !was)
  }

  return (
    <div className="k-password-row">
      <input
        id={inputId}
        type={shown ? 'text' : 'password'}
        className="k-input k-well"
        value={value}
        maxLength={maxLength}
        autoFocus={autoFocus}
        aria-label={ariaLabel}
        onChange={(e) => onChange(e.target.value)}
      />
      <Button
        type="button"
        variant="secondary"
        aria-pressed={shown}
        aria-controls={inputId}
        onClick={toggle}
      >
        {shown ? 'Skryť' : 'Zobraziť'}
      </Button>
    </div>
  )
}
