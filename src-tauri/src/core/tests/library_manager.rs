use crate::core::library_manager::LibraryItemRecord;
use crate::core::skill_store::SkillStore;

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    let store = SkillStore::new(db);
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

#[test]
fn add_library_and_retrieve() {
    let (_dir, store) = make_store();
    let lib = store
        .add_library(
            "Test Lib",
            "https://github.com/example/repo",
            "marketplace",
            r#"["skill"]"#,
            false,
        )
        .expect("add_library");

    assert_eq!(lib.name, "Test Lib");
    assert_eq!(lib.url, "https://github.com/example/repo");
    assert!(!lib.trusted);

    let libs = store.list_libraries().expect("list_libraries");
    assert_eq!(libs.len(), 1);
    assert_eq!(libs[0].id, lib.id);
    assert_eq!(libs[0].name, "Test Lib");
}

#[test]
fn delete_library_cascades_items() {
    let (_dir, store) = make_store();
    let lib = store
        .add_library(
            "Cascade Lib",
            "https://github.com/example/cascade",
            "github_repo",
            r#"["skill"]"#,
            true,
        )
        .expect("add_library");

    let item = LibraryItemRecord {
        id: "item-1".to_string(),
        library_id: lib.id.clone(),
        asset_type: "skill".to_string(),
        name: "Test Skill".to_string(),
        description: Some("A test skill".to_string()),
        subpath: Some("skills/test".to_string()),
        metadata_json: None,
        indexed_at: 1000,
    };
    store
        .upsert_library_item(&item)
        .expect("upsert_library_item");

    let items = store
        .list_library_items(Some(&lib.id), None)
        .expect("list_library_items");
    assert_eq!(items.len(), 1);

    store.delete_library(&lib.id).expect("delete_library");

    let libs = store.list_libraries().expect("list_libraries");
    assert!(libs.is_empty());

    let items_after = store
        .list_library_items(Some(&lib.id), None)
        .expect("list_library_items");
    assert!(items_after.is_empty());
}

#[test]
fn upsert_library_item_insert_and_update() {
    let (_dir, store) = make_store();
    let lib = store
        .add_library(
            "Upsert Lib",
            "https://github.com/example/upsert",
            "marketplace",
            r#"["skill"]"#,
            false,
        )
        .expect("add_library");

    let item = LibraryItemRecord {
        id: "item-u1".to_string(),
        library_id: lib.id.clone(),
        asset_type: "skill".to_string(),
        name: "Original Name".to_string(),
        description: Some("Original".to_string()),
        subpath: Some("skills/a".to_string()),
        metadata_json: None,
        indexed_at: 100,
    };
    store.upsert_library_item(&item).expect("insert");

    let items = store.list_library_items(Some(&lib.id), None).expect("list");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "Original Name");

    // Upsert with same library_id + subpath should update
    let updated = LibraryItemRecord {
        id: "item-u2".to_string(), // different id
        library_id: lib.id.clone(),
        asset_type: "skill".to_string(),
        name: "Updated Name".to_string(),
        description: Some("Updated".to_string()),
        subpath: Some("skills/a".to_string()), // same subpath
        metadata_json: None,
        indexed_at: 200,
    };
    store.upsert_library_item(&updated).expect("upsert");

    let items2 = store.list_library_items(Some(&lib.id), None).expect("list");
    assert_eq!(items2.len(), 1);
    assert_eq!(items2[0].name, "Updated Name");
    assert_eq!(items2[0].description.as_deref(), Some("Updated"));
}

#[test]
fn search_library_items_by_query() {
    let (_dir, store) = make_store();
    let lib = store
        .add_library(
            "Search Lib",
            "https://github.com/example/search",
            "marketplace",
            r#"["skill"]"#,
            false,
        )
        .expect("add_library");

    let items = vec![
        LibraryItemRecord {
            id: "s1".to_string(),
            library_id: lib.id.clone(),
            asset_type: "skill".to_string(),
            name: "Code Review".to_string(),
            description: Some("Automated code review assistant".to_string()),
            subpath: Some("skills/code-review".to_string()),
            metadata_json: None,
            indexed_at: 100,
        },
        LibraryItemRecord {
            id: "s2".to_string(),
            library_id: lib.id.clone(),
            asset_type: "skill".to_string(),
            name: "Test Generator".to_string(),
            description: Some("Generate unit tests".to_string()),
            subpath: Some("skills/test-gen".to_string()),
            metadata_json: None,
            indexed_at: 100,
        },
        LibraryItemRecord {
            id: "s3".to_string(),
            library_id: lib.id.clone(),
            asset_type: "plugin".to_string(),
            name: "Code Formatter".to_string(),
            description: Some("Format code automatically".to_string()),
            subpath: Some("plugins/formatter".to_string()),
            metadata_json: None,
            indexed_at: 100,
        },
    ];
    for item in &items {
        store.upsert_library_item(item).expect("upsert");
    }

    // Search by name
    let results = store.search_library_items("Code", None).expect("search");
    assert_eq!(results.len(), 2); // Code Review + Code Formatter

    // Search by description
    let results2 = store
        .search_library_items("unit tests", None)
        .expect("search");
    assert_eq!(results2.len(), 1);
    assert_eq!(results2[0].name, "Test Generator");

    // Search with asset_type filter
    let results3 = store
        .search_library_items("Code", Some("plugin"))
        .expect("search");
    assert_eq!(results3.len(), 1);
    assert_eq!(results3[0].name, "Code Formatter");

    // No results
    let results4 = store
        .search_library_items("nonexistent", None)
        .expect("search");
    assert!(results4.is_empty());
}

#[test]
fn seed_default_libraries_is_idempotent() {
    let (_dir, store) = make_store();

    let count1 = store.seed_default_libraries().expect("seed first");
    assert_eq!(count1, 3);

    let libs = store.list_libraries().expect("list");
    assert_eq!(libs.len(), 3);
    assert!(libs.iter().all(|l| l.trusted));

    // Second call should not add duplicates
    let count2 = store.seed_default_libraries().expect("seed second");
    assert_eq!(count2, 0);

    let libs2 = store.list_libraries().expect("list");
    assert_eq!(libs2.len(), 3);
}
