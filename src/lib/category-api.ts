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

export interface CategoryDeleteItem {
  id: number
  parent_id: number | null
  name: string
  archived: boolean
}

export interface CategoryDeletePreview {
  category_id: number
  affected_categories: CategoryDeleteItem[]
  transaction_count: number
  confirmed_count: number
  rule_count: number
  rule_source_count: number
  recurring_member_count: number
}

export interface CategoryDeleteRequest {
  preview: CategoryDeletePreview
}

export interface RuleDeletePreview {
  rule_id: number
  open_rule_references: number
  open_classification_changes: number
}

export const categoryApi = {
  preview: (request: CategoryUpdateRequest) => invoke<CategoryUpdatePreview>('category_update_preview', { request }),
  update: (request: CategoryUpdateRequest) => invoke<Category>('update_category', { request }),
  previewDelete: (categoryId: number) => invoke<CategoryDeletePreview>('category_delete_preview', { categoryId }),
  deleteCategory: (preview: CategoryDeletePreview) => invoke<CategoryDeletePreview>('delete_category', { request: { preview } satisfies CategoryDeleteRequest }),
  seedRuleForTransaction: (transactionId: number) => invoke<RuleView | null>('seed_rule_for_transaction', { transactionId }),
  redirectRule: (ruleId: number, categoryId: number) => invoke<RuleRedirectOutcome>('update_rule_category', { ruleId, categoryId }),
  previewRuleDelete: (ruleId: number) => invoke<RuleDeletePreview>('rule_delete_preview', { ruleId }),
}
