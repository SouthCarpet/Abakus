use crate::{Result, Store, StoreError};
use parser::fold::fold;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum CategoryKind { Expense, Income }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct Category { pub id: i64, pub parent_id: Option<i64>, pub name: String, pub kind: CategoryKind, pub sort: i64, pub system: bool, pub archived: bool }

/// Section 9 wire contract: reparent + kind change with explicit acknowledgement.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryUpdateRequest {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub kind: CategoryKind,
    pub acknowledge_kind_change: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CategoryUpdatePreview {
    pub effective_kind: CategoryKind,
    pub affected_categories: usize,
    pub transaction_count: usize,
    pub confirmed_count: usize,
    pub requires_confirmation: bool,
}

const NAME_MAX_CHARS: usize = 100;

/// Trims, then rejects empty, over-length and control-character (incl. NUL,
/// `\n`, `\t`) names. Slash stays legal: current seed names contain it.
fn validate_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() { return Err(StoreError::Parse("Názov kategórie je prázdny.".into())); }
    if trimmed.chars().count() > NAME_MAX_CHARS { return Err(StoreError::Parse(format!("Názov kategórie je príliš dlhý. Limit je {NAME_MAX_CHARS} znakov."))); }
    if trimmed.chars().any(|c| c.is_control()) { return Err(StoreError::Parse("Názov kategórie obsahuje nepovolený znak.".into())); }
    Ok(trimmed.to_string())
}

impl CategoryKind { pub fn as_str(self) -> &'static str { match self { Self::Expense => "expense", Self::Income => "income" } } pub fn parse(s: &str) -> Self { if s == "income" { Self::Income } else { Self::Expense } } }

impl Store {
    pub(crate) fn seed_categories(&mut self) -> Result<()> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match self.seed_categories_tx() {
            Ok(()) => { self.conn.execute_batch("COMMIT")?; Ok(()) }
            Err(e) => { let _ = self.conn.execute_batch("ROLLBACK"); Err(e) }
        }
    }

    /// Section 9 contract: no BEGIN/COMMIT of its own, so A's v4 migration can
    /// call this inside its own open transaction. `seed_categories` above is
    /// the legacy standalone entry point, kept for `Store::init`'s fresh-db path.
    pub(crate) fn seed_categories_tx(&mut self) -> Result<()> {
        for (i, c) in crate::seed_categories::SEED.iter().enumerate() {
            self.conn.execute("INSERT INTO categories (parent_id, name, kind, sort, system) VALUES (NULL, ?1, ?2, ?3, ?4)", rusqlite::params![c.name, c.kind, i as i64, c.system])?;
            let parent = self.conn.last_insert_rowid();
            for (j, sub) in c.subs.iter().enumerate() { self.conn.execute("INSERT INTO categories (parent_id, name, kind, sort, system) VALUES (?1, ?2, ?3, ?4, 0)", rusqlite::params![parent, sub, c.kind, j as i64])?; }
        }
        Ok(())
    }

