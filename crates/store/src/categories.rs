use crate::{Result, Store, StoreError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum CategoryKind { Expense, Income }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct Category { pub id: i64, pub parent_id: Option<i64>, pub name: String, pub kind: CategoryKind, pub sort: i64, pub system: bool, pub archived: bool }

impl CategoryKind { pub fn as_str(self) -> &'static str { match self { Self::Expense => "expense", Self::Income => "income" } } pub fn parse(s: &str) -> Self { if s == "income" { Self::Income } else { Self::Expense } } }

impl Store {
    pub(crate) fn seed_categories(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        for (i, c) in crate::seed_categories::SEED.iter().enumerate() {
            tx.execute("INSERT INTO categories (parent_id, name, kind, sort, system) VALUES (NULL, ?1, ?2, ?3, ?4)", rusqlite::params![c.name, c.kind, i as i64, c.system])?;
            let parent = tx.last_insert_rowid();
            for (j, sub) in c.subs.iter().enumerate() { tx.execute("INSERT INTO categories (parent_id, name, kind, sort, system) VALUES (?1, ?2, ?3, ?4, 0)", rusqlite::params![parent, sub, c.kind, j as i64])?; }
        }
        tx.commit()?; Ok(())
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

    /// Create (`id: None`) or rename/reparent (`id: Some`) a category. System
    /// categories are protected from the `UPDATE` by `AND system = 0`.
    pub fn save_category(&mut self, id: Option<i64>, parent_id: Option<i64>, name: &str, kind: CategoryKind) -> Result<Category> {
        let name = name.trim();
        if name.is_empty() { return Err(StoreError::Parse("názov je prázdny".into())); }
        let id = match id {
            Some(i) => {
                let n = self.conn.execute("UPDATE categories SET name = ?2, parent_id = ?3 WHERE id = ?1 AND system = 0", rusqlite::params![i, name, parent_id])?;
                if n == 0 { return Err(StoreError::Parse("systémovú kategóriu nemožno premenovať".into())); }
                i
            }
            None => {
                self.conn.execute("INSERT INTO categories (parent_id, name, kind, sort) VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(sort),0)+1 FROM categories WHERE parent_id IS ?1))", rusqlite::params![parent_id, name, kind.as_str()])?;
                self.conn.last_insert_rowid()
            }
        };
        self.list_categories()?.into_iter().find(|c| c.id == id).ok_or_else(|| StoreError::Db("category vanished".into()))
    }

    /// Archives a category (soft delete); refused for system categories.
    pub fn archive_category(&mut self, id: i64) -> Result<()> {
        let n = self.conn.execute("UPDATE categories SET archived = 1 WHERE id = ?1 AND system = 0", [id])?;
        if n == 0 { return Err(StoreError::Parse("systémovú kategóriu nemožno archivovať".into())); }
        Ok(())
    }
}
