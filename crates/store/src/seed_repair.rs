//! Section 9 Spotify repair. An existing install seeded before the
//! Predplatné/Spotify split still has its `spotify` seed rule pointing at
//! Apple. This repairs ONLY that exact legacy signature, inside A's v4
//! migration transaction: no BEGIN/COMMIT of its own (contract §1/§8: no
//! nested transactions in seed or repair helpers).
//!
//! `repair_spotify_seed_tx` is `pub(crate)` by contract and has no caller
//! yet in this isolated worktree: A wires the one call site into `migrate.rs`
//! during the final selective merge. Until then the plain (non-test) build
//! sees it as unreachable, so this module is exempted from `dead_code`; the
//! `#[cfg(test)]` block below exercises it directly and exhaustively.
#![allow(dead_code)]
use crate::{Result, Store};

struct LegacySignature { rule_id: i64, predplatne_id: i64 }
enum SiblingLookup { Found(i64), None, Ambiguous }

impl Store {
    pub(crate) fn repair_spotify_seed_tx(&mut self) -> Result<()> {
        let Some(sig) = self.legacy_spotify_signature()? else { return Ok(()) };
        let spotify_id = match self.active_spotify_sibling(sig.predplatne_id)? {
            SiblingLookup::Found(id) => id,
            SiblingLookup::Ambiguous => return Ok(()),
            SiblingLookup::None => self.create_spotify_sibling(sig.predplatne_id)?,
        };
        self.conn.execute("UPDATE rules SET category_id = ?2 WHERE id = ?1", rusqlite::params![sig.rule_id, spotify_id])?;
        let ids: Vec<i64> = {
            let mut st = self.conn.prepare("SELECT id FROM transactions WHERE rule_id = ?1 AND status IN ('suggested','unassigned')")?;
            let rows = st.query_map([sig.rule_id], |r| r.get(0))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        self.classify_ids(&ids)?;
        Ok(())
    }

    /// Only an exact match repairs anything: seed kind, key `spotify`, empty
    /// place, destination an active, non-system, expense category named
    /// exactly "Apple" whose parent is an active, non-system, expense
    /// category named exactly "Predplatné". A deleted rule, a rule already
    /// redirected elsewhere, or a renamed/reparented/archived tree all fail
    /// this match and are preserved untouched.
    fn legacy_spotify_signature(&self) -> Result<Option<LegacySignature>> {
        type Row = (i64, String, bool, bool, String, i64, String, bool, bool, String);
        let row: Option<Row> = self
            .conn
            .query_row(
                "SELECT r.id, c.name, c.archived, c.system, c.kind, p.id, p.name, p.archived, p.system, p.kind \
                 FROM rules r JOIN categories c ON c.id = r.category_id JOIN categories p ON p.id = c.parent_id \
                 WHERE r.match_kind = 'seed' AND r.key = 'spotify' AND r.place = ''",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get::<_, i64>(2)? != 0,
                        row.get::<_, i64>(3)? != 0,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get::<_, i64>(7)? != 0,
                        row.get::<_, i64>(8)? != 0,
                        row.get(9)?,
                    ))
                },
            )
            .ok();
        Ok(row.and_then(is_legacy_signature))
    }

    fn active_spotify_sibling(&self, predplatne_id: i64) -> Result<SiblingLookup> {
        let matches: Vec<i64> = self
            .children_of(predplatne_id)?
            .into_iter()
            .filter(|c| !c.archived && !c.system && parser::fold::fold(&c.name) == parser::fold::fold("Spotify"))
            .map(|c| c.id)
            .collect();
        Ok(match matches.as_slice() {
            [] => SiblingLookup::None,
            [id] => SiblingLookup::Found(*id),
            _ => SiblingLookup::Ambiguous,
        })
    }

    fn create_spotify_sibling(&mut self, predplatne_id: i64) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO categories (parent_id, name, kind, sort) VALUES (?1, 'Spotify', 'expense', (SELECT COALESCE(MAX(sort),0)+1 FROM categories WHERE parent_id = ?1))",
            [predplatne_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }
}

#[allow(clippy::type_complexity)]
fn is_legacy_signature(row: (i64, String, bool, bool, String, i64, String, bool, bool, String)) -> Option<LegacySignature> {
    let (rule_id, cat_name, cat_archived, cat_system, cat_kind, parent_id, parent_name, parent_archived, parent_system, parent_kind) = row;
    let ok = cat_name == "Apple" && !cat_archived && !cat_system && cat_kind == "expense" && parent_name == "Predplatné" && !parent_archived && !parent_system && parent_kind == "expense";
    ok.then_some(LegacySignature { rule_id, predplatne_id: parent_id })
}

