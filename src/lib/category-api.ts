import { invoke } from '@tauri-apps/api/core'
import type { Category, CategoryKind, RuleView } from '../api'

export interface CategoryUpdateRequest {
  id: number
  parent_id: number | null
  name: string
  kind: CategoryKind
  acknowledge_kind_change: boolean
}

export interface CategoryUpdatePreview {
  effective_kind: CategoryKind
  affected_categories: number
  transaction_count: number
  confirmed_count: number
  requires_confirmation: boolean
}

export interface RuleRedirectOutcome {
  rule_id: number
  category_id: number
  updated: number
}

export const categoryApi = {
  preview: (request: CategoryUpdateRequest) => invoke<CategoryUpdatePreview>('category_update_preview', { request }),
  update: (request: CategoryUpdateRequest) => invoke<Category>('update_category', { request }),
  seedRuleForTransaction: (transactionId: number) => invoke<RuleView | null>('seed_rule_for_transaction', { transactionId }),
  redirectRule: (ruleId: number, categoryId: number) => invoke<RuleRedirectOutcome>('update_rule_category', { ruleId, categoryId }),
}
