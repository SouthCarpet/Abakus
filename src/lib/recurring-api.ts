import { invoke } from '@tauri-apps/api/core'
import type { AccountKind, TxRow } from '../api'

export type Cadence = 'monthly' | 'quarterly' | 'yearly'
export type Direction = 'expense' | 'income'
export type RecurringState = 'active' | 'upcoming' | 'missing' | 'ended' | 'unknown'
export type UnknownReason = 'missing_coverage' | 'ambiguous_membership' | 'no_evidence' | 'unmatched_history' | null

export interface RecurringQuery {
  from: string | null
  to: string | null
  account_kind: AccountKind | null
  today: string
}

export type RecurringSelection =
  | { scope: 'group'; transaction_id: number }
  | { scope: 'selected'; transaction_ids: number[] }

export type RecurringDecisionInput =
  | { mode: 'confirmed'; cadence: Cadence; anchor_date: string }
  | { mode: 'ignored' }

export interface SaveRecurringRequest {
  decision_id: number | null
  selection: RecurringSelection
  decision: RecurringDecisionInput
}

export interface RecurringDecision {
  id: number
  series_key: string
  group_key: string
  account_id: number
  scope: 'group' | 'selected'
  mode: 'confirmed' | 'ignored'
  cadence: Cadence | null
  anchor_date: string | null
  updated_at: string
}

export interface PriceChange {
  currency: string
  previous_cents: number
  current_cents: number
  delta_cents: number
  effective_from: string
}

export interface RecurringRow {
  series_key: string
  group_key: string
  decision_id: number | null
  scope: 'group' | 'selected'
  decision: 'estimate' | 'confirmed' | 'ignored'
  account_id: number
  account_label: string
  account_kind: AccountKind
  direction: Direction
  name: string
  cadence: Cadence | null
  anchor_date: string | null
  subscription_category: boolean
  category_id: number | null
  evidence_count: number
  last_paid: string | null
  next_due: string | null
  state: RecurringState
  unknown_reason: UnknownReason
  grace_until: string | null
  currency: string | null
  original_amount_cents: number | null
  amount_cents: number | null
  amount_basis: 'stable_pair' | 'latest_observation' | null
  monthly_cents: number | null
  annual_cents: number | null
  price_change: PriceChange | null
  manual_membership: boolean
  foreign_eur_estimate: boolean
}

export interface RecurringAmounts {
  monthly_income_cents: number
  monthly_expense_cents: number
  monthly_net_cents: number
  annual_income_cents: number
  annual_expense_cents: number
  annual_net_cents: number
  remaining_income_cents: number
  remaining_expense_cents: number
}

export interface RecurringOverview {
  as_of: string
  history_from: string | null
  future_period: boolean
  unfinished_period: boolean
  rows: RecurringRow[]
  confirmed: RecurringAmounts
  estimates: RecurringAmounts
  excluded: { missing: number; ended: number; unknown: number }
  expense_share_basis_points: number | null
  average_expense_cents: number | null
  average_months: string[]
}

export interface RecurringDetailRequest {
  series_key: string
  query: RecurringQuery
}

export interface RecurringDetail {
  row: RecurringRow
  transactions: TxRow[]
  matching_transaction_ids: number[]
  compatible_transactions: TxRow[]
}

export interface TransactionRecurringContext {
  transaction_id: number
  group_key: string | null
  ambiguous: boolean
  decision: RecurringDecision | null
  inferred_cadence: Cadence | null
  compatible_transactions: TxRow[]
}

export const recurringApi = {
  overview: (query: RecurringQuery) => invoke<RecurringOverview>('recurring_overview', { query }),
  detail: (request: RecurringDetailRequest) => invoke<RecurringDetail>('recurring_detail', { request }),
  transactionContext: (transactionId: number, asOf: string) =>
    invoke<TransactionRecurringContext>('transaction_recurring_context', { transactionId, asOf }),
  save: (request: SaveRecurringRequest) => invoke<RecurringDecision>('save_recurring', { request }),
  reset: (decisionId: number) => invoke<void>('reset_recurring', { decisionId }),
}
