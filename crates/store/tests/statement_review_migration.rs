//! Oracles: plan 091 review21-contract.md migration and review21-test-plan.md.
use rusqlite::{types::Value, Connection};
use store::Store;

struct LegacyDb(std::path::PathBuf);
impl LegacyDb {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abakus-review21-migration-{name}-{}.db", std::process::id()));
        assert!(!path.exists());
        let raw = Connection::open(&path).unwrap();
        raw.execute_batch(include_str!("support/review_v5.sql")).unwrap();
        raw.execute_batch("INSERT INTO accounts (id,iban,kind,label) VALUES (1,'SK4411000000000012345678','personal','Pôvodný');
            INSERT INTO statements (id,account_id,number,period_start,period_end,opening_cents,closing_cents,checksum_status,file_hash) VALUES (1,1,1,'2026-06-01','2026-06-01',1000,1000,'ok','legacy');
            INSERT INTO categories (id,name,kind) VALUES (1,'Vlastná','expense');
            INSERT INTO rules (id,match_kind,key,category_id,hit_count) VALUES (1,'merchant','legacy',1,7);
            INSERT INTO transactions (id,statement_id,account_id,fingerprint,posted_date,tx_date,kind,amount_cents,merchant_raw,merchant_norm,raw_block,category_id,status,rule_id,source,note) VALUES (1,1,1,'old-fp','2026-06-01','2026-06-01','card',0,'Legacy','legacy','presný blok',1,'confirmed',1,'learned','Žltá\npoznámka');
            INSERT INTO rule_sources (rule_id,transaction_id,statement_id) VALUES (1,1,1);
            INSERT INTO settings (key,value) VALUES ('check_updates','1');").unwrap();
        Self(path)
    }
    fn raw(&self) -> Connection { Connection::open(&self.0).unwrap() }
}
impl Drop for LegacyDb { fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); } }

fn rows(raw: &Connection, sql: &str) -> Vec<Vec<Value>> {
    let mut query = raw.prepare(sql).unwrap();
    let columns = query.column_count();
    query.query_map([], |row| (0..columns).map(|index| row.get(index)).collect::<rusqlite::Result<Vec<Value>>>())
        .unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap()
}

fn old_data(raw: &Connection) -> Vec<Vec<Vec<Value>>> {
    ["SELECT * FROM accounts", "SELECT id,account_id,number,period_start,period_end,opening_cents,closing_cents,checksum_status,checksum_off_by,file_hash,imported_at FROM statements",
        "SELECT * FROM transactions", "SELECT * FROM categories", "SELECT * FROM rules", "SELECT * FROM rule_sources",
        "SELECT * FROM recurring_decisions", "SELECT * FROM recurring_members", "SELECT * FROM settings WHERE key <> 'schema_version'"]
        .map(|sql| rows(raw, sql)).into()
}

fn schema(raw: &Connection) -> Vec<Vec<Value>> { rows(raw, "SELECT type,name,tbl_name,sql FROM sqlite_master ORDER BY type,name") }

#[test]
fn populated_v5_keeps_all_old_values_and_unknown_warnings_after_two_opens() {
    let db = LegacyDb::new("preserve");
    let before = old_data(&db.raw());
    drop(Store::open(&db.0).unwrap());
    assert_eq!(old_data(&db.raw()), before);
    assert_eq!(rows(&db.raw(), "SELECT value FROM settings WHERE key='schema_version'"), vec![vec![Value::Text("6".into())]]);
    assert_eq!(rows(&db.raw(), "SELECT parser_warnings_json FROM statements"), vec![vec![Value::Null]]);
    let migrated_schema = schema(&db.raw());
    drop(Store::open(&db.0).unwrap());
    assert_eq!(schema(&db.raw()), migrated_schema);
    assert_eq!(old_data(&db.raw()), before);
    assert_eq!(rows(&db.raw(), "SELECT parser_warnings_json FROM statements"), vec![vec![Value::Null]]);
}

