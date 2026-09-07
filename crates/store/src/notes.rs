//! 0.1.2: a free-text note the user attaches to one transaction. Exact text,
//! including newlines, is preserved; nothing here touches classification,
//! category, fingerprint, or the search-index columns (`merchant_norm` etc).
use crate::{Result, Store, StoreError};

/// Unicode code points, not bytes or UTF-16 units: `note.chars().count()`.
pub const NOTE_MAX_CHARS: usize = 2000;

impl Store {
    /// Rejects (without writing anything) a NUL byte, a note over the limit,
    /// or an id that does not exist. An empty string is a valid note: it
    /// clears whatever was there. Validation runs BEFORE the `UPDATE`, so a
    /// rejected save never touches the row even if the id happens to exist.
    pub fn save_transaction_note(&mut self, id: i64, note: &str) -> Result<()> {
        if note.contains('\0') {
            return Err(StoreError::NoteContainsNul);
        }
        let actual = note.chars().count();
        if actual > NOTE_MAX_CHARS {
            return Err(StoreError::NoteTooLong { max: NOTE_MAX_CHARS, actual });
        }
        let updated = self.conn.execute("UPDATE transactions SET note = ?2 WHERE id = ?1", rusqlite::params![id, note])?;
        if updated == 0 {
            return Err(StoreError::UnknownTransaction { id });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser::AccountKind;

    fn seeded_transaction(s: &mut Store) -> i64 {
        s.upsert_account("SK4411000000000012345678", AccountKind::Personal, "Osobný").unwrap();
        s.conn.execute_batch(
            "INSERT INTO statements (id, account_id, number, period_start, period_end, checksum_status, file_hash) VALUES (1, 1, 1, '2026-06-01', '2026-06-30', 'ok', 'h1'); \
             INSERT INTO transactions (id, statement_id, account_id, fingerprint, posted_date, tx_date, kind, amount_cents, merchant_raw, merchant_norm, raw_block, status, source) \
               VALUES (1, 1, 1, 'fp-1', '2026-06-01', '2026-06-01', 'card', -500, 'ACME', 'acme', 'raw', 'unassigned', 'none');",
        ).unwrap();
        1
    }

    #[test]
    fn an_empty_note_clears_and_round_trips() {
        let mut s = Store::open_in_memory().unwrap();
        let id = seeded_transaction(&mut s);
        s.save_transaction_note(id, "kúpiť darček").unwrap();

        s.save_transaction_note(id, "").unwrap();

        assert_eq!(s.list_transactions(&crate::TxFilter::default()).unwrap()[0].note, "");
    }

    #[test]
    fn a_nul_byte_is_rejected_and_leaves_the_row_untouched() {
        let mut s = Store::open_in_memory().unwrap();
        let id = seeded_transaction(&mut s);
        s.save_transaction_note(id, "pred").unwrap();

        let e = s.save_transaction_note(id, "obsahuje\0nul").unwrap_err();

        assert!(matches!(e, StoreError::NoteContainsNul), "got {e:?}");
        assert_eq!(s.list_transactions(&crate::TxFilter::default()).unwrap()[0].note, "pred");
    }

    #[test]
    fn exactly_the_limit_is_accepted_one_over_is_rejected_and_untouched() {
        let mut s = Store::open_in_memory().unwrap();
        let id = seeded_transaction(&mut s);
        let at_limit = "č".repeat(NOTE_MAX_CHARS);
        s.save_transaction_note(id, &at_limit).unwrap();

        let over = "č".repeat(NOTE_MAX_CHARS + 1);
        let e = s.save_transaction_note(id, &over).unwrap_err();

        assert!(matches!(e, StoreError::NoteTooLong { max: NOTE_MAX_CHARS, actual } if actual == NOTE_MAX_CHARS + 1), "got {e:?}");
        assert_eq!(s.list_transactions(&crate::TxFilter::default()).unwrap()[0].note, at_limit, "the over-limit attempt must not have written anything");
    }

    #[test]
    fn a_nonexistent_id_is_rejected_and_changes_nothing() {
        let mut s = Store::open_in_memory().unwrap();
        seeded_transaction(&mut s);

        let e = s.save_transaction_note(9_999, "poznámka").unwrap_err();

        assert!(matches!(e, StoreError::UnknownTransaction { id: 9_999 }), "got {e:?}");
    }

    #[test]
    fn newlines_and_unicode_are_preserved_exactly() {
        let mut s = Store::open_in_memory().unwrap();
        let id = seeded_transaction(&mut s);
        let text = "zaplatiť do 5.\nspýtať sa účtovníčky\n\ndakujem 🙂";

        s.save_transaction_note(id, text).unwrap();

        assert_eq!(s.list_transactions(&crate::TxFilter::default()).unwrap()[0].note, text);
    }

    /// The invariant `csv_line`/`row_matches_text` and the schema comment all
    /// depend on: reclassifying a row (a rule change, an account edit, a
    /// delete elsewhere) never writes `note`, because its `UPDATE` never
    /// names the column.
    #[test]
    fn a_note_survives_reclassification() {
        let mut s = Store::open_in_memory().unwrap();
        let id = seeded_transaction(&mut s);
        s.save_transaction_note(id, "osobná poznámka").unwrap();

        s.classify_ids(&[id]).unwrap();

        assert_eq!(s.list_transactions(&crate::TxFilter::default()).unwrap()[0].note, "osobná poznámka");
    }
}
