# 0.1.2 category workflows and Spotify repair

Lane C of the recurring/category extension (worktree `080-recurring-categories`,
branch `feature/080-recurring-categories`), executing
`A:/projects-vault/animus/data/agent-runs/abakus-012-20260907/recurring-contract.md`
section 9 and `recurring-categories-maker.md`. Covers category move/kind-change
with explicit acknowledgement, name validation, seed rule provenance and
redirect, the Spotify fresh-seed/repair split, and the shared `CategoryDialog`/
`CategoryPicker` creation seam. No edits to A's recurring store/migration
files, B's recurring UI, `src/api.ts`, `TxFilter`/`TxRow`, or version numbers.

## What shipped

### Category move, rename and kind change (`crates/store/src/categories.rs`)

- `CategoryUpdateRequest { id, parent_id, name, kind, acknowledge_kind_change }`
  and `CategoryUpdatePreview { effective_kind, affected_categories,
  transaction_count, confirmed_count, requires_confirmation }` (new public
  types, `Store::category_update_preview`/`Store::update_category`).
- **Parent kind wins.** A child's `kind` field is inert: the effective kind is
  always its (possibly new) parent's kind. A root (staying or becoming one)
  keeps the request's own kind; promoting a child to root defaults that field
  to the UI's own choice, editable, so it can retain the old kind by default.
- **Reparent validation** (`resolve_parent_and_kind` /
  `validated_reparent_target`): self, missing, archived, system, and
  non-top-level parent targets are all refused before any write. A
  non-top-level target also covers "descendant" for this two-level model (the
  only way a target could be a descendant of the edited category is to be one
  of its own children, which is never top-level). A root that already has
  children refuses moving under another root, so a third tree level can never
  be created. **A request that merely restates the category's own current
  parent skips all of this**: it is a rename/kind-only edit, not a move, and
  must not trip validation meant for reparenting (a bug caught by
  `c01_the_protected_hotovost_subtree_keeps_rename_but_refuses_move_and_kind_change`
  during development: the original code refused every edit of a child whose
  *existing* parent was System, because it re-validated that parent as if it
  were a fresh reparent target).
- **Protected system subtree** (`check_protected_subtree`): a category itself
  marked `system` is fully immutable (existing invariant, preserved). A
  non-system category whose *parent* is system (currently only
  `Hotovosť/bankomat`) keeps its existing permitted rename/archive, but
  refuses a move or a kind change, even with acknowledgement.
- **Kind-change acknowledgement** (`kind_change_effects`): `affected_categories`
  is 1 for a leaf/child, or `1 + child count` (any archived state included)
  when a root's kind changes, since propagation always cascades to every
  child. `transaction_count`/`confirmed_count` come from
  `list_transactions(&TxFilter { category_id: Some(target.id), .. })`, which
  already matches the category OR its children (`query::where_clause`), so no
  separate id-collection pass is needed. `requires_confirmation` is true only
  when the kind actually changes AND at least one transaction is affected; a
  no-op kind (or a kind change with zero linked transactions) needs no
  acknowledgement. The preview is recomputed fresh inside `update_category`
  itself and revalidated against `acknowledge_kind_change` there, never
  trusted from a client-cached preview, so a transaction added between preview
  and save is still counted.
- **Names**: trimmed, non-empty, ≤100 Unicode scalar values, no control
  characters (NUL and `\n`/`\t` included via `char::is_control()`); slash
  stays legal since it is not a control character. Siblings are compared with
  `parser::fold`, including root siblings and archived ones, excluding the
  edited id. An edit whose folded name AND parent both stay exactly what they
  already were is allowed even over a pre-existing legacy duplicate (never
  rewritten); creating a new duplicate, or moving/renaming into one, is
  refused.
- The legacy `Store::save_category(id, parent_id, name, kind)` signature is
  unchanged; for an existing `id` it now delegates internally to
  `update_category` with `acknowledge_kind_change: false`, so every legacy
  caller gets the same validated name/duplicate/protected-subtree checks for
  free. Creation (`id: None`) goes through the same name validation and
  duplicate check via a small dedicated `create_category` path.

### Seed rule provenance and redirect (`crates/store/src/rules_repo.rs`)

- `Store::seed_rule_for_transaction(transaction_id) -> Result<Option<RuleView>>`:
  the actual `transactions.rule_id` joined to a `match_kind = 'seed'` rule,
  never inferred from merchant text or category name. A missing transaction is
  an error; a transaction with no seed pointer (learned rule, or none at all)
  is `Ok(None)`.
- `Store::update_rule_category(rule_id, category_id) -> Result<RuleRedirectOutcome>`:
  limited to seed rules this release (refused for any other rule kind). The
  target must be a usable, non-system, non-archived category: a leaf, or a
  root with no *active* children (an archived child does not block it).
  Atomic: the whole operation, including a mid-write failure, rolls back.
  Reclassifies through the existing `reclassify_open` (which already
  implements full rule precedence), so it only ever touches `suggested`/
  `unassigned` rows for which the redirected rule genuinely wins; confirmed
  and transfer rows are left byte-for-byte unchanged. A higher-priority
  learned rule that already shadowed the seed rule for some row keeps
  shadowing it.

