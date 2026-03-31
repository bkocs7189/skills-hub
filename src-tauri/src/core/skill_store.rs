use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use tauri::Manager;

const DB_FILE_NAME: &str = "skills_hub.db";
const LEGACY_APP_IDENTIFIERS: &[&str] = &["com.tauri.dev", "com.tauri.dev.skillshub"];

// Schema versioning: bump when making changes and add a migration step.
const SCHEMA_VERSION: i32 = 4;

// Minimal schema for MVP: skills, skill_targets, settings, discovered_skills(optional).
const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS skills (
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

CREATE TABLE IF NOT EXISTS skill_targets (
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

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS discovered_skills (
  id TEXT PRIMARY KEY,
  tool TEXT NOT NULL,
  found_path TEXT NOT NULL,
  name_guess TEXT NULL,
  fingerprint TEXT NULL,
  found_at INTEGER NOT NULL,
  imported_skill_id TEXT NULL,
  FOREIGN KEY(imported_skill_id) REFERENCES skills(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);
CREATE INDEX IF NOT EXISTS idx_skills_updated_at ON skills(updated_at);
"#;

#[derive(Clone, Debug)]
pub struct SkillStore {
    db_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct AssetRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub asset_type: String,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub source_subpath: Option<String>,
    pub source_revision: Option<String>,
    pub central_path: Option<String>,
    pub config_json: Option<String>,
    pub security_status: Option<String>,
    pub content_hash: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_sync_at: Option<i64>,
    pub last_seen_at: i64,
    pub status: String,
}

/// Backward-compatible alias.
pub type SkillRecord = AssetRecord;

#[derive(Clone, Debug)]
pub struct AssetTargetRecord {
    pub id: String,
    pub asset_id: String,
    pub tool: String,
    pub target_path: String,
    pub sync_mode: String,
    pub status: String,
    pub last_error: Option<String>,
    pub synced_at: Option<i64>,
}

/// Backward-compatible alias.
pub type SkillTargetRecord = AssetTargetRecord;

impl SkillStore {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    #[allow(dead_code)]
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn ensure_schema(&self) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;

            let user_version: i32 = conn.query_row("PRAGMA user_version;", [], |row| row.get(0))?;
            if user_version == 0 {
                conn.execute_batch(SCHEMA_V1)?;
                // V2: add description column
                conn.execute_batch("ALTER TABLE skills ADD COLUMN description TEXT NULL;")?;
                // V3: add source_subpath column
                conn.execute_batch("ALTER TABLE skills ADD COLUMN source_subpath TEXT NULL;")?;
                // V4: rename tables and add new columns/tables
                migrate_v3_to_v4(conn)?;
                conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            } else if user_version < SCHEMA_VERSION {
                // Incremental migrations
                if user_version < 2 {
                    conn.execute_batch("ALTER TABLE skills ADD COLUMN description TEXT NULL;")?;
                }
                if user_version < 3 {
                    conn.execute_batch("ALTER TABLE skills ADD COLUMN source_subpath TEXT NULL;")?;
                }
                if user_version < 4 {
                    migrate_v3_to_v4(conn)?;
                }
                conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            } else if user_version > SCHEMA_VERSION {
                anyhow::bail!(
                    "database schema version {} is newer than app supports {}",
                    user_version,
                    SCHEMA_VERSION
                );
            }

            Ok(())
        })
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
            let mut rows = stmt.query(params![key])?;
            Ok(rows
                .next()?
                .map(|row| row.get::<_, String>(0))
                .transpose()?)
        })
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
            Ok(())
        })
    }

    #[allow(dead_code)]
    pub fn set_onboarding_completed(&self, completed: bool) -> Result<()> {
        self.set_setting(
            "onboarding_completed",
            if completed { "true" } else { "false" },
        )
    }

    pub fn upsert_asset(&self, record: &AssetRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO assets (
          id, name, description, asset_type, source_type, source_ref, source_subpath, source_revision,
          central_path, config_json, security_status, content_hash,
          created_at, updated_at, last_sync_at, last_seen_at, status
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
          ?9, ?10, ?11, ?12,
          ?13, ?14, ?15, ?16, ?17
        )
        ON CONFLICT(id) DO UPDATE SET
          name = excluded.name,
          description = excluded.description,
          asset_type = excluded.asset_type,
          source_type = excluded.source_type,
          source_ref = excluded.source_ref,
          source_subpath = excluded.source_subpath,
          source_revision = excluded.source_revision,
          central_path = excluded.central_path,
          config_json = excluded.config_json,
          security_status = excluded.security_status,
          content_hash = excluded.content_hash,
          created_at = excluded.created_at,
          updated_at = excluded.updated_at,
          last_sync_at = excluded.last_sync_at,
          last_seen_at = excluded.last_seen_at,
          status = excluded.status",
                params![
                    record.id,
                    record.name,
                    record.description,
                    record.asset_type,
                    record.source_type,
                    record.source_ref,
                    record.source_subpath,
                    record.source_revision,
                    record.central_path,
                    record.config_json,
                    record.security_status,
                    record.content_hash,
                    record.created_at,
                    record.updated_at,
                    record.last_sync_at,
                    record.last_seen_at,
                    record.status
                ],
            )?;
            Ok(())
        })
    }

    /// Backward-compatible wrapper.
    pub fn upsert_skill(&self, record: &AssetRecord) -> Result<()> {
        self.upsert_asset(record)
    }

    pub fn upsert_asset_target(&self, record: &AssetTargetRecord) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO asset_targets (
          id, asset_id, tool, target_path, sync_mode, status, last_error, synced_at
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8
        )
        ON CONFLICT(asset_id, tool) DO UPDATE SET
          target_path = excluded.target_path,
          sync_mode = excluded.sync_mode,
          status = excluded.status,
          last_error = excluded.last_error,
          synced_at = excluded.synced_at",
                params![
                    record.id,
                    record.asset_id,
                    record.tool,
                    record.target_path,
                    record.sync_mode,
                    record.status,
                    record.last_error,
                    record.synced_at
                ],
            )?;
            Ok(())
        })
    }

    /// Backward-compatible wrapper.
    pub fn upsert_skill_target(&self, record: &AssetTargetRecord) -> Result<()> {
        self.upsert_asset_target(record)
    }

    pub fn list_assets(&self, asset_type: Option<&str>) -> Result<Vec<AssetRecord>> {
        self.with_conn(|conn| {
            let (sql, do_bind) = match asset_type {
                Some(_) => (
                    "SELECT id, name, description, asset_type, source_type, source_ref, source_subpath, source_revision,
                            central_path, config_json, security_status, content_hash,
                            created_at, updated_at, last_sync_at, last_seen_at, status
                     FROM assets
                     WHERE asset_type = ?1
                     ORDER BY updated_at DESC",
                    true,
                ),
                None => (
                    "SELECT id, name, description, asset_type, source_type, source_ref, source_subpath, source_revision,
                            central_path, config_json, security_status, content_hash,
                            created_at, updated_at, last_sync_at, last_seen_at, status
                     FROM assets
                     ORDER BY updated_at DESC",
                    false,
                ),
            };
            let mut stmt = conn.prepare(sql)?;
            let rows = if do_bind {
                stmt.query_map(params![asset_type.unwrap()], row_to_asset)?
            } else {
                stmt.query_map([], row_to_asset)?
            };

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    /// Backward-compatible wrapper — lists all assets (no filter).
    pub fn list_skills(&self) -> Result<Vec<AssetRecord>> {
        self.list_assets(None)
    }

    pub fn get_asset(&self, asset_id: &str) -> Result<Option<AssetRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, asset_type, source_type, source_ref, source_subpath, source_revision,
                        central_path, config_json, security_status, content_hash,
                        created_at, updated_at, last_sync_at, last_seen_at, status
                 FROM assets
                 WHERE id = ?1
                 LIMIT 1",
            )?;
            let mut rows = stmt.query(params![asset_id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(row_to_asset(row)?))
            } else {
                Ok(None)
            }
        })
    }

    /// Backward-compatible wrapper.
    pub fn get_skill_by_id(&self, skill_id: &str) -> Result<Option<AssetRecord>> {
        self.get_asset(skill_id)
    }

    pub fn update_skill_description(
        &self,
        skill_id: &str,
        description: Option<&str>,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE assets SET description = ?1 WHERE id = ?2",
                params![description, skill_id],
            )?;
            Ok(())
        })
    }

    pub fn list_skills_missing_description(&self) -> Result<Vec<AssetRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, asset_type, source_type, source_ref, source_subpath, source_revision,
                        central_path, config_json, security_status, content_hash,
                        created_at, updated_at, last_sync_at, last_seen_at, status
                 FROM assets
                 WHERE description IS NULL",
            )?;
            let rows = stmt.query_map([], row_to_asset)?;
            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    pub fn delete_asset(&self, asset_id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM assets WHERE id = ?1", params![asset_id])?;
            Ok(())
        })
    }

    /// Backward-compatible wrapper.
    pub fn delete_skill(&self, skill_id: &str) -> Result<()> {
        self.delete_asset(skill_id)
    }

    pub fn list_asset_targets(&self, asset_id: &str) -> Result<Vec<AssetTargetRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, asset_id, tool, target_path, sync_mode, status, last_error, synced_at
         FROM asset_targets
         WHERE asset_id = ?1
         ORDER BY tool ASC",
            )?;
            let rows = stmt.query_map(params![asset_id], |row| {
                Ok(AssetTargetRecord {
                    id: row.get(0)?,
                    asset_id: row.get(1)?,
                    tool: row.get(2)?,
                    target_path: row.get(3)?,
                    sync_mode: row.get(4)?,
                    status: row.get(5)?,
                    last_error: row.get(6)?,
                    synced_at: row.get(7)?,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    /// Backward-compatible wrapper.
    pub fn list_skill_targets(&self, skill_id: &str) -> Result<Vec<AssetTargetRecord>> {
        self.list_asset_targets(skill_id)
    }

    pub fn list_all_skill_target_paths(&self) -> Result<Vec<(String, String)>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT tool, target_path
         FROM asset_targets",
            )?;
            let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    pub fn get_skill_target(
        &self,
        skill_id: &str,
        tool: &str,
    ) -> Result<Option<AssetTargetRecord>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, asset_id, tool, target_path, sync_mode, status, last_error, synced_at
         FROM asset_targets
         WHERE asset_id = ?1 AND tool = ?2",
            )?;
            let mut rows = stmt.query(params![skill_id, tool])?;
            if let Some(row) = rows.next()? {
                Ok(Some(AssetTargetRecord {
                    id: row.get(0)?,
                    asset_id: row.get(1)?,
                    tool: row.get(2)?,
                    target_path: row.get(3)?,
                    sync_mode: row.get(4)?,
                    status: row.get(5)?,
                    last_error: row.get(6)?,
                    synced_at: row.get(7)?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub fn delete_skill_target(&self, skill_id: &str, tool: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM asset_targets WHERE asset_id = ?1 AND tool = ?2",
                params![skill_id, tool],
            )?;
            Ok(())
        })
    }

    fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("failed to open db at {:?}", self.db_path))?;
        // Enforce foreign key constraints on every connection (rusqlite PRAGMA is per-connection).
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        f(&conn)
    }
}

