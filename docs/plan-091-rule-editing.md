# Plan 091 rule editing backend

This slice implements the backend contract for accepted point 14 of Abakus
0.2.0. The store, Tauri command and TypeScript API are ready. The rules table
in `src/screens/Categories.tsx` now edits and deletes through this contract:
each row's category cell is a `CategoryPicker` in assignment mode that calls
`update_rule_category` directly, and the delete button opens a dialog that
previews `open_rule_references`/`open_classification_changes` before calling
`delete_rule`.

## Behavior

`Store::update_rule_category` accepts all persisted rule kinds: `seed`,
`exact`, `merchant` and `counterparty_account`. It validates the target before
opening the write transaction. The target must exist, be active, be
non-system, and be either a child category or a root with no active children.
The SQL update repeats the allowed-kind condition. A failed rule update or
open-row reclassification rolls back the full operation.

After an edit, all `suggested` and `unassigned` rows run through the existing
classifier and its existing precedence. Confirmed and transfer assignments
keep their category, status and source. The `updated` field in
`RuleRedirectOutcome` is the number of open rows that finish as `suggested`
and point to the edited rule. It is not the number of all rule references.

`Store::rule_delete_preview` evaluates every current open row against the
same classifier with the selected rule removed. It has no write side effects.
It returns two counts:

- `open_rule_references`: open rows whose current `rule_id` is the selected
  rule.
- `open_classification_changes`: open rows whose resulting status or category
  would differ after deletion.

A row can contribute to the first count and not the second. This happens when
another rule gives the same status and category. Its provenance can still
change. The preview excludes confirmed and transfer rows because deletion does
not reclassify them.

`Store::delete_rule` now rejects an unknown rule. It clears foreign-key
references, deletes the rule and reclassifies open rows in one transaction.
Confirmed and transfer rows retain category, status and source. A confirmed
row that pointed to the deleted rule has `rule_id = NULL` afterward. Its source
label remains historical text and must not be presented as a live rule link.

No schema migration is required.

## Additive API inventory

- Rust type `RuleDeletePreview`.
- Rust field `RuleDeletePreview.rule_id: i64`.
- Rust field `RuleDeletePreview.open_rule_references: usize`.
- Rust field `RuleDeletePreview.open_classification_changes: usize`.
- Rust method `Store::rule_delete_preview(rule_id)`.
- Tauri command `rule_delete_preview`.
- Tauri command argument `ruleId: number` mapped to Rust `rule_id`.
- TypeScript interface `RuleDeletePreview`.
- TypeScript field `RuleDeletePreview.rule_id: number`.
- TypeScript field `RuleDeletePreview.open_rule_references: number`.
- TypeScript field `RuleDeletePreview.open_classification_changes: number`.
- TypeScript API method `categoryApi.previewRuleDelete(ruleId)`.
- Rule change: `Store::update_rule_category` also accepts `exact`, `merchant`
  and `counterparty_account` targets.
- Rule change: deleting an unknown rule returns `UnknownRule`.
- Rule change: rule deletion is atomic across reference detachment, rule
  deletion and open-row reclassification.

## Verification scope

The focused store tests use parsed synthetic statements, in-memory databases
and temporary SQLite files. They cover learned rule kinds, precedence, valid
and invalid targets, rollback, fallback rules, confirmed rows, transfer rows
and unknown rules. `src-tauri/tests/category_json.rs` checks the exact command
argument and response field names.

Frontend status: shipped in `src/screens/Categories.tsx` and covered by
`src/screens/Categories.test.tsx` ("Categories rule editing (point 14)").
Native/visual acceptance is controller/independently-routed-verifier
territory, not this note's.
