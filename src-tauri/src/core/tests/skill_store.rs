use std::path::PathBuf;

use crate::core::skill_store::{AssetRecord, AssetTargetRecord, SkillStore};

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    let store = SkillStore::new(db);
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

fn make_skill(id: &str, name: &str, central_path: &str, updated_at: i64) -> AssetRecord {
    AssetRecord {
        id: id.to_string(),
        name: name.to_string(),
        description: None,
        asset_type: "skill".to_string(),
        source_type: "local".to_string(),
        source_ref: Some("/tmp/source".to_string()),
        source_subpath: None,
        source_revision: None,
        central_path: Some(central_path.to_string()),
        config_json: None,
        security_status: None,
        content_hash: None,
        created_at: 1,
        updated_at,
        last_sync_at: None,
        last_seen_at: 1,
        status: "ok".to_string(),
    }
}

#[test]
fn schema_is_idempotent() {
    let (_dir, store) = make_store();
    store.ensure_schema().expect("ensure_schema again");
}

#[test]
fn settings_roundtrip_and_update() {
    let (_dir, store) = make_store();

    assert_eq!(store.get_setting("missing").unwrap(), None);
    store.set_setting("k", "v1").unwrap();
    assert_eq!(store.get_setting("k").unwrap().as_deref(), Some("v1"));
    store.set_setting("k", "v2").unwrap();
    assert_eq!(store.get_setting("k").unwrap().as_deref(), Some("v2"));

    store.set_onboarding_completed(true).unwrap();
    assert_eq!(
        store
            .get_setting("onboarding_completed")
            .unwrap()
            .as_deref(),
        Some("true")
    );
    store.set_onboarding_completed(false).unwrap();
    assert_eq!(
        store
            .get_setting("onboarding_completed")
            .unwrap()
            .as_deref(),
        Some("false")
    );
}

#[test]
fn skills_upsert_list_get_delete() {
    let (_dir, store) = make_store();

    let a = make_skill("a", "A", "/central/a", 10);
    let b = make_skill("b", "B", "/central/b", 20);
    store.upsert_skill(&a).unwrap();
    store.upsert_skill(&b).unwrap();

    let listed = store.list_skills().unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, "b");
    assert_eq!(listed[1].id, "a");

    let got = store.get_skill_by_id("a").unwrap().unwrap();
    assert_eq!(got.name, "A");

    let mut a2 = a.clone();
    a2.name = "A2".to_string();
    a2.updated_at = 30;
    store.upsert_skill(&a2).unwrap();
    assert_eq!(store.get_skill_by_id("a").unwrap().unwrap().name, "A2");
    assert_eq!(store.list_skills().unwrap()[0].id, "a");

    store.delete_skill("a").unwrap();
    assert!(store.get_skill_by_id("a").unwrap().is_none());
}

#[test]
fn skill_targets_upsert_unique_constraint_and_list_order() {
    let (_dir, store) = make_store();
    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    let t1 = AssetTargetRecord {
        id: "t1".to_string(),
        asset_id: "s1".to_string(),
        tool: "cursor".to_string(),
        target_path: "/target/1".to_string(),
        sync_mode: "copy".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
    };
    store.upsert_skill_target(&t1).unwrap();
    assert_eq!(
        store
            .get_skill_target("s1", "cursor")
            .unwrap()
            .unwrap()
            .target_path,
        "/target/1"
    );

    let mut t1b = t1.clone();
    t1b.id = "t2".to_string();
    t1b.target_path = "/target/2".to_string();
    store.upsert_skill_target(&t1b).unwrap();
    assert_eq!(
        store.get_skill_target("s1", "cursor").unwrap().unwrap().id,
        "t1",
        "unique(asset_id, tool) conflict should update existing row, not replace id"
    );
    assert_eq!(
        store
            .get_skill_target("s1", "cursor")
            .unwrap()
            .unwrap()
            .target_path,
        "/target/2"
    );

    let t2 = AssetTargetRecord {
        id: "t3".to_string(),
        asset_id: "s1".to_string(),
        tool: "claude_code".to_string(),
        target_path: "/target/cc".to_string(),
        sync_mode: "copy".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
    };
    store.upsert_skill_target(&t2).unwrap();

    let targets = store.list_skill_targets("s1").unwrap();
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0].tool, "claude_code");
    assert_eq!(targets[1].tool, "cursor");

    store.delete_skill_target("s1", "cursor").unwrap();
    assert!(store.get_skill_target("s1", "cursor").unwrap().is_none());
}