### Spotify fresh seed and legacy repair

- `crates/store/src/seed_categories.rs`: `Predplatné` now seeds a `Spotify`
  child alongside the existing `Apple` one.
- `crates/store/src/seed_rules.toml`: the `spotify` seed key now targets
  `Predplatné/Spotify` instead of `Predplatné/Apple`. A fresh database needs
  no repair at all: it is seeded correctly from these two files alone.
- `crates/store/src/seed_repair.rs` (new): `pub(crate) fn
  repair_spotify_seed_tx(&mut self) -> Result<()>`, for an EXISTING install
  seeded before this split. No `BEGIN`/`COMMIT` of its own: it is designed to
  run inside A's v4 migration transaction, the one integration point this lane
  does not own. It repairs only an exact legacy signature: a seed rule with
  key `spotify`, empty place, whose category is an active, non-system,
  expense category literally named `Apple`, whose parent is an active,
  non-system, expense category literally named `Predplatné`. Any deviation
  (deleted rule, rule already redirected elsewhere, renamed/reparented/
  archived tree, a learned rule shadowing it) fails that exact match and the
  function is a no-op: no complete reseed, no accidental new categories or
  rules on an unrelated minimal fixture. On a match it reuses a single
  unambiguous existing active expense "Spotify" sibling of `Apple`'s parent if
  one exists, creates one if none exists, and does nothing (preserving the
  legacy Apple assignment) if the match is ambiguous (more than one
  Spotify-like sibling) rather than forcing a pick. It then redirects the seed
  rule and reclassifies only its own open rows, so confirmed rows are
  untouched. It is idempotent by construction: once repaired, the legacy
  signature no longer matches, so a second call is a no-op.

  **Documented limit, stated per contract §9's own "narrow limit" clause**:
  the old schema records no customization flag, so an intentional user choice
  that happens to be byte-identical to the exact old seed (same rule row,
  same Apple target, unmodified) is indistinguishable from an untouched
  install and receives the repair like any other exact match. Every
  *distinguishable* user choice (a rename, a reparent, an archive, an
  explicit redirect elsewhere, a learned rule) is preserved.

### Frontend seam (`src/lib/category-api.ts`, `src/components/CategoryDialog.tsx`, `src/components/CategoryPicker.tsx`)

- `categoryApi` (new): `preview`, `update`, `seedRuleForTransaction`,
  `redirectRule`, wired to the four new Tauri commands
  (`category_update_preview`, `update_category`, `seed_rule_for_transaction`,
  `update_rule_category`).
- `CategoryDialog` (new, shared seam): `{open, initialParentId?, initialKind?,
  onClose, onCreated}`. Creation only, through the existing (now validated)
  `api.saveCategory`; calls `onCreated` only after real persistence. A failed
  create preserves the typed draft (name/parent/kind are not cleared on
  error, only when the dialog reopens). This is the exact component both
  `Categories.tsx`'s "+ kategória"/"+ podkategória" affordances and
  `Transactions.tsx`'s "Nová kategória..." picker option are meant to reuse;
  B's recurring editor (not yet published in this worktree) reuses it too.
- `CategoryPicker` gains an optional `onCreate?: () => void`. Only assignment
  mode offers a trailing "Nová kategória..." option, and picking it calls
  `onCreate()` without ever sending a sentinel id through `onChange`.

### Categories screen: edit → preview → confirm → update

`Categories.tsx` gains an "Upraviť" action per category (alongside the
existing inline rename input and "Archivovať"), opening `CategoryEditDialog`:
name/parent/kind form → `categoryApi.preview` → if `requires_confirmation`,
show the affected/transaction/confirmed counts and require an explicit
"Potvrdiť zmenu druhu" click (a "Zrušiť" here makes no write and returns to
the form) → `categoryApi.update` → refresh the tree and rules. A save that
needs no acknowledgement updates directly.

### Transactions screen: seed redirect and category creation during assignment

- An expanded, non-transfer transaction row calls
  `categoryApi.seedRuleForTransaction`; if it is classified by an actual seed
  rule, a redirect picker appears with a plain-language line naming the
  current seed category. Picking a target calls `categoryApi.redirectRule`
  and refreshes the row list.
- Both the per-row picker and the bulk-assign picker now offer "Nová
  kategória...", opening one shared `CategoryDialog`. In the bulk context,
  the created category only *preselects* the bulk picker's own value; the
  existing "Priradiť" button remains the explicit assignment action. In the
  single-row context (which has no separate confirm button: selecting a
  category in that picker already *is* the assignment, before this change
  too), the created category is assigned to that one row immediately after
  creation succeeds; a subsequent assignment failure never re-creates the
  category; retrying is picking the now-available category again.