/// Helper to map a row from the assets table into an AssetRecord.
fn row_to_asset(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetRecord> {
    Ok(AssetRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        asset_type: row.get(3)?,
        source_type: row.get(4)?,
        source_ref: row.get(5)?,
        source_subpath: row.get(6)?,
        source_revision: row.get(7)?,
        central_path: row.get(8)?,
        config_json: row.get(9)?,
        security_status: row.get(10)?,
        content_hash: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        last_sync_at: row.get(14)?,
        last_seen_at: row.get(15)?,
        status: row.get(16)?,
    })
}

/// Migrate from V3 schema (skills/skill_targets/discovered_skills) to V4
/// (assets/asset_targets/discovered_assets + new tables).
fn migrate_v3_to_v4(conn: &Connection) -> Result<()> {
    // Temporarily disable FK checks so we can safely migrate data across tables
    // without cascade deletes firing when we drop old tables.
    conn.execute_batch("PRAGMA foreign_keys = OFF;")?;

    // --- 1. Rename skills → assets, add new columns ---
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS assets (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          description TEXT NULL,
          asset_type TEXT NOT NULL DEFAULT 'skill',
          source_type TEXT NOT NULL,
          source_ref TEXT NULL,
          source_subpath TEXT NULL,
          source_revision TEXT NULL,
          central_path TEXT UNIQUE,
          config_json TEXT,
          security_status TEXT DEFAULT 'unchecked',
          content_hash TEXT NULL,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL,
          last_sync_at INTEGER NULL,
          last_seen_at INTEGER NOT NULL,
          status TEXT NOT NULL
        );
        "#,
    )?;

    // Migrate existing data if `skills` table exists
    let has_skills: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='skills';",
        [],
        |row| row.get(0),
    )?;
    if has_skills > 0 {
        conn.execute_batch(
            r#"
            INSERT OR IGNORE INTO assets
              (id, name, description, asset_type, source_type, source_ref, source_subpath, source_revision,
               central_path, config_json, security_status, content_hash,
               created_at, updated_at, last_sync_at, last_seen_at, status)
            SELECT
              id, name, description, 'skill', source_type, source_ref, source_subpath, source_revision,
              central_path, NULL, 'unchecked', content_hash,
              created_at, updated_at, last_sync_at, last_seen_at, status
            FROM skills;
            "#,
        )?;
    }

    // --- 2. Recreate skill_targets → asset_targets ---
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS asset_targets (
          id TEXT PRIMARY KEY,
          asset_id TEXT NOT NULL,
          tool TEXT NOT NULL,
          target_path TEXT NOT NULL,
          sync_mode TEXT NOT NULL,
          status TEXT NOT NULL,
          last_error TEXT NULL,
          synced_at INTEGER NULL,
          UNIQUE(asset_id, tool),
          FOREIGN KEY(asset_id) REFERENCES assets(id) ON DELETE CASCADE
        );
        "#,
    )?;

    let has_skill_targets: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='skill_targets';",
        [],
        |row| row.get(0),
    )?;
    if has_skill_targets > 0 {
        conn.execute_batch(
            r#"
            INSERT OR IGNORE INTO asset_targets
              (id, asset_id, tool, target_path, sync_mode, status, last_error, synced_at)
            SELECT
              id, skill_id, tool, target_path, mode, status, last_error, synced_at
            FROM skill_targets;
            "#,
        )?;
    }

    // --- 3. Recreate discovered_skills → discovered_assets ---
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS discovered_assets (
          id TEXT PRIMARY KEY,
          tool TEXT NOT NULL,
          found_path TEXT NOT NULL,
          name_guess TEXT NULL,
          fingerprint TEXT NULL,
          found_at INTEGER NOT NULL,
          imported_asset_id TEXT NULL,
          FOREIGN KEY(imported_asset_id) REFERENCES assets(id) ON DELETE SET NULL
        );
        "#,
    )?;

    let has_discovered_skills: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='discovered_skills';",
        [],
        |row| row.get(0),
    )?;
    if has_discovered_skills > 0 {
        conn.execute_batch(
            r#"
            INSERT OR IGNORE INTO discovered_assets
              (id, tool, found_path, name_guess, fingerprint, found_at, imported_asset_id)
            SELECT
              id, tool, found_path, name_guess, fingerprint, found_at, imported_skill_id
            FROM discovered_skills;
            "#,
        )?;
    }

    // Drop old tables now that all data has been migrated
    conn.execute_batch(
        r#"
        DROP TABLE IF EXISTS skill_targets;
        DROP TABLE IF EXISTS discovered_skills;
        DROP TABLE IF EXISTS skills;
        "#,
    )?;

    // Recreate indexes on new table
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_assets_name ON assets(name);
        CREATE INDEX IF NOT EXISTS idx_assets_updated_at ON assets(updated_at);
        "#,
    )?;

    // Re-enable FK checks
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    // --- 4. Create new tables ---
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS libraries (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          url TEXT NOT NULL UNIQUE,
          library_type TEXT NOT NULL,
          asset_types TEXT NOT NULL,
          trusted INTEGER NOT NULL DEFAULT 0,
          last_indexed_at INTEGER NULL,
          item_count INTEGER NOT NULL DEFAULT 0,
          status TEXT NOT NULL DEFAULT 'active'
        );

        CREATE TABLE IF NOT EXISTS library_items (
          id TEXT PRIMARY KEY,
          library_id TEXT NOT NULL,
          asset_type TEXT NOT NULL,
          name TEXT NOT NULL,
          description TEXT NULL,
          subpath TEXT NOT NULL,
          metadata_json TEXT NULL,
          indexed_at INTEGER NOT NULL,
          FOREIGN KEY(library_id) REFERENCES libraries(id) ON DELETE CASCADE
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_library_items_lib_subpath ON library_items(library_id, subpath);

        CREATE TABLE IF NOT EXISTS deploy_profiles (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          is_default INTEGER NOT NULL DEFAULT 0,
          rules TEXT NOT NULL DEFAULT '[]'
        );
        "#,
    )?;

    Ok(())
}

