import { useCallback, useState } from 'react'
import { PERIOD_LABELS, type PeriodKind } from '../lib/period'
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
    if (typeof parsed.kind === 'string') return { kind: parsed.kind, custom: parsed.custom }
  } catch {
    // corrupt or inaccessible storage: fall back to the default period
  }
  return DEFAULT_PERIOD
}

export function usePeriod(): [PeriodValue, (value: PeriodValue) => void] {
  const [period, setPeriodState] = useState<PeriodValue>(loadPeriod)
  const setPeriod = useCallback((value: PeriodValue) => {
    setPeriodState(value)
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
  }, [])
  return [period, setPeriod]
}

export function PeriodPicker({ value, onChange }: { value: PeriodValue; onChange: (value: PeriodValue) => void }) {
  return (
    <div className="k-row">
      <div className="k-row" role="group" aria-label="Obdobie">
        {KINDS.map((kind) => (
          <Button
            key={kind}
            variant={value.kind === kind ? 'primary' : 'secondary'}
            onClick={() => onChange(kind === 'custom' ? { kind, custom: value.custom ?? { from: '', to: '' } } : { kind })}
          >
            {PERIOD_LABELS[kind]}
          </Button>
        ))}
      </div>
      {value.kind === 'custom' ? (
        <>
          <input
            className="k-input k-well"
            type="date"
            value={value.custom?.from ?? ''}
            onChange={(e) => onChange({ kind: 'custom', custom: { from: e.target.value, to: value.custom?.to ?? '' } })}
          />
          <input
            className="k-input k-well"
            type="date"
            value={value.custom?.to ?? ''}
            onChange={(e) => onChange({ kind: 'custom', custom: { from: value.custom?.from ?? '', to: e.target.value } })}
          />
        </>
      ) : null}
    </div>
  )
}