/// `repair_spotify_seed_tx` is intentionally `pub(crate)` per the contract
/// (it only ever runs inside A's v4 migration transaction), so these are
/// same-crate unit tests, not a `tests/` integration target. The public,
/// cross-crate boundary is instead covered by `tests/spotify_repair.rs`,
/// which asserts the fresh-seed half of this same section through
/// `Store::open_in_memory` alone.
#[cfg(test)]
mod tests {
    use crate::{CategoryKind, Store, TxFilter};
    use parser::AccountKind;

    fn make_legacy(s: &mut Store) -> i64 {
        let spotify_id = s.category_by_path("Predplatné/Spotify").unwrap().unwrap();
        let apple_id = s.category_by_path("Predplatné/Apple").unwrap().unwrap();
        let rule_id: i64 = s.conn.query_row("SELECT id FROM rules WHERE match_kind='seed' AND key='spotify'", [], |r| r.get(0)).unwrap();
        // Repoint the rule at Apple BEFORE deleting Spotify: `rules.category_id`
        // has no ON DELETE CASCADE, so deleting a still-referenced category
        // would trip the foreign key, not model the pre-change legacy shape.
        s.conn.execute("UPDATE rules SET category_id = ?2 WHERE id = ?1", rusqlite::params![rule_id, apple_id]).unwrap();
        s.conn.execute("DELETE FROM categories WHERE id = ?1", [spotify_id]).unwrap();
        rule_id
    }

    #[test]
    fn repairs_the_legacy_signature_by_creating_and_redirecting_to_spotify() {
        let mut s = Store::open_in_memory().unwrap();
        let rule_id = make_legacy(&mut s);

        s.repair_spotify_seed_tx().unwrap();

        let spotify_id = s.category_by_path("Predplatné/Spotify").unwrap().expect("spotify recreated");
        let cat: i64 = s.conn.query_row("SELECT category_id FROM rules WHERE id = ?1", [rule_id], |r| r.get(0)).unwrap();
        assert_eq!(cat, spotify_id);
    }

    /// M01/C04: reopen must never create a second Spotify category or move
    /// the rule again once it already points at it.
    #[test]
    fn is_idempotent_on_reopen() {
        let mut s = Store::open_in_memory().unwrap();
        let rule_id = make_legacy(&mut s);
        s.repair_spotify_seed_tx().unwrap();
        let first = s.category_by_path("Predplatné/Spotify").unwrap().unwrap();

        s.repair_spotify_seed_tx().unwrap();

        let cats: Vec<i64> = s.list_categories().unwrap().into_iter().filter(|c| c.name == "Spotify").map(|c| c.id).collect();
        assert_eq!(cats, vec![first], "must not create a second Spotify category");
        let cat: i64 = s.conn.query_row("SELECT category_id FROM rules WHERE id = ?1", [rule_id], |r| r.get(0)).unwrap();
        assert_eq!(cat, first);
    }

    fn seed_transaction(s: &Store, id: i64, fp: &str, status: &str, category_id: i64, rule_id: i64) {
        s.conn.execute(
            "INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, status, source, category_id, rule_id) \
             VALUES (?1, 1, 1, ?2, '2026-06-01', '2026-06-01', 'card', -999, 'SPOTIFY', 'spotify', 'raw', ?3, 'seed', ?4, ?5)",
            rusqlite::params![id, fp, status, category_id, rule_id],
        ).unwrap();
    }

    #[test]
    fn reclassifies_open_rows_but_preserves_confirmed_ones() {
        let mut s = Store::open_in_memory().unwrap();
        let rule_id = make_legacy(&mut s);
        let apple_id = s.category_by_path("Predplatné/Apple").unwrap().unwrap();
        s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
        s.conn.execute_batch("INSERT INTO statements (id, account_id, number, period_start, period_end, checksum_status, file_hash) VALUES (1, 1, 1, '2026-06-01', '2026-06-30', 'ok', 'h1');").unwrap();
        seed_transaction(&s, 1, "fp-open", "suggested", apple_id, rule_id);
        seed_transaction(&s, 2, "fp-confirmed", "confirmed", apple_id, rule_id);

        s.repair_spotify_seed_tx().unwrap();

        let spotify_id = s.category_by_path("Predplatné/Spotify").unwrap().unwrap();
        let rows = s.list_transactions(&TxFilter::default()).unwrap();
        let open = rows.iter().find(|r| r.id == 1).unwrap();
        let confirmed = rows.iter().find(|r| r.id == 2).unwrap();
        assert_eq!(open.category_id, Some(spotify_id), "open row must follow the redirected seed rule");
        assert_eq!(confirmed.category_id, Some(apple_id), "a confirmed row must stay exactly as it was");
    }

    #[test]
    fn preserves_a_deleted_seed_rule() {
        let mut s = Store::open_in_memory().unwrap();
        let rule_id = make_legacy(&mut s);
        s.conn.execute("DELETE FROM rules WHERE id = ?1", [rule_id]).unwrap();

        s.repair_spotify_seed_tx().unwrap();

        assert!(s.category_by_path("Predplatné/Spotify").unwrap().is_none(), "no rule left to repair, nothing to create");
    }