pub fn default_db_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf> {
    let app_dir = app
        .path()
        .app_data_dir()
        .context("failed to resolve app data dir")?;
    std::fs::create_dir_all(&app_dir)
        .with_context(|| format!("failed to create app data dir {:?}", app_dir))?;
    Ok(app_dir.join(DB_FILE_NAME))
}

pub fn migrate_legacy_db_if_needed(target_db_path: &Path) -> Result<()> {
    let Some(data_dir) = dirs::data_dir() else {
        return Ok(());
    };

    if let Ok(true) = db_has_any_data(target_db_path) {
        return Ok(());
    }

    let legacy_db_path = LEGACY_APP_IDENTIFIERS
        .iter()
        .map(|id| data_dir.join(id).join(DB_FILE_NAME))
        .find(|path| path.exists());

    let Some(legacy_db_path) = legacy_db_path else {
        return Ok(());
    };

    if legacy_db_path == target_db_path {
        return Ok(());
    }

    if let Some(parent) = target_db_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create app data dir {:?}", parent))?;
    }

    if target_db_path.exists() {
        let backup = target_db_path.with_extension(format!(
            "bak-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ));
        std::fs::rename(target_db_path, &backup).with_context(|| {
            format!(
                "failed to backup existing db {:?} -> {:?}",
                target_db_path, backup
            )
        })?;
    }

    std::fs::copy(&legacy_db_path, target_db_path).with_context(|| {
        format!(
            "failed to migrate legacy db {:?} -> {:?}",
            legacy_db_path, target_db_path
        )
    })?;

    Ok(())
}

/// Check whether the database has any data (checks both legacy `skills` and new `assets` table names).
fn db_has_any_data(db_path: &Path) -> Result<bool> {
    if !db_path.exists() {
        return Ok(false);
    }

    let conn =
        Connection::open(db_path).with_context(|| format!("failed to open db at {:?}", db_path))?;

    // Check new `assets` table first
    let has_assets: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='assets';",
        [],
        |row| row.get(0),
    )?;
    if has_assets > 0 {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM assets;", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(true);
        }
    }

    // Fall back to legacy `skills` table
    let has_skills: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='skills';",
        [],
        |row| row.get(0),
    )?;
    if has_skills > 0 {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM skills;", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
#[path = "tests/skill_store.rs"]
mod tests;
