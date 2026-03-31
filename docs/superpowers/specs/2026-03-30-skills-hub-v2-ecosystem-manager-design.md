# Skills Hub v2: AI Tool Ecosystem Manager

**Date:** 2026-03-30
**Status:** Approved
**Author:** Brett Kelsey + Claude

## Context

Skills Hub v0.4.1 manages AI agent skill files — install once, sync everywhere across 47 tools. This design extends it into a comprehensive AI tool ecosystem manager covering skills, MCP servers, plugins, and executables. The motivation:

- MCP server configs are scattered across tools (15 in Claude Code, 6 in Cursor, 0 in Claude Desktop) with no coordination.
- Claude Code's plugin ecosystem (48 plugins, 10 marketplaces, 380MB) has no visibility, health monitoring, or cross-tool awareness.
- Plugin installation errors (28 load errors observed during this session) have no diagnostic tooling.
- Claude Desktop App is not tracked as a deployment target.
- No security validation exists before installing third-party skills, plugins, or MCP servers.

## Architecture: Unified Entity Model

All managed items — skills, MCP servers, plugins, executables — are "assets" with different `asset_type` values. They share a single install → central registry → sync-to-tools pipeline, with type-specific adapters for storage and sync strategies.

## Data Model

### `assets` table (replaces `skills`)

| Column | Type | Notes |
|--------|------|-------|
| id | TEXT PK | UUID |
| asset_type | TEXT NOT NULL | `skill`, `mcp_server`, `plugin`, `executable` |
| name | TEXT NOT NULL | Display name |
| description | TEXT | From SKILL.md, PLUGIN.md, or user-provided |
| source_type | TEXT | `local`, `git`, `marketplace`, `cli_import` |
| source_ref | TEXT | URL, path, or marketplace identifier |
| source_subpath | TEXT | For git repos with multiple assets |
| source_revision | TEXT | Git SHA or version |
| central_path | TEXT UNIQUE | Path in central repo (skills/executables) or NULL (MCP/plugins managed externally) |
| config_json | TEXT | Type-specific config. MCP: `{command, args, env}`. Plugin: `{marketplace, scope, cli_path}` |
| content_hash | TEXT | SHA256 for change detection |
| security_status | TEXT DEFAULT 'unchecked' | `unchecked`, `trusted`, `unknown_source`, `flagged`, `deep_scanned` |
| created_at | INTEGER NOT NULL | Unix ms |
| updated_at | INTEGER NOT NULL | Unix ms |
| last_sync_at | INTEGER | Unix ms |
| last_seen_at | INTEGER NOT NULL | Unix ms |
| status | TEXT NOT NULL | `ok`, `error`, `disabled` |

### `asset_targets` table (replaces `skill_targets`)

| Column | Type | Notes |
|--------|------|-------|
| id | TEXT PK | UUID |
| asset_id | TEXT NOT NULL FK | References assets.id |
| tool | TEXT NOT NULL | Tool key (e.g., `claude_code`, `cursor`) |
| target_path | TEXT NOT NULL | Absolute path or config file path |
| sync_mode | TEXT NOT NULL | `symlink`, `junction`, `copy`, `json_merge`, `cli_command` |
| status | TEXT NOT NULL | `ok`, `error` |
| last_error | TEXT | Error message |
| synced_at | INTEGER | Unix ms |
| UNIQUE(asset_id, tool) | | One target per asset+tool |

### `libraries` table (new)

| Column | Type | Notes |
|--------|------|-------|
| id | TEXT PK | UUID |
| name | TEXT NOT NULL | Display name |
| url | TEXT NOT NULL UNIQUE | Git URL or API endpoint |
| library_type | TEXT NOT NULL | `marketplace`, `github_repo`, `curated_list` |
| asset_types | TEXT | JSON array: `["skill", "plugin", "mcp_server"]` |
| trusted | BOOLEAN DEFAULT 0 | Trusted publisher (skip security gate) |
| last_indexed_at | INTEGER | Unix ms |
| item_count | INTEGER | Assets available |
| status | TEXT NOT NULL | `ok`, `error`, `indexing` |

### `library_items` table (new)

| Column | Type | Notes |
|--------|------|-------|
| id | TEXT PK | UUID |
| library_id | TEXT NOT NULL FK | References libraries.id |
| asset_type | TEXT NOT NULL | `skill`, `plugin`, `mcp_server` |
| name | TEXT NOT NULL | Display name |
| description | TEXT | Parsed from metadata |
| subpath | TEXT | Path within the library |
| metadata_json | TEXT | Stars, downloads, author, tags |
| indexed_at | INTEGER NOT NULL | Unix ms |

