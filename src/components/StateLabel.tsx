import type { Status } from '../api'
import { statusLabel } from '../lib/categories'

// Kaliber R28: the shape carries the state, the word names it, colour only supplements.
// Nezaradené: empty dot. Odhad: half dot. Potvrdené: full dot.
// Prevod (outside the totals): dot with a line through it.
const MARK_CLASS: Record<Status, string> = {
  unassigned: 'k-mark',
  suggested: 'k-mark k-mark--half',
  confirmed: 'k-mark k-mark--full',
  transfer: 'k-mark ab-mark--transfer',
}

export function StateMark({ status }: { status: Status }) {
  return <span className={MARK_CLASS[status]} aria-hidden="true" />
}

export function StateLabel({ status }: { status: Status }) {
  return (
    <span className="k-state-label">
      <StateMark status={status} />
      {statusLabel(status)}
    </span>
  )
}