    pub fn list_categories(&self) -> Result<Vec<Category>> {
        let mut st = self.conn.prepare("SELECT id, parent_id, name, kind, sort, system, archived FROM categories ORDER BY parent_id IS NOT NULL, sort, id")?;
        let rows = st.query_map([], |r| Ok(Category { id: r.get(0)?, parent_id: r.get(1)?, name: r.get(2)?, kind: CategoryKind::parse(&r.get::<_, String>(3)?), sort: r.get(4)?, system: r.get::<_, i64>(5)? != 0, archived: r.get::<_, i64>(6)? != 0 }))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
    /// "Parent/sub" (split on the last slash) or "Parent".
    pub fn category_by_path(&self, path: &str) -> Result<Option<i64>> {
        let (parent, sub) = match path.rsplit_once('/') { Some((p, s)) if self.top_level_id(p)?.is_some() => (p, Some(s)), _ => (path, None) };
        let Some(pid) = self.top_level_id(parent)? else { return Ok(None) };
        match sub { None => Ok(Some(pid)), Some(s) => Ok(self.conn.query_row("SELECT id FROM categories WHERE parent_id = ?1 AND name = ?2", rusqlite::params![pid, s], |r| r.get(0)).ok()) }
    }
    fn top_level_id(&self, name: &str) -> Result<Option<i64>> { Ok(self.conn.query_row("SELECT id FROM categories WHERE parent_id IS NULL AND name = ?1", [name], |r| r.get(0)).ok()) }
    pub fn cash_category_id(&self) -> Result<Option<i64>> { self.category_by_path("Hotovosť/bankomat") }
    pub fn unassigned_category_id(&self) -> Result<i64> { self.category_by_path("Nezaradené")?.ok_or_else(|| StoreError::Db("Nezaradené missing".into())) }

    pub(crate) fn category_row(&self, id: i64) -> Result<Option<Category>> { Ok(self.list_categories()?.into_iter().find(|c| c.id == id)) }
    pub(crate) fn children_of(&self, parent_id: i64) -> Result<Vec<Category>> { Ok(self.list_categories()?.into_iter().filter(|c| c.parent_id == Some(parent_id)).collect()) }

    /// Create (`id: None`) or rename/reparent (`id: Some`) a category. The
    /// update path now delegates to the fully validated `update_category`
    /// with `acknowledge_kind_change: false`; legacy callers never move a
    /// category between kinds, so this never trips the acknowledgement gate.
    pub fn save_category(&mut self, id: Option<i64>, parent_id: Option<i64>, name: &str, kind: CategoryKind) -> Result<Category> {
        match id {
            Some(i) => self.update_category(&CategoryUpdateRequest { id: i, parent_id, name: name.to_string(), kind, acknowledge_kind_change: false }),
            None => self.create_category(parent_id, name, kind),
        }
    }

    fn create_category(&mut self, parent_id: Option<i64>, name: &str, kind: CategoryKind) -> Result<Category> {
        let name = validate_name(name)?;
        if let Some(pid) = parent_id {
            let parent = self.category_row(pid)?.ok_or(StoreError::UnknownCategory { id: pid })?;
            if parent.archived { return Err(StoreError::Parse("Archivovaná kategória nemôže byť rodičom.".into())); }
        }
        self.check_duplicate_name_for_create(parent_id, &name)?;
        self.conn.execute("INSERT INTO categories (parent_id, name, kind, sort) VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(sort),0)+1 FROM categories WHERE parent_id IS ?1))", rusqlite::params![parent_id, name, kind.as_str()])?;
        let id = self.conn.last_insert_rowid();
        self.category_row(id)?.ok_or_else(|| StoreError::Db("category vanished".into()))
    }

    fn check_duplicate_name_for_create(&self, parent_id: Option<i64>, name: &str) -> Result<()> {
        let folded = fold(name);
        let dup = self.list_categories()?.into_iter().any(|c| c.parent_id == parent_id && fold(&c.name) == folded);
        if dup { return Err(StoreError::Parse("Kategória s týmto názvom na tejto úrovni už existuje.".into())); }
        Ok(())
    }

    /// Archives a category (soft delete); refused for system categories.
    pub fn archive_category(&mut self, id: i64) -> Result<()> {
        let n = self.conn.execute("UPDATE categories SET archived = 1 WHERE id = ?1 AND system = 0", [id])?;
        if n == 0 { return Err(StoreError::Parse("systémovú kategóriu nemožno archivovať".into())); }
        Ok(())
    }

    /// Read-only: what `update_category` would do, for the UI's confirmation
    /// step. Recomputed fresh at save time too, so a concurrently added
    /// transaction is never missed (C02).
    pub fn category_update_preview(&self, req: &CategoryUpdateRequest) -> Result<CategoryUpdatePreview> {
        let target = self.category_row(req.id)?.ok_or(StoreError::UnknownCategory { id: req.id })?;
        let (_, new_kind) = self.resolve_parent_and_kind(&target, req)?;
        self.kind_change_effects(&target, new_kind)
    }

    /// Section 9: reparent + rename + kind change, one atomic write. System
    /// categories are fully immutable; a protected system subtree (currently
    /// Hotovosť/bankomat) keeps its permitted rename/archive but refuses
    /// reparenting and kind changes. A kind change with affected transactions
    /// requires `acknowledge_kind_change`, revalidated here (not trusted from
    /// a stale client-side preview).
    pub fn update_category(&mut self, req: &CategoryUpdateRequest) -> Result<Category> {
        let target = self.category_row(req.id)?.ok_or(StoreError::UnknownCategory { id: req.id })?;
        if target.system { return Err(StoreError::Parse("systémovú kategóriu nemožno premenovať".into())); }
        let name = validate_name(&req.name)?;
        let (new_parent_id, new_kind) = self.resolve_parent_and_kind(&target, req)?;
        self.check_protected_subtree(&target, new_parent_id, new_kind)?;
        let preview = self.kind_change_effects(&target, new_kind)?;
        if preview.requires_confirmation && !req.acknowledge_kind_change {
            return Err(StoreError::Parse("Zmena druhu kategórie ovplyvní históriu transakcií a vyžaduje potvrdenie.".into()));
        }
        self.check_duplicate_name(&target, new_parent_id, &name)?;
        self.apply_category_update(&target, new_parent_id, &name, new_kind)
    }

    /// Parent kind wins for a child; a root (staying or becoming one) keeps
    /// the request's own kind, defaulting to its old kind in the UI. Also
    /// refuses self/descendant/missing/archived/system parent targets and a
    /// third tree level (§9: the target parent must itself be top-level, and
    /// a root with children cannot move under another root).
    fn resolve_parent_and_kind(&self, target: &Category, req: &CategoryUpdateRequest) -> Result<(Option<i64>, CategoryKind)> {
        // No reparenting requested: skip every parent-target check below
        // (self/descendant/missing/archived/system/top-level), since restating
        // the category's own current parent is not "moving into" anything. A
        // root may still freely change its own kind; a child's kind already
        // always mirrors its unchanged parent's, by the same invariant every
        // prior update maintains.
        if req.parent_id == target.parent_id {
            return Ok((target.parent_id, if target.parent_id.is_none() { req.kind } else { target.kind }));
        }
        match req.parent_id {
            None => Ok((None, req.kind)),
            Some(pid) => Ok((Some(pid), self.validated_reparent_target(target, pid)?.kind)),
        }
    }

    /// Self/missing/archived/system/non-top-level/would-create-a-third-level
    /// refusals for an ACTUAL reparent request (the caller already excluded
    /// "parent unchanged"). A non-top-level target, including one of
    /// `target`'s own children, also covers the "descendant" refusal for
    /// this 2-level model.
    fn validated_reparent_target(&self, target: &Category, pid: i64) -> Result<Category> {
        if pid == target.id { return Err(StoreError::Parse("Kategória nemôže byť vlastným rodičom.".into())); }
        let parent = self.category_row(pid)?.ok_or(StoreError::UnknownCategory { id: pid })?;
        if parent.archived { return Err(StoreError::Parse("Archivovaná kategória nemôže byť rodičom.".into())); }
        if parent.system { return Err(StoreError::Parse("Systémová kategória nemôže byť rodičom.".into())); }
        if parent.parent_id.is_some() { return Err(StoreError::Parse("Nadradená kategória musí byť najvyššej úrovne.".into())); }
        if target.parent_id.is_none() && !self.children_of(target.id)?.is_empty() {
            return Err(StoreError::Parse("Kategóriu s podkategóriami nemožno presunúť pod inú kategóriu.".into()));
        }
        Ok(parent)
    }

    fn check_protected_subtree(&self, target: &Category, new_parent_id: Option<i64>, new_kind: CategoryKind) -> Result<()> {
        let Some(cur_parent_id) = target.parent_id else { return Ok(()) };
        let Some(cur_parent) = self.category_row(cur_parent_id)? else { return Ok(()) };
        if !cur_parent.system { return Ok(()); }
        if new_parent_id != target.parent_id || new_kind != target.kind {
            return Err(StoreError::Parse("Kategóriu v chránenej systémovej vetve nemožno presunúť ani zmeniť jej druh.".into()));
        }
        Ok(())
    }

    /// `TxFilter.category_id` already matches the category OR its children
    /// (see `query::where_clause`), so this reads exactly the affected subtree:
    /// just the category for a leaf, or the category plus its children for a
    /// root, with no separate id-collection step.
    fn kind_change_effects(&self, target: &Category, new_kind: CategoryKind) -> Result<CategoryUpdatePreview> {
        let changed = new_kind != target.kind;
        let affected_categories = if changed { 1 + self.children_of(target.id)?.len() } else { 0 };
        let (transaction_count, confirmed_count) = if changed {
            let rows = self.list_transactions(&crate::TxFilter { category_id: Some(target.id), ..Default::default() })?;
            (rows.len(), rows.iter().filter(|r| r.status == rules::Status::Confirmed).count())
        } else {
            (0, 0)
        };
        Ok(CategoryUpdatePreview { effective_kind: new_kind, affected_categories, transaction_count, confirmed_count, requires_confirmation: changed && transaction_count > 0 })
    }

    /// Siblings compared via `parser::fold`, including archived ones and
    /// excluding the edited row. An edit that leaves the folded name and
    /// parent unchanged is allowed even over an existing legacy duplicate
    /// (never rewritten); creating or moving into one is refused.
    fn check_duplicate_name(&self, target: &Category, new_parent_id: Option<i64>, name: &str) -> Result<()> {
        if fold(name) == fold(&target.name) && new_parent_id == target.parent_id { return Ok(()); }
        let folded = fold(name);
        let dup = self.list_categories()?.into_iter().any(|c| c.id != target.id && c.parent_id == new_parent_id && fold(&c.name) == folded);
        if dup { return Err(StoreError::Parse("Kategória s týmto názvom na tejto úrovni už existuje.".into())); }
        Ok(())
    }

    fn apply_category_update(&mut self, target: &Category, new_parent_id: Option<i64>, name: &str, new_kind: CategoryKind) -> Result<Category> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match self.apply_category_update_tx(target, new_parent_id, name, new_kind) {
            Ok(cat) => { self.conn.execute_batch("COMMIT")?; Ok(cat) }
            Err(e) => { let _ = self.conn.execute_batch("ROLLBACK"); Err(e) }
        }
    }

    /// Sort is preserved unless the category actually moves, in which case it
    /// is appended after its new siblings. A root's kind change propagates to
    /// every child, including archived ones, in the same transaction.
    fn apply_category_update_tx(&mut self, target: &Category, new_parent_id: Option<i64>, name: &str, new_kind: CategoryKind) -> Result<Category> {
        let sort = if new_parent_id == target.parent_id {
            target.sort
        } else {
            self.conn.query_row("SELECT COALESCE(MAX(sort),0)+1 FROM categories WHERE parent_id IS ?1", [new_parent_id], |r| r.get(0))?
        };
        let n = self.conn.execute("UPDATE categories SET name = ?2, parent_id = ?3, kind = ?4, sort = ?5 WHERE id = ?1 AND system = 0", rusqlite::params![target.id, name, new_parent_id, new_kind.as_str(), sort])?;
        if n == 0 { return Err(StoreError::UnknownCategory { id: target.id }); }
        if new_parent_id.is_none() && new_kind != target.kind {
            self.conn.execute("UPDATE categories SET kind = ?2 WHERE parent_id = ?1", rusqlite::params![target.id, new_kind.as_str()])?;
        }
        self.category_row(target.id)?.ok_or_else(|| StoreError::Db("category vanished".into()))
    }
}
