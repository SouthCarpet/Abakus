//! Section 9 Spotify: the public-boundary half of the fresh-seed change.
//! `repair_spotify_seed_tx` itself is `pub(crate)` by contract (it only ever
//! runs inside A's v4 migration transaction, wired at final integration), so
//! it cannot be called from this external `tests/` crate; its exhaustive
//! behavioral coverage lives in `crates/store/src/seed_repair.rs`'s own
//! `#[cfg(test)]` module instead. This file proves the other half: a FRESH
//! database (the common case, no legacy signature to repair at all) already
//! has the Spotify category and the `spotify` seed rule pointing at it,
//! through nothing but the public `Store` API.
use store::{CategoryKind, Store};

#[test]
fn a_fresh_database_already_has_spotify_under_predplatne() {
    let s = Store::open_in_memory().unwrap();

    let spotify = s.category_by_path("Predplatné/Spotify").unwrap().expect("Spotify exists under Predplatné on a fresh install");

    let cats = s.list_categories().unwrap();
    let spotify_cat = cats.iter().find(|c| c.id == spotify).unwrap();
    assert_eq!(spotify_cat.kind, CategoryKind::Expense);
    assert!(!spotify_cat.system);
    assert!(!spotify_cat.archived);
}

#[test]
fn a_fresh_database_seed_rule_points_spotify_at_the_spotify_category_not_apple() {
    let s = Store::open_in_memory().unwrap();
    let spotify = s.category_by_path("Predplatné/Spotify").unwrap().unwrap();
    let apple = s.category_by_path("Predplatné/Apple").unwrap().unwrap();

    let rule = s.list_rules().unwrap().into_iter().find(|r| r.key == "spotify").unwrap();

    assert_eq!(rule.category_id, spotify);
    assert_ne!(rule.category_id, apple, "the fresh seed no longer points spotify at Apple");
}

#[test]
fn a_fresh_database_still_seeds_apple_as_its_own_distinct_category() {
    let s = Store::open_in_memory().unwrap();
    let apple = s.category_by_path("Predplatné/Apple").unwrap().unwrap();
    let itunes_rule = s.list_rules().unwrap().into_iter().find(|r| r.key == "itunes").unwrap();
    assert_eq!(itunes_rule.category_id, apple, "Apple/iTunes classification is untouched by the Spotify split");
}