#[test]
fn marker_failure_after_alter_restores_v5_schema_data_and_marker_then_retry_works() {
    let db = LegacyDb::new("rollback");
    db.raw().execute_batch("CREATE TRIGGER reject_v6 BEFORE INSERT ON settings WHEN NEW.key='schema_version' AND NEW.value='6' AND EXISTS (SELECT 1 FROM pragma_table_info('statements') WHERE name='parser_warnings_json') BEGIN SELECT RAISE(ABORT,'synthetic late v6 failure'); END;").unwrap();
    let before_schema = schema(&db.raw());
    let before_data = old_data(&db.raw());
    let error = Store::open(&db.0).err().expect("final version write must fail after ALTER");
    assert!(error.to_string().contains("synthetic late v6 failure"));
    assert_eq!(schema(&db.raw()), before_schema);
    assert_eq!(old_data(&db.raw()), before_data);
    assert_eq!(rows(&db.raw(), "SELECT value FROM settings WHERE key='schema_version'"), vec![vec![Value::Text("5".into())]]);
    db.raw().execute_batch("DROP TRIGGER reject_v6").unwrap();
    drop(Store::open(&db.0).unwrap());
    assert_eq!(rows(&db.raw(), "SELECT parser_warnings_json FROM statements"), vec![vec![Value::Null]]);
}

#[test]
fn future_version_refusal_preserves_exact_schema_data_and_marker() {
    let db = LegacyDb::new("future");
    db.raw().execute("UPDATE settings SET value='99' WHERE key='schema_version'", []).unwrap();
    let before_schema = schema(&db.raw());
    let before_data = old_data(&db.raw());
    assert!(Store::open(&db.0).is_err());
    assert_eq!(schema(&db.raw()), before_schema);
    assert_eq!(old_data(&db.raw()), before_data);
    assert_eq!(rows(&db.raw(), "SELECT value FROM settings WHERE key='schema_version'"), vec![vec![Value::Text("99".into())]]);
}

#[test]
fn fresh_database_reaches_v6_with_nullable_warning_column() {
    let path = std::env::temp_dir().join(format!("abakus-review21-fresh-{}.db", std::process::id()));
    assert!(!path.exists());
    let db = LegacyDb(path);
    drop(Store::open(&db.0).unwrap());
    assert_eq!(rows(&db.raw(), "SELECT value FROM settings WHERE key='schema_version'"), vec![vec![Value::Text("6".into())]]);
    assert_eq!(rows(&db.raw(), "SELECT type,\"notnull\",dflt_value FROM pragma_table_info('statements') WHERE name='parser_warnings_json'"), vec![vec![Value::Text("TEXT".into()), Value::Integer(0), Value::Null]]);
}

#[test]
fn migrated_balanced_confirmed_statement_keeps_unknown_warnings_and_incomplete_status() {
    let db = LegacyDb::new("legacy-review");
    let store = Store::open(&db.0).unwrap();
    let review = store.statement_review(1).unwrap();
    assert_eq!(review.parser_warnings, None);
    assert_eq!(review.checksum, parser::Checksum::Ok);
    assert_eq!((review.unassigned_count, review.suggested_count), (0, 0));
    assert_eq!(review.status, store::StatementReviewStatus::EvidenceIncomplete);
    drop(store);
    assert_eq!(Store::open(&db.0).unwrap().statement_review(1).unwrap(), review);
}

#[test]
fn migrated_unknown_warnings_with_open_row_need_attention_without_hiding_unknown() {
    let db = LegacyDb::new("legacy-open");
    db.raw().execute("UPDATE transactions SET status='unassigned'", []).unwrap();
    let review = Store::open(&db.0).unwrap().statement_review(1).unwrap();
    assert_eq!(review.parser_warnings, None);
    assert_eq!(review.checksum, parser::Checksum::Ok);
    assert_eq!((review.unassigned_count, review.suggested_count), (1, 0));
    assert_eq!(review.status, store::StatementReviewStatus::NeedsAttention);
}

#[test]
fn same_file_retry_after_v5_migration_never_fills_unknown_warning_evidence() {
    let db = LegacyDb::new("legacy-retry");
    let mut store = Store::open(&db.0).unwrap();
    let date = chrono::NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
    let retry_input = parser::Statement {
        iban: "SK4411000000000012345678".into(), account_kind: parser::AccountKind::Personal,
        number: 1, period_start: date, period_end: date, opening_cents: Some(1000), closing_cents: Some(877),
        transactions: vec![], warnings: vec!["new parser knowledge".into()],
    };
    let before = old_data(&db.raw());
    let outcome = store.import_statement(&retry_input, "legacy").unwrap();
    assert_eq!((outcome.statement_id, outcome.inserted, outcome.duplicates, outcome.already_imported), (1, 0, 0, true));
    assert_eq!(outcome.warnings, vec!["new parser knowledge"]);
    assert_eq!(outcome.checksum, parser::Checksum::OffBy(123));
    assert_eq!(store.statement_review(1).unwrap().parser_warnings, None);
    assert_eq!(store.statement_review(1).unwrap().checksum, parser::Checksum::Ok);
    assert_eq!(old_data(&db.raw()), before);
}
