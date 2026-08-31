use store::Store;

/// t15 review addition: `save_category`'s rename branch checks the UPDATE row
/// count, mirroring `archive_category`, so a system category cannot be
/// silently renamed away.
#[test]
fn renaming_a_system_category_is_refused() {
    let mut s = Store::open_in_memory().unwrap();
    let system_id = s.list_categories().unwrap().into_iter().find(|c| c.system).unwrap().id;
    let err = s.save_category(Some(system_id), None, "premenované", store::CategoryKind::Expense).unwrap_err();
    assert!(matches!(err, store::StoreError::Parse(ref m) if m == "systémovú kategóriu nemožno premenovať"));
    let unchanged = s.list_categories().unwrap().into_iter().find(|c| c.id == system_id).unwrap();
    assert_ne!(unchanged.name, "premenované");
}