#[test]
fn deleting_skill_cascades_targets() {
    let (_dir, store) = make_store();
    let skill = make_skill("s1", "S1", "/central/s1", 1);
    store.upsert_skill(&skill).unwrap();

    let t = AssetTargetRecord {
        id: "t1".to_string(),
        asset_id: "s1".to_string(),
        tool: "cursor".to_string(),
        target_path: "/target/1".to_string(),
        sync_mode: "copy".to_string(),
        status: "ok".to_string(),
        last_error: None,
        synced_at: None,
    };
    store.upsert_skill_target(&t).unwrap();
    assert_eq!(store.list_skill_targets("s1").unwrap().len(), 1);

    store.delete_skill("s1").unwrap();
    assert_eq!(store.list_skill_targets("s1").unwrap().len(), 0);
}

#[test]
fn description_stored_and_retrieved() {
    let (_dir, store) = make_store();
    let mut skill = make_skill("d1", "D1", "/central/d1", 1);
    skill.description = Some("A test skill description".to_string());
    store.upsert_skill(&skill).unwrap();

    let got = store.get_skill_by_id("d1").unwrap().unwrap();
    assert_eq!(got.description.as_deref(), Some("A test skill description"));
}

#[test]
fn description_null_by_default() {
    let (_dir, store) = make_store();
    let skill = make_skill("d2", "D2", "/central/d2", 1);
    store.upsert_skill(&skill).unwrap();

    let got = store.get_skill_by_id("d2").unwrap().unwrap();
    assert!(got.description.is_none());
}

#[test]
fn update_skill_description_backfills() {
    let (_dir, store) = make_store();
    let skill = make_skill("d3", "D3", "/central/d3", 1);
    store.upsert_skill(&skill).unwrap();

    assert!(store
        .get_skill_by_id("d3")
        .unwrap()
        .unwrap()
        .description
        .is_none());

    store
        .update_skill_description("d3", Some("backfilled"))
        .unwrap();
    assert_eq!(
        store
            .get_skill_by_id("d3")
            .unwrap()
            .unwrap()
            .description
            .as_deref(),
        Some("backfilled")
    );
}

#[test]
fn list_skills_missing_description_filters_correctly() {
    let (_dir, store) = make_store();

    let s1 = make_skill("m1", "M1", "/central/m1", 1);
    store.upsert_skill(&s1).unwrap();

    let mut s2 = make_skill("m2", "M2", "/central/m2", 2);
    s2.description = Some("has desc".to_string());
    store.upsert_skill(&s2).unwrap();

    let missing = store.list_skills_missing_description().unwrap();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].id, "m1");
}

#[test]
fn error_context_includes_db_path() {
    let store = SkillStore::new(PathBuf::from("/this/path/should/not/exist/test.db"));
    let err = store.ensure_schema().unwrap_err();
    let msg = format!("{:#}", err);
    assert!(msg.contains("failed to open db at"), "{msg}");
}