- A place marked for B: the not-yet-published `RecurringTransactionAction`
  mounts in the same expanded detail, right after `NoteEditor`, per the
  contract's exact integration line (left as a comment in
  `TransactionRow`, `src/screens/Transactions.tsx`).

### Test-only fixture correction (`src-tauri/src/commands.rs`)

`export_csv_counts_records_not_lines_when_a_field_has_an_embedded_newline`
used to build its multiline field by putting `"Multi\nline"` into a category
name. Category names now reject control characters (`\n` included), so that
construction is no longer legal through the real public API. The test now
puts the same literal multiline text into `save_transaction_note` instead
(already documented in `summary::csv_quote`'s own doc comment as "the first
multiline field this function ever sees"), keeping the exact same CSV
assertions (`written.contains("Multi\nline")`, record-count vs.
line-count divergence) through real, still-public, still-synthetic
persistence. No production command changed.

## New tests

- `crates/store/src/seed_repair.rs` (`#[cfg(test)] mod tests`, 10 cases):
  `repair_spotify_seed_tx` is `pub(crate)` by contract (only ever called from
  A's not-yet-integrated v4 migration transaction in this isolated worktree),
  so it cannot be called from an external `tests/` crate; these are same-crate
  unit tests instead, covering the exact-match repair, idempotent reopen,
  open-vs-confirmed reclassification, and every preservation case (deleted
  rule, redirected rule, archived/renamed Apple or Predplatné, an ambiguous
  existing Spotify sibling, a single unambiguous one, a minimal fixture with
  no legacy signature at all).
- `crates/store/tests/spotify_repair.rs` (3 cases): the public-boundary half.
  A fresh `Store::open_in_memory()` already has `Predplatné/Spotify` and
  the `spotify` seed rule pointing at it, through nothing but the public API.
- `crates/store/tests/category_workflows.rs` (15 cases, mapped to acceptance
  IDs C01/C02/C03/C05): move, promote-to-root, refuse-reparent-with-children,
  self/descendant/missing/archived/system parent refusals, the protected
  Hotovosť subtree, kind-change preview/ack/propagation with exact counts,
  name validation bounds, folded-duplicate refusal vs. an unchanged legacy
  edit (via a real file-backed fixture, same `rusqlite::Connection` pattern as
  `tests/migration.rs`, since the validated create path can no longer
  construct one), seed rule provenance, and redirect (including the
  active-vs-archived-children target rule and the seed-only restriction).
  Fixtures go through the real parser/import/classify path
  (`import_statement`, `assign`, `confirm`) wherever possible, per the local
  acceptance rule against calling production logic to compute expected
  values.
- `src-tauri/tests/category_json.rs` (8 cases): the wire-contract drift gate
  for the four new commands and their request/response shapes, same pattern
  as `commands_json.rs`.
- `src/components/CategoryDialog.test.tsx` (5 cases), `CategoryPicker.test.tsx`
  (+3 cases for `onCreate`), `screens/Categories.test.tsx` (+3 cases for the
  edit/preview/confirm flow).

## Commands run

```
cargo test --jobs 4 -p store --lib                     # 36 passed (incl. 10 seed_repair)
cargo test --jobs 4 -p store --test category_workflows # 14 passed
cargo test --jobs 4 -p store --test spotify_repair     # 3 passed
cargo test --jobs 4 --workspace                        # all green, see lane report
cargo clippy --jobs 4 --workspace --all-targets -- -D warnings
py -m lizard -l rust -C 12 -L 1000 -a 1000 -w crates/store/src src-tauri/src   # clean
node node_modules/vitest/vitest.mjs run                # 238 passed
node node_modules/typescript/bin/tsc -b --pretty false # clean
node node_modules/eslint/bin/eslint.js src             # clean
```

## Honest remaining limits

- `repair_spotify_seed_tx` has no live call site in this isolated worktree
  (A owns wiring it into `migrate.rs`'s v4 transaction). It is exhaustively
  unit-tested directly and the plain (non-test) build is exempted from
  `dead_code` for exactly this reason (`#![allow(dead_code)]`,
  `crates/store/src/seed_repair.rs`, documented inline). End-to-end proof
  through `Store::open` on a real legacy file is A's integration seam, not a
  silent gap.
- The frontend's redirect and seed-rule-provenance UI (`SeedRuleRedirect` in
  `Transactions.tsx`) is covered by its own component logic and by the
  backend's `category_json`/`category_workflows` wire and behavior tests, but
  has no dedicated `SeedRuleRedirect`-specific component test in this pass;
  it is exercised indirectly through the existing `Transactions.tsx`
  integration tests' render tree. Native/visual acceptance (U05/U06/U07) is
  controller/independently-routed-verifier territory per the acceptance
  rubric, not this lane's.