### `deploy_profiles` table (new)

| Column | Type | Notes |
|--------|------|-------|
| id | TEXT PK | UUID |
| name | TEXT NOT NULL | "My Default", "Work Setup" |
| is_default | BOOLEAN DEFAULT 0 | Auto-apply on install |
| rules | TEXT NOT NULL | JSON: `{"skill": ["claude_code", "cursor"], "mcp_server": ["claude_code", "claude_desktop"]}` |

### Migration: V3 → V4

**Important:** SQLite `ALTER TABLE ... RENAME COLUMN` requires SQLite 3.25+. Since rusqlite's bundled version may vary, use the safe approach: create new tables, copy data, drop old tables.

```sql
-- Step 1: Rename skills table
ALTER TABLE skills RENAME TO assets;
ALTER TABLE assets ADD COLUMN asset_type TEXT NOT NULL DEFAULT 'skill';
ALTER TABLE assets ADD COLUMN config_json TEXT;
ALTER TABLE assets ADD COLUMN security_status TEXT DEFAULT 'unchecked';

-- Step 2: Recreate skill_targets as asset_targets with renamed columns
CREATE TABLE asset_targets (
    id TEXT PRIMARY KEY,
    asset_id TEXT NOT NULL,
    tool TEXT NOT NULL,
    target_path TEXT NOT NULL,
    sync_mode TEXT NOT NULL,
    status TEXT NOT NULL,
    last_error TEXT,
    synced_at INTEGER,
    UNIQUE(asset_id, tool) ON CONFLICT REPLACE
);
INSERT INTO asset_targets (id, asset_id, tool, target_path, sync_mode, status, last_error, synced_at)
    SELECT id, skill_id, tool, target_path, mode, status, last_error, synced_at FROM skill_targets;
DROP TABLE skill_targets;

-- Step 3: Fix discovered_skills FK (references old table name)
CREATE TABLE discovered_assets (
    id TEXT PRIMARY KEY,
    tool TEXT NOT NULL,
    found_path TEXT NOT NULL,
    name_guess TEXT,
    fingerprint TEXT,
    found_at INTEGER NOT NULL,
    imported_asset_id TEXT REFERENCES assets(id) ON DELETE SET NULL
);
INSERT INTO discovered_assets (id, tool, found_path, name_guess, fingerprint, found_at, imported_asset_id)
    SELECT id, tool, found_path, name_guess, fingerprint, found_at, imported_skill_id FROM discovered_skills;
DROP TABLE discovered_skills;

-- Step 4: Backfill security_status for existing skills
UPDATE assets SET security_status = 'trusted' WHERE source_type = 'local';

-- Step 5: New tables
CREATE TABLE libraries (...);
CREATE TABLE library_items (...);
CREATE TABLE deploy_profiles (...);

-- Step 6: Indexes for library search
CREATE INDEX idx_library_items_library_type ON library_items(library_id, asset_type);
CREATE UNIQUE INDEX idx_library_items_dedup ON library_items(library_id, subpath);
```

### Migration Safety Notes

- `central_path` changes from `TEXT NOT NULL` to `TEXT` (nullable) for MCP/plugin assets. The `AssetRecord` Rust struct must use `central_path: Option<String>`.
- `migrate_legacy_db_if_needed()` must check for both `skills` and `assets` table names in `sqlite_master` to handle both old and new schemas.
- `db_has_any_skills()` renamed to `db_has_any_assets()` with dual-table check.
- All commands that dereference `central_path` (e.g., `set_central_repo_path`) must skip assets where `central_path IS NULL`.
- The `SyncMode` enum gains `JsonMerge` and `CliCommand` variants. All `match` blocks on `SyncMode` must handle the new variants.
- `backfill_skill_descriptions()` in `lib.rs` must be updated to query `assets` table with `WHERE asset_type = 'skill'`.

## Sync Architecture

### Unified Pipeline

```
install/deploy(asset, deploy_profile)
    │
    ▼
resolve_targets(asset, profile) → [(tool_id, sync_strategy)]
    │
    ▼
for each target:
    match asset.asset_type {
        "skill" | "executable" → sync_directory(source, target)
                                  // symlink → junction → copy (existing)
        "mcp_server"           → sync_json_config(asset.config_json, tool)
                                  // read config → upsert entry → write back
        "plugin"               → sync_via_cli(asset, tool)
                                  // shell out to tool's plugin CLI
    }
    │
    ▼
upsert asset_targets record
```