#[test]
fn v4_migration_from_v3_database() {
    // Create a V3 database manually, then run ensure_schema to migrate it to V4.
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("v3.db");

    // Bootstrap a V3 schema
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE skills (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              source_type TEXT NOT NULL,
              source_ref TEXT NULL,
              source_revision TEXT NULL,
              central_path TEXT NOT NULL UNIQUE,
              content_hash TEXT NULL,
              created_at INTEGER NOT NULL,
              updated_at INTEGER NOT NULL,
              last_sync_at INTEGER NULL,
              last_seen_at INTEGER NOT NULL,
              status TEXT NOT NULL
            );
            CREATE TABLE skill_targets (
              id TEXT PRIMARY KEY,
              skill_id TEXT NOT NULL,
              tool TEXT NOT NULL,
              target_path TEXT NOT NULL,
              mode TEXT NOT NULL,
              status TEXT NOT NULL,
              last_error TEXT NULL,
              synced_at INTEGER NULL,
              UNIQUE(skill_id, tool),
              FOREIGN KEY(skill_id) REFERENCES skills(id) ON DELETE CASCADE
            );
            CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE discovered_skills (
              id TEXT PRIMARY KEY, tool TEXT NOT NULL, found_path TEXT NOT NULL,
              name_guess TEXT NULL, fingerprint TEXT NULL, found_at INTEGER NOT NULL,
              imported_skill_id TEXT NULL,
              FOREIGN KEY(imported_skill_id) REFERENCES skills(id) ON DELETE SET NULL
            );
            "#,
        )
        .unwrap();
        // V2
        conn.execute_batch("ALTER TABLE skills ADD COLUMN description TEXT NULL;")
            .unwrap();
        // V3
        conn.execute_batch("ALTER TABLE skills ADD COLUMN source_subpath TEXT NULL;")
            .unwrap();
        conn.pragma_update(None, "user_version", 3).unwrap();

        // Insert some V3 data
        conn.execute(
            "INSERT INTO skills (id, name, source_type, source_ref, source_subpath, source_revision, central_path, content_hash, created_at, updated_at, last_sync_at, last_seen_at, status, description)
             VALUES ('s1', 'MySkill', 'local', '/src', NULL, NULL, '/central/s1', NULL, 1, 2, NULL, 1, 'ok', 'a desc')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO skill_targets (id, skill_id, tool, target_path, mode, status, last_error, synced_at)
             VALUES ('t1', 's1', 'cursor', '/target/1', 'copy', 'ok', NULL, NULL)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO discovered_skills (id, tool, found_path, name_guess, fingerprint, found_at, imported_skill_id)
             VALUES ('d1', 'cursor', '/found/1', 'guess', NULL, 100, 's1')",
            [],
        ).unwrap();
    }

    // Now open via SkillStore and migrate
    let store = SkillStore::new(db_path.clone());
    store.ensure_schema().expect("V4 migration should succeed");

    // Verify data migrated correctly
    let assets = store.list_assets(None).unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].id, "s1");
    assert_eq!(assets[0].name, "MySkill");
    assert_eq!(assets[0].asset_type, "skill");
    assert_eq!(assets[0].central_path.as_deref(), Some("/central/s1"));
    assert_eq!(assets[0].description.as_deref(), Some("a desc"));
    assert_eq!(assets[0].security_status.as_deref(), Some("unchecked"));

    let targets = store.list_asset_targets("s1").unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].asset_id, "s1");
    assert_eq!(targets[0].sync_mode, "copy");

    // Verify old tables are dropped
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let old_skills: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='skills';",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(old_skills, 0, "old skills table should be dropped");

        let old_targets: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='skill_targets';",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(old_targets, 0, "old skill_targets table should be dropped");

        let old_discovered: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='discovered_skills';",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            old_discovered, 0,
            "old discovered_skills table should be dropped"
        );

        // Verify new tables exist
        let new_tables: Vec<String> = {
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };
        assert!(new_tables.contains(&"assets".to_string()));
        assert!(new_tables.contains(&"asset_targets".to_string()));
        assert!(new_tables.contains(&"discovered_assets".to_string()));
        assert!(new_tables.contains(&"libraries".to_string()));
        assert!(new_tables.contains(&"library_items".to_string()));
        assert!(new_tables.contains(&"deploy_profiles".to_string()));

        // Verify user_version is 4
        let version: i32 = conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 4);
    }
}

#[test]
fn mcp_plugin_asset_with_no_central_path() {
    let (_dir, store) = make_store();

    let mcp = AssetRecord {
        id: "mcp1".to_string(),
        name: "my-mcp-server".to_string(),
        description: Some("An MCP server".to_string()),
        asset_type: "mcp_server".to_string(),
        source_type: "registry".to_string(),
        source_ref: Some("@modelcontextprotocol/server-github".to_string()),
        source_subpath: None,
        source_revision: None,
        central_path: None,
        config_json: Some(
            r#"{"command":"npx","args":["-y","@modelcontextprotocol/server-github"]}"#.to_string(),
        ),
        security_status: Some("approved".to_string()),
        content_hash: None,
        created_at: 1,
        updated_at: 2,
        last_sync_at: None,
        last_seen_at: 1,
        status: "ok".to_string(),
    };

    store.upsert_asset(&mcp).unwrap();

    let got = store.get_asset("mcp1").unwrap().unwrap();
    assert_eq!(got.asset_type, "mcp_server");
    assert!(got.central_path.is_none());
    assert!(got.config_json.is_some());
    assert_eq!(got.security_status.as_deref(), Some("approved"));

    // list_assets with filter
    let all = store.list_assets(None).unwrap();
    assert_eq!(all.len(), 1);
    let mcp_only = store.list_assets(Some("mcp_server")).unwrap();
    assert_eq!(mcp_only.len(), 1);
    let skills_only = store.list_assets(Some("skill")).unwrap();
    assert_eq!(skills_only.len(), 0);
}

#[test]
fn list_assets_filters_by_type() {
    let (_dir, store) = make_store();

    let skill = make_skill("s1", "Skill1", "/central/s1", 10);
    store.upsert_asset(&skill).unwrap();

    let mut plugin = make_skill("p1", "Plugin1", "/central/p1", 20);
    plugin.asset_type = "plugin".to_string();
    store.upsert_asset(&plugin).unwrap();

    let all = store.list_assets(None).unwrap();
    assert_eq!(all.len(), 2);

    let skills = store.list_assets(Some("skill")).unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].id, "s1");

    let plugins = store.list_assets(Some("plugin")).unwrap();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0].id, "p1");
}
