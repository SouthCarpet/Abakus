import { invoke } from '@tauri-apps/api/core'

export type AccountKind = 'personal' | 'business'
export type CategoryKind = 'expense' | 'income'
export type Status = 'transfer' | 'confirmed' | 'suggested' | 'unassigned'
export type Checksum = { status: 'ok' } | { status: 'off_by'; off_by: number } | { status: 'not_verifiable' }
export type ImportStatus = 'imported' | 'already_imported' | 'locked' | 'unknown_account' | 'error'

export interface Account { id: number; iban: string; kind: AccountKind; label: string; has_password: boolean }
export interface Category { id: number; parent_id: number | null; name: string; kind: CategoryKind; sort: number; system: boolean; archived: boolean }
export interface RuleView { id: number; kind: 'exact' | 'merchant' | 'counterparty_account' | 'seed'; key: string; place: string | null; category_id: number; category_name: string; parent_name: string | null; hit_count: number }
export interface TxFilter { from?: string | null; to?: string | null; account_id?: number | null; category_id?: number | null; status?: Status | null; text?: string | null; statement_id?: number | null }
export interface TxRow { id: number; account_id: number; account_kind: AccountKind; statement_number: number; posted_date: string; tx_date: string; kind: string; amount_cents: number; orig_amount_cents: number | null; orig_currency: string | null; merchant_raw: string; place: string | null; counterparty_name: string | null; counterparty_iban: string | null; category_id: number | null; category_name: string | null; parent_name: string | null; status: Status; source: string; raw_block: string }
export interface ImportReport { path: string; status: ImportStatus; accountLabel: string | null; accountKind: AccountKind | null; ibanMasked: string | null; iban: string | null; statementNumber: number | null; periodStart: string | null; periodEnd: string | null; inserted: number; duplicates: number; checksum: Checksum | null; warnings: string[]; message: string | null }
export interface Summary { income_cents: number; expense_cents: number; transfer_cents: number; net_cents: number; unassigned_count: number; suggested_count: number; by_category: { category_id: number; name: string; parent_name: string | null; cents: number }[]; by_month: { month: string; income_cents: number; expense_cents: number }[]; by_month_category: { month: string; category_id: number; name: string; cents: number }[]; top_merchants: { merchant: string; cents: number; count: number }[] }
export interface BadChecksum { statement_id: number; number: number; account_label: string; off_by_cents: number }

export const api = {
  importStatements: (paths: string[]) => invoke<ImportReport[]>('import_statements', { paths }),
  importWithPassword: (path: string, password: string, remember: boolean) => invoke<ImportReport>('import_with_password', { path, password, remember }),
  listAccounts: () => invoke<Account[]>('list_accounts'),
  saveAccount: (iban: string, kind: AccountKind, label: string) => invoke<Account>('save_account', { iban, kind, label }),
  clearPassword: (accountId: number) => invoke<void>('clear_password', { accountId }),
  listCategories: () => invoke<Category[]>('list_categories'),
  saveCategory: (id: number | null, parentId: number | null, name: string, kind: CategoryKind) => invoke<Category>('save_category', { id, parentId, name, kind }),
  archiveCategory: (id: number) => invoke<void>('archive_category', { id }),
  listRules: () => invoke<RuleView[]>('list_rules'),
  deleteRule: (id: number) => invoke<void>('delete_rule', { id }),
  listTransactions: (filter: TxFilter) => invoke<TxRow[]>('list_transactions', { filter }),
  assign: (ids: number[], categoryId: number, applyToMatching: boolean) => invoke<{ updated: number; rules_created: number; skipped_transfers: number }>('assign', { ids, categoryId, applyToMatching }),
  confirm: (ids: number[]) => invoke<number>('confirm', { ids }),
  summary: (from: string | null, to: string | null, accountId: number | null) => invoke<Summary>('summary', { from, to, accountId }),
  exportCsv: (filter: TxFilter, path: string) => invoke<number>('export_csv', { filter, path }),
  badChecksums: () => invoke<BadChecksum[]>('bad_checksums'),
  dataDir: () => invoke<string>('data_dir'),
}

export function formatEur(cents: number): string {
  const sign = cents < 0 ? '-' : ''
  const abs = Math.abs(cents)
  const whole = Math.floor(abs / 100).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ' ')
  return `${sign}${whole},${(abs % 100).toString().padStart(2, '0')} €`
}