### MCP Server JSON Config Sync

1. Read tool's config file (path from adapter's `mcp_config_path`)
2. Create `.bak` backup alongside original
3. Parse JSON, navigate to `mcpServers` key (from adapter's `mcp_config_key`)
4. Upsert the server entry by name
5. Write file back with pretty-print formatting
6. On failure, restore from `.bak`

Removal: same flow but delete the entry instead of upserting.

### Plugin Sync (Claude Code wrapper)

- **Read state:** Parse `~/.claude/plugins/installed_plugins.json` directly (fast, no CLI needed)
- **Write state:** Shell out to `claude` CLI for install/remove operations
- **Diagnostics:** Independently validate plugin files (check PLUGIN.md structure, verify referenced files exist, check for syntax errors in skill/agent definitions)
- **No cross-tool plugin sync in v1** — plugins are Claude Code-specific today

### Config File Safety

Before writing any tool config file (MCP sync):
1. Write `{filename}.bak` backup
2. Validate the output JSON is well-formed before writing
3. If write fails, restore from backup
4. Log the change for audit trail

## Tool Adapter Extensions

```rust
pub struct ToolAdapter {
    // Existing
    pub id: ToolId,
    pub display_name: &'static str,
    pub relative_skills_dir: &'static str,
    pub relative_detect_dir: &'static str,

    // New — all have defaults so existing 42 adapters use ..ToolAdapter::default_extensions()
    pub supports_mcp: bool,
    pub mcp_config_path: Option<&'static str>,
    pub mcp_config_key: Option<&'static str>,
    pub supports_plugins: bool,
    pub plugin_cli: Option<&'static str>,
}

impl ToolAdapter {
    /// Returns default values for the new fields — used by existing adapters
    /// via struct update syntax: ToolAdapter { id, display_name, ..., ..Self::default_extensions() }
    pub const fn default_extensions() -> Self { /* all new fields false/None */ }
}
```

**Initialization pattern for existing adapters:**
```rust
ToolAdapter {
    id: ToolId::Cursor,
    display_name: "Cursor",
    relative_skills_dir: ".cursor/skills",
    relative_detect_dir: ".cursor",
    ..ToolAdapter::default_extensions()
}
```
This avoids modifying all 42 adapter definitions. Only adapters gaining MCP/plugin support need explicit new field values.

### New Adapter: Claude Desktop

- `ToolId::ClaudeDesktop`
- detect_dir: `Library/Application Support/Claude` (macOS)
- skills_dir: `Library/Application Support/Claude/skills`
- mcp_config_path: `Library/Application Support/Claude/claude_desktop_config.json`
- mcp_config_key: `mcpServers`
- supports_mcp: true
- supports_plugins: false

### Tools with MCP support (v1)

| Tool | macOS Config Path | Windows Config Path | Key |
|------|------------------|--------------------|----|
| Claude Code | `.claude/config/config.json` | `.claude/config/config.json` | `mcpServers` |
| Cursor | `.cursor/mcp.json` | `.cursor/mcp.json` | `mcpServers` |
| Claude Desktop | `Library/Application Support/Claude/claude_desktop_config.json` | `AppData/Roaming/Claude/claude_desktop_config.json` | `mcpServers` |

**Note:** The actual Claude Code MCP config path must be verified at runtime. Check both `.claude/config/config.json` and `.claude.json` — the location varies by Claude Code version. Use a fallback resolution: try `config/config.json` first, then `.claude.json`.

### MCP Config JSON Schema (canonical type)

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}
```

All MCP configs are validated against this type on import and before writing to tool config files.

### Deploy Profile Commands (missing from phase list)

```
get_deploy_profiles, create_deploy_profile, update_deploy_profile, delete_deploy_profile, set_default_profile
```

### Library Items Deduplication

`library_items` has a UNIQUE index on `(library_id, subpath)`. Re-indexing uses `INSERT ... ON CONFLICT(library_id, subpath) DO UPDATE` to upsert.

### FTS5 Requirement

`Cargo.toml` must enable FTS5: change `rusqlite` feature from `"bundled"` to `"bundled-full"` (or add `SQLITE_ENABLE_FTS5` compile flag). This is required for Phase 4 library search.

## Library Registry & Explore

### Library Sources

A library is any URL that contains discoverable assets. Three types:
- **marketplace** — Claude Code plugin marketplace (has `PLUGIN.md` files)
- **github_repo** — Any GitHub repo containing skills/plugins/MCP configs
- **curated_list** — JSON/YAML file listing assets (like featured-skills.json)

### Indexing Flow

1. User adds library URL or library is pre-seeded
2. Clone/fetch via existing `git_fetcher`
3. Walk directory tree, detect SKILL.md and PLUGIN.md files
4. Parse metadata, store in `library_items`
5. Periodic re-index (configurable, default daily)

### Pre-seeded Libraries (first run)

- Existing `featured-skills.json` (bundled)
- All marketplaces from `~/.claude/plugins/known_marketplaces.json`
- Each existing marketplace directory becomes a library

### Search

SQLite FTS5 full-text search across `library_items.name` and `library_items.description`. Future: semantic/AI-powered natural language search.

## Security Gate

### Trust Tiers

| Tier | Label | Behavior |
|------|-------|----------|
| **Trusted Publisher** | Green shield | Auto-install, no prompts. Pre-configured: Anthropic official marketplaces |
| **Known Source** | Green shield (after approval) | User has manually marked this library as trusted |
| **Unknown Source** | Yellow shield | Tier 1 auto-scan, Tier 2 offered |
| **Flagged** | Red shield | Failed checks, strong warning |

Trusted publisher list stored in settings, editable from UI. Ships with Anthropic marketplaces pre-trusted: `claude-plugins-official`, `anthropic-agent-skills`, `claude-code-templates`, `claude-canvas`.

### Tier 1: Quick Trust Check (automatic)

- Source reputation: from trusted/known library?
- File type validation: skills should be markdown only, flag hidden executables
- Metadata validation: valid SKILL.md/PLUGIN.md with required fields
- Blocklist check: local blocklist (bundled + user-maintained)
- Result → `security_status` field on asset

### Tier 2: Deep Scan (opt-in)

- If Ghost Security installed: invoke Ghost scan-code pipeline
- Fallback: built-in lightweight scanner (hardcoded URLs, eval/exec patterns, env var exfiltration, encoded payloads)
- Results shown in modal before install confirmation

## UI Design

### Navigation

```
[My Ecosystem]  [Explore]  [Settings]
```

### My Ecosystem Page (replaces My Skills)

- **Filter bar:** `All` | `Skills` | `MCP Servers` | `Plugins` | `Executables`
- **Sort/search:** dropdown + search box
- **Asset cards:** unified design with:
  - Type badge (color-coded: blue=skill, purple=MCP, orange=plugin, green=executable)
  - Name + description
  - Source indicator (git, folder, marketplace icons)
  - Tool sync badges (existing pill pattern)
  - Status indicator (healthy/error/disabled)
  - Security shield (trust tier color)
  - Actions: sync, update, unsync, delete
- **Discovery banner:** scans for skills AND MCP servers AND plugins on first run

### Explore Page (enhanced)

- **Search bar** + type filter chips
- **Library selector:** "All Libraries" or specific library
- **"Manage Libraries" button** → modal to add/remove library URLs
- **Result cards:** name, description, type badge, source library, trust shield, install count
- **Install flow:** click Install → security gate → deploy profile auto-sync

### Settings Page (extended)

Existing settings plus:
- **Deploy Profiles:** default tool targets per asset type, checkboxes per tool
- **Trusted Publishers:** list of auto-trusted library sources, add/remove
- **Tool Status Dashboard:** all detected tools, installed/not, sync summary, health

### New Modals

- **MCP Server Config:** form for name, command, args[], env{} — for manual MCP server creation
- **Security Scan Results:** findings list with severity, file, description
- **Library Management:** add/remove/refresh library URLs
- **Import Conflicts:** side-by-side comparison for MCP servers found in multiple tools

## Onboarding / Import Flow

On first run (or when new tools detected):

1. Scan all tool directories for existing skills (existing behavior)
2. Scan all tool MCP config files for existing MCP servers (new)
3. Read `~/.claude/plugins/installed_plugins.json` for existing plugins (new)
4. Present unified import plan with conflict detection
5. User resolves conflicts (e.g., same MCP server in Claude Code and Cursor with different configs)
6. Import selected items into central `assets` registry
7. Skills Hub becomes the authority going forward

## Implementation Phases

### Phase 1: Foundation (DB + Core + Claude Desktop)
- Migrate `skills` → `assets`, `skill_targets` → `asset_targets`
- Extend ToolAdapter struct with new fields
- Add Claude Desktop adapter
- Add `libraries`, `library_items`, `deploy_profiles` tables
- Update all existing commands to use new table names
- Rename frontend types to match
- Ensure all existing skill functionality works unchanged

### Phase 2: MCP Server Management
- New core module: `mcp_manager.rs` (JSON config read/write/merge)
- New Tauri commands: `get_mcp_servers`, `add_mcp_server`, `sync_mcp_to_tool`, `unsync_mcp_from_tool`, `import_mcp_servers`
- MCP import/onboarding flow (scan existing configs, detect conflicts)
- Frontend: MCP server cards in My Ecosystem, MCP config modal
- Deploy profile support for MCP servers

### Phase 3: Plugin Management
- New core module: `plugin_manager.rs` (read installed_plugins.json, CLI wrapper, diagnostics)
- New Tauri commands: `get_plugins`, `install_plugin`, `remove_plugin`, `diagnose_plugins`
- Plugin health dashboard (status, errors, marketplace info)
- Frontend: plugin cards in My Ecosystem with health indicators
- Plugin diagnostics modal

### Phase 4: Library Registry + Explore
- New core module: `library_manager.rs` (indexing, FTS5 search)
- New Tauri commands: `add_library`, `remove_library`, `index_library`, `search_library_items`
- Pre-seed from existing marketplaces and featured-skills.json
- Enhanced Explore page with library selector, unified search
- Install flow from Explore → security gate → deploy

### Phase 5: Security Gate
- Tier 1: trust tier system, file validation, blocklist checking
- Tier 2: Ghost integration + fallback scanner
- Trusted publisher management in Settings
- Security shield indicators throughout UI

### Phase 6: Deploy Profiles + Polish
- Deploy profile CRUD in Settings
- Auto-deploy on install based on default profile
- Unified onboarding flow (skills + MCP + plugins)
- i18n for all new text (EN + ZH)

## Files to Modify

### Backend (Rust)
- `src-tauri/src/core/skill_store.rs` → rename + extend schema to V4
- `src-tauri/src/core/installer.rs` → generalize for asset types
- `src-tauri/src/core/sync_engine.rs` → add `json_merge` and `cli_command` modes
- `src-tauri/src/core/tool_adapters/mod.rs` → extend struct, add Claude Desktop
- `src-tauri/src/core/onboarding.rs` → scan MCP configs and plugins
- `src-tauri/src/commands/mod.rs` → new commands + rename existing DTOs
- `src-tauri/src/lib.rs` → register new commands
- **New:** `src-tauri/src/core/mcp_manager.rs`
- **New:** `src-tauri/src/core/plugin_manager.rs`
- **New:** `src-tauri/src/core/library_manager.rs`
- **New:** `src-tauri/src/core/security_gate.rs`

### Frontend (React/TypeScript)
- `src/components/skills/types.ts` → rename Skill→Asset types, add MCP/Plugin types
- `src/App.tsx` → new view states, new state variables, new handlers
- `src/components/skills/Header.tsx` → updated nav tabs
- `src/components/skills/SkillCard.tsx` → generalize to AssetCard
- `src/components/skills/SkillsList.tsx` → generalize to AssetList
- `src/components/skills/FilterBar.tsx` → add type filter chips
- `src/components/skills/ExplorePage.tsx` → library selector, unified search
- `src/components/skills/SettingsPage.tsx` → deploy profiles, trusted publishers, tool dashboard
- `src/i18n/resources.ts` → new translation keys (EN + ZH)
- **New:** `src/components/skills/modals/McpConfigModal.tsx`
- **New:** `src/components/skills/modals/SecurityScanModal.tsx`
- **New:** `src/components/skills/modals/LibraryManageModal.tsx`
- **New:** `src/components/skills/modals/ImportConflictModal.tsx`

## Verification

After each phase:
1. `npm run check` — lint, build, Rust fmt/clippy/test all pass
2. `npm run tauri:dev` — manual smoke test of new + existing functionality
3. Verify existing skill install/sync/update/delete flows still work
4. For MCP phases: verify config files are written correctly and tools pick up changes
5. For plugin phases: verify Claude Code CLI integration works