    #[test]
    fn preserves_a_rule_already_redirected_elsewhere() {
        let mut s = Store::open_in_memory().unwrap();
        let rule_id = make_legacy(&mut s);
        let netflix = s.category_by_path("Predplatné/Netflix").unwrap().unwrap();
        s.conn.execute("UPDATE rules SET category_id = ?2 WHERE id = ?1", rusqlite::params![rule_id, netflix]).unwrap();

        s.repair_spotify_seed_tx().unwrap();

        let cat: i64 = s.conn.query_row("SELECT category_id FROM rules WHERE id = ?1", [rule_id], |r| r.get(0)).unwrap();
        assert_eq!(cat, netflix, "an explicit redirect elsewhere is the user's own choice");
        assert!(s.category_by_path("Predplatné/Spotify").unwrap().is_none());
    }

    #[test]
    fn preserves_an_archived_apple_category() {
        let mut s = Store::open_in_memory().unwrap();
        let rule_id = make_legacy(&mut s);
        let apple = s.category_by_path("Predplatné/Apple").unwrap().unwrap();
        s.archive_category(apple).unwrap();

        s.repair_spotify_seed_tx().unwrap();

        let cat: i64 = s.conn.query_row("SELECT category_id FROM rules WHERE id = ?1", [rule_id], |r| r.get(0)).unwrap();
        assert_eq!(cat, apple);
    }

    #[test]
    fn preserves_a_renamed_predplatne_root() {
        let mut s = Store::open_in_memory().unwrap();
        let rule_id = make_legacy(&mut s);
        let predplatne = s.category_by_path("Predplatné").unwrap().unwrap();
        s.save_category(Some(predplatne), None, "Predplatné (premenované)", CategoryKind::Expense).unwrap();

        s.repair_spotify_seed_tx().unwrap();

        let apple = s.category_by_path("Predplatné (premenované)/Apple").unwrap().unwrap();
        let cat: i64 = s.conn.query_row("SELECT category_id FROM rules WHERE id = ?1", [rule_id], |r| r.get(0)).unwrap();
        assert_eq!(cat, apple, "a renamed Predplatné root no longer matches the exact legacy signature");
    }

    #[test]
    fn preserves_a_conflicting_ambiguous_spotify_sibling() {
        let mut s = Store::open_in_memory().unwrap();
        let rule_id = make_legacy(&mut s);
        let predplatne = s.category_by_path("Predplatné").unwrap().unwrap();
        s.conn.execute("INSERT INTO categories (parent_id, name, kind, sort) VALUES (?1, 'Spotify', 'expense', 90)", [predplatne]).unwrap();
        s.conn.execute("INSERT INTO categories (parent_id, name, kind, sort) VALUES (?1, 'SPOTIFY', 'expense', 91)", [predplatne]).unwrap();

        s.repair_spotify_seed_tx().unwrap();

        let apple = s.category_by_path("Predplatné/Apple").unwrap().unwrap();
        let cat: i64 = s.conn.query_row("SELECT category_id FROM rules WHERE id = ?1", [rule_id], |r| r.get(0)).unwrap();
        assert_eq!(cat, apple, "an ambiguous existing Spotify-named sibling must not be forced into use");
    }

    #[test]
    fn reuses_a_single_existing_unambiguous_spotify_sibling() {
        let mut s = Store::open_in_memory().unwrap();
        let rule_id = make_legacy(&mut s);
        let predplatne = s.category_by_path("Predplatné").unwrap().unwrap();
        s.conn.execute("INSERT INTO categories (parent_id, name, kind, sort) VALUES (?1, 'spotify', 'expense', 90)", [predplatne]).unwrap();
        let existing = s.conn.last_insert_rowid();

        s.repair_spotify_seed_tx().unwrap();

        let cat: i64 = s.conn.query_row("SELECT category_id FROM rules WHERE id = ?1", [rule_id], |r| r.get(0)).unwrap();
        assert_eq!(cat, existing, "a single unambiguous existing sibling is reused, not duplicated");
        let count = s.list_categories().unwrap().into_iter().filter(|c| c.parent_id == Some(predplatne) && parser::fold::fold(&c.name) == parser::fold::fold("Spotify")).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn a_minimal_legacy_fixture_without_the_spotify_seed_rule_gains_nothing() {
        let mut s = Store::open_in_memory().unwrap();
        s.conn.execute("DELETE FROM rules", []).unwrap();
        s.conn.execute("DELETE FROM categories WHERE name = 'Spotify'", []).unwrap();

        s.repair_spotify_seed_tx().unwrap();

        assert!(s.category_by_path("Predplatné/Spotify").unwrap().is_none());
        assert_eq!(s.list_rules().unwrap().len(), 0, "no complete reseed");
    }
}
