use crate::{Category, Result, Store, StoreError};
use rusqlite::params_from_iter;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryDeleteItem {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryDeletePreview {
    pub category_id: i64,
    pub affected_categories: Vec<CategoryDeleteItem>,
    pub transaction_count: usize,
    pub confirmed_count: usize,
    pub rule_count: usize,
    pub rule_source_count: usize,
    pub recurring_member_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryDeleteRequest {
    pub preview: CategoryDeletePreview,
}

impl Store {
    /// Returns the exact subtree and every direct data effect that a confirmed
    /// deletion will have. A transfer or a rule reference outside this subtree
    /// is inconsistent state and stops both preview and apply.
    pub fn category_delete_preview(&self, category_id: i64) -> Result<CategoryDeletePreview> {
        let owns_transaction = self.conn.is_autocommit();
        if owns_transaction {
            self.conn.execute_batch("BEGIN")?;
        }
        let result = self.category_delete_preview_tx(category_id);
        if owns_transaction {
            return finish_read(&self.conn, result);
        }
        result
    }

    fn category_delete_preview_tx(&self, category_id: i64) -> Result<CategoryDeletePreview> {
        let categories = self.deletion_subtree(category_id)?;
        self.check_deletion_protection(category_id, &categories)?;
        let ids: Vec<i64> = categories.iter().map(|category| category.id).collect();
        self.check_deletion_references(&ids)?;

        let transaction_count = self.count_where_ids("transactions", "category_id", &ids)?;
        let confirmed_count = self.count_transactions_with_status(&ids, "confirmed")?;
        let rule_count = self.count_where_ids("rules", "category_id", &ids)?;
        let rule_source_count = self.count_rule_sources(&ids)?;
        let recurring_member_count = self.count_recurring_members(&ids)?;

        Ok(CategoryDeletePreview {
            category_id,
            affected_categories: categories
                .into_iter()
                .map(CategoryDeleteItem::from)
                .collect(),
            transaction_count,
            confirmed_count,
            rule_count,
            rule_source_count,
            recurring_member_count,
        })
    }

    /// Deletes only after the preview is recomputed under the same write lock
    /// and still matches byte-for-byte at the data-model level. All assigned
    /// rows, including confirmed rows, become the existing unassigned state.
    pub fn delete_category(
        &mut self,
        request: &CategoryDeleteRequest,
    ) -> Result<CategoryDeletePreview> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        let result = self.delete_category_tx(request);
        finish_write(&self.conn, result)
    }

    fn delete_category_tx(
        &mut self,
        request: &CategoryDeleteRequest,
    ) -> Result<CategoryDeletePreview> {
        let current = self.category_delete_preview(request.preview.category_id)?;
        if current != request.preview {
            return Err(StoreError::Parse(
                "Kategória sa od vytvorenia náhľadu zmenila. Vytvorte nový náhľad a potvrďte ho."
                    .into(),
            ));
        }

        let ids: Vec<i64> = current
            .affected_categories
            .iter()
            .map(|category| category.id)
            .collect();
        let placeholders = placeholders(ids.len());
        let moved = self.conn.execute(
            &format!(
                "UPDATE transactions SET category_id = NULL, status = 'unassigned', rule_id = NULL, source = 'none' \
                 WHERE category_id IN ({placeholders}) AND status <> 'transfer'"
            ),
            params_from_iter(ids.iter()),
        )?;
        if moved != current.transaction_count {
            return Err(StoreError::Db(
                "počet presunutých transakcií sa nezhoduje s náhľadom".into(),
            ));
        }

        let deleted_rules = self.conn.execute(
            &format!("DELETE FROM rules WHERE category_id IN ({placeholders})"),
            params_from_iter(ids.iter()),
        )?;
        if deleted_rules != current.rule_count {
            return Err(StoreError::Db(
                "počet odstránených pravidiel sa nezhoduje s náhľadom".into(),
            ));
        }

        self.delete_categories_leaf_first(&current.affected_categories)?;
        Ok(current)
    }

    fn deletion_subtree(&self, category_id: i64) -> Result<Vec<Category>> {
        let categories = self.list_categories()?;
        if !categories.iter().any(|category| category.id == category_id) {
            return Err(StoreError::UnknownCategory { id: category_id });
        }

        let mut ids = HashSet::from([category_id]);
        loop {
            let before = ids.len();
            for category in &categories {
                if category
                    .parent_id
                    .is_some_and(|parent_id| ids.contains(&parent_id))
                {
                    ids.insert(category.id);
                }
            }
            if ids.len() == before {
                break;
            }
        }
        Ok(categories
            .into_iter()
            .filter(|category| ids.contains(&category.id))
            .collect())
    }

    fn check_deletion_protection(&self, category_id: i64, subtree: &[Category]) -> Result<()> {
        if subtree.iter().any(|category| category.system) {
            return Err(StoreError::Parse(
                "Systémovú kategóriu nemožno odstrániť.".into(),
            ));
        }

        let categories = self.list_categories()?;
        let mut current_id = category_id;
        let mut visited = HashSet::new();
        while visited.insert(current_id) {
            let current = categories
                .iter()
                .find(|category| category.id == current_id)
                .ok_or(StoreError::UnknownCategory { id: current_id })?;
            let Some(parent_id) = current.parent_id else {
                return Ok(());
            };
            let parent = categories
                .iter()
                .find(|category| category.id == parent_id)
                .ok_or_else(|| StoreError::Parse("Kategória má neplatného rodiča.".into()))?;
            if parent.system {
                return Err(StoreError::Parse(
                    "Kategóriu v chránenej systémovej vetve nemožno odstrániť.".into(),
                ));
            }
            current_id = parent_id;
        }
        Err(StoreError::Parse(
            "Kategória má cyklickú rodičovskú väzbu.".into(),
        ))
    }

    fn check_deletion_references(&self, ids: &[i64]) -> Result<()> {
        if self.count_transactions_with_status(ids, "transfer")? > 0 {
            return Err(StoreError::Parse(
                "Kategória obsahuje prevod s neplatným odkazom. Odstránenie bolo zastavené.".into(),
            ));
        }

        let placeholders = placeholders(ids.len());
        let mut parameters = ids.to_vec();
        parameters.extend_from_slice(ids);
        let outside_references: i64 = self.conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM transactions t JOIN rules r ON r.id = t.rule_id \
                 WHERE r.category_id IN ({placeholders}) \
                 AND (t.category_id IS NULL OR t.category_id NOT IN ({placeholders}))"
            ),
            params_from_iter(parameters.iter()),
            |row| row.get(0),
        )?;
        if outside_references > 0 {
            return Err(StoreError::Parse(
                "Pravidlo kategórie odkazuje na transakciu mimo odstraňovanej vetvy. Odstránenie bolo zastavené.".into(),
            ));
        }
        Ok(())
    }

    fn count_where_ids(&self, table: &str, column: &str, ids: &[i64]) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM {table} WHERE {column} IN ({})",
                placeholders(ids.len())
            ),
            params_from_iter(ids.iter()),
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    fn count_transactions_with_status(&self, ids: &[i64], status: &str) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM transactions WHERE category_id IN ({}) AND status = ?",
                placeholders(ids.len())
            ),
            {
                let category_parameters = ids.iter().map(|id| id as &dyn rusqlite::ToSql);
                let status_parameter = std::iter::once(&status as &dyn rusqlite::ToSql);
                rusqlite::params_from_iter(category_parameters.chain(status_parameter))
            },
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    fn count_rule_sources(&self, ids: &[i64]) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM rule_sources sources JOIN rules ON rules.id = sources.rule_id \
                 WHERE rules.category_id IN ({})",
                placeholders(ids.len())
            ),
            params_from_iter(ids.iter()),
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    fn count_recurring_members(&self, ids: &[i64]) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            &format!(
                "SELECT COUNT(DISTINCT members.fingerprint) FROM recurring_members members \
                 JOIN transactions ON transactions.fingerprint = members.fingerprint \
                 WHERE transactions.category_id IN ({})",
                placeholders(ids.len())
            ),
            params_from_iter(ids.iter()),
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    fn delete_categories_leaf_first(&self, categories: &[CategoryDeleteItem]) -> Result<()> {
        let mut remaining = categories.to_vec();
        while !remaining.is_empty() {
            let Some(index) = remaining.iter().position(|candidate| {
                !remaining
                    .iter()
                    .any(|other| other.parent_id == Some(candidate.id))
            }) else {
                return Err(StoreError::Parse(
                    "Kategória má cyklickú rodičovskú väzbu.".into(),
                ));
            };
            let category = remaining.remove(index);
            let deleted = self
                .conn
                .execute("DELETE FROM categories WHERE id = ?1", [category.id])?;
            if deleted != 1 {
                return Err(StoreError::Db("kategória počas odstránenia zmizla".into()));
            }
        }
        Ok(())
    }
}

impl From<Category> for CategoryDeleteItem {
    fn from(category: Category) -> Self {
        Self {
            id: category.id,
            parent_id: category.parent_id,
            name: category.name,
            archived: category.archived,
        }
    }
}

fn placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(",")
}

fn finish_write<T>(connection: &rusqlite::Connection, result: Result<T>) -> Result<T> {
    let value = match result {
        Ok(value) => value,
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK");
            return Err(error);
        }
    };
    if let Err(error) = connection.execute_batch("COMMIT") {
        let _ = connection.execute_batch("ROLLBACK");
        return Err(error.into());
    }
    Ok(value)
}

fn finish_read<T>(connection: &rusqlite::Connection, result: Result<T>) -> Result<T> {
    let value = match result {
        Ok(value) => value,
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK");
            return Err(error);
        }
    };
    if let Err(error) = connection.execute_batch("COMMIT") {
        let _ = connection.execute_batch("ROLLBACK");
        return Err(error.into());
    }
    Ok(value)
}
