import { useCallback, useId, useState } from 'react'
import { PERIOD_LABELS, validPeriod, type PeriodKind } from '../lib/period'
import { Button } from './Button'

const STORAGE_KEY = 'abakus.period'
const KINDS: PeriodKind[] = ['this_month', 'last_month', 'm3', 'm6', 'y1', 'all', 'custom']

export interface PeriodValue {
  kind: PeriodKind
  custom?: { from: string; to: string }
}

const DEFAULT_PERIOD: PeriodValue = { kind: 'this_month' }

function loadPeriod(): PeriodValue {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return DEFAULT_PERIOD
    const parsed = JSON.parse(raw) as Partial<PeriodValue>
    if (parsed && KINDS.includes(parsed.kind as PeriodKind) && validPeriod(parsed as PeriodValue)) return parsed as PeriodValue
  } catch {
    // corrupt or inaccessible storage: fall back to the default period
  }
  return DEFAULT_PERIOD
}

export function usePeriod(): [PeriodValue, (value: PeriodValue) => void] {
  const [period, setPeriodState] = useState<PeriodValue>(loadPeriod)
  const setPeriod = useCallback((value: PeriodValue) => {
    setPeriodState(value)
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(value)) }
    catch { /* Keep controls usable when storage is unavailable. */ }
  }, [])
  return [period, setPeriod]
}

// The group has a visible name ("Obdobie"), so a button such as "Všetko" says what it filters.
// The selected button takes the selected tone, not the accent fill: the register has one lead.
export function PeriodPicker({ value, onChange }: { value: PeriodValue; onChange: (value: PeriodValue) => void }) {
  const nameId = useId()
  return (
    <div className="ab-row">
      <div className="ab-period">
        <span id={nameId} className="ab-field-name">Obdobie</span>
        <div className="ab-row ab-period-buttons" role="group" aria-labelledby={nameId}>
          {KINDS.map((kind) => (
            <Button
              key={kind}
              aria-pressed={value.kind === kind}
              variant="secondary"
              className={value.kind === kind ? 'ab-pressed' : undefined}
              onClick={() => onChange(kind === 'custom' ? { kind, custom: value.custom ?? { from: '', to: '' } } : { kind })}
            >
              {PERIOD_LABELS[kind]}
            </Button>
          ))}
        </div>
      </div>
      {!validPeriod(value) ? <p role="alert">Zadajte platný začiatok a koniec obdobia. Začiatok nesmie byť po konci.</p> : null}
      {value.kind === 'custom' ? (
        <>
          <input
            className="k-input k-well"
            type="date"
            value={value.custom?.from ?? ''}
            aria-label="Od dátumu"
            onChange={(e) => onChange({ kind: 'custom', custom: { from: e.target.value, to: value.custom?.to ?? '' } })}
          />
          <input
            className="k-input k-well"
            type="date"
            value={value.custom?.to ?? ''}
            aria-label="Do dátumu"
            onChange={(e) => onChange({ kind: 'custom', custom: { from: value.custom?.from ?? '', to: e.target.value } })}
          />
        </>
      ) : null}
    </div>
  )
}
