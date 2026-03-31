# Skills Hub v2: AI Tool Ecosystem Manager — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform Skills Hub from a skill-only file manager into a unified AI tool ecosystem manager supporting skills, MCP servers, plugins, and executables with cross-tool sync, library browsing, security gating, and deploy profiles.

**Architecture:** Unified Entity Model — all managed items are "assets" with type-specific sync strategies. Skills sync as directories (existing), MCP servers sync as JSON config fragments, plugins sync via CLI wrappers. A library registry indexes multiple sources for browsable/searchable asset discovery.

**Tech Stack:** Tauri 2, Rust (rusqlite, serde_json, git2, reqwest), React 19, TypeScript, Tailwind CSS 4, i18next

**Spec:** `docs/superpowers/specs/2026-03-30-skills-hub-v2-ecosystem-manager-design.md`

**Conventions:** See `CLAUDE.md` — run `npm run check` before every commit. Both EN and ZH translations required for all new text.

---

## Phase 1: Foundation (DB Migration + Core Renames + Claude Desktop)

This is the riskiest phase — it renames core types across every layer while preserving all existing functionality. Every subsequent phase depends on this.

### Task 1.1: Extend ToolAdapter Struct

**Files:**
- Modify: `src-tauri/src/core/tool_adapters/mod.rs`
- Test: `src-tauri/src/core/tests/tool_adapters.rs`

- [ ] **Step 1: Add new fields to ToolAdapter with default_extensions()**

Add `supports_mcp`, `mcp_config_path`, `mcp_config_key`, `supports_plugins`, `plugin_cli` fields. Add `default_extensions()` const fn. Update all existing adapter initializations to use `..ToolAdapter::default_extensions()` struct update syntax.

- [ ] **Step 2: Add ClaudeDesktop variant to ToolId enum**

Add `ClaudeDesktop` variant, `as_key()` → `"claude_desktop"`, add adapter entry in `default_tool_adapters()` with macOS detection path `Library/Application Support/Claude` and MCP config path.

- [ ] **Step 3: Add MCP fields to Claude Code, Cursor, Claude Desktop adapters**

Claude Code: `mcp_config_path: Some(".claude/config/config.json")`, `supports_mcp: true`
Cursor: `mcp_config_path: Some(".cursor/mcp.json")`, `supports_mcp: true`
Claude Desktop: `mcp_config_path: Some("Library/Application Support/Claude/claude_desktop_config.json")`, `supports_mcp: true`

- [ ] **Step 4: Update tests, run `cargo test`**

Ensure existing adapter tests pass. Add test for `ClaudeDesktop` adapter lookup.

- [ ] **Step 5: Run `npm run check` and commit**

```bash
git add src-tauri/src/core/tool_adapters/mod.rs src-tauri/src/core/tests/tool_adapters.rs
git commit -m "feat: extend ToolAdapter with MCP/plugin fields, add Claude Desktop adapter"
```

---

### Task 1.2: Migrate Database Schema V3 → V4

**Files:**
- Modify: `src-tauri/src/core/skill_store.rs`
- Test: `src-tauri/src/core/tests/skill_store.rs`

- [ ] **Step 1: Bump SCHEMA_VERSION to 4, add V4 migration DDL**

In `ensure_schema()`, add `if user_version < 4` branch with the migration SQL from the spec:
- `ALTER TABLE skills RENAME TO assets` + add columns
- Recreate `skill_targets` as `asset_targets` (copy data, drop old)
- Recreate `discovered_skills` as `discovered_assets` (fix FK)
- Create `libraries`, `library_items`, `deploy_profiles` tables
- Add indexes

- [ ] **Step 2: Rename SkillRecord → AssetRecord**

Change struct name and make `central_path: Option<String>`. Add `asset_type: String`, `config_json: Option<String>`, `security_status: Option<String>` fields.

- [ ] **Step 3: Rename SkillTargetRecord → AssetTargetRecord**

Rename `skill_id` → `asset_id`, `mode` → `sync_mode`.

- [ ] **Step 4: Update all SQL queries to reference new table/column names**

- `upsert_skill()` → `upsert_asset()` — queries `assets` table
- `list_skills()` → `list_assets()` — optionally filter by `asset_type`
- `get_skill()` → `get_asset()`
- `delete_skill()` → `delete_asset()`
- `upsert_skill_target()` → `upsert_asset_target()` — uses `asset_id`, `sync_mode`
- `list_skill_targets()` → `list_asset_targets()`
- Update `discovered_skills` queries to `discovered_assets` with `imported_asset_id`

- [ ] **Step 5: Update `migrate_legacy_db_if_needed()` and `db_has_any_skills()`**

Rename to `db_has_any_assets()`. Check for both `skills` and `assets` in `sqlite_master`.

- [ ] **Step 6: Add type alias for backward compatibility**

```rust
pub type SkillRecord = AssetRecord;  // Temporary — remove after all callers updated
pub type SkillTargetRecord = AssetTargetRecord;
```

- [ ] **Step 7: Update tests for new schema**

Rewrite skill_store tests to use `AssetRecord` with `asset_type: "skill"`. Test V4 migration from a V3 database. Test that MCP/plugin assets can have `central_path: None`.

- [ ] **Step 8: Run `cargo test` and commit**

```bash
git commit -m "feat: migrate DB schema V3→V4, rename skills→assets"
```

---

### Task 1.3: Update Commands Layer

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands/tests/commands.rs`

- [ ] **Step 1: Rename DTOs**

- `ManagedSkillDto` → `ManagedAssetDto` (add `asset_type`, `config_json`, `security_status`)
- `SkillTargetDto` → `AssetTargetDto` (rename `mode` → `sync_mode`)
- Keep `InstallResultDto`, `UpdateResultDto`, etc. (they're operation-specific, not type-specific)

- [ ] **Step 2: Update `get_managed_skills` → `get_managed_assets`**

Query `list_assets()` instead of `list_skills()`. Optionally accept `asset_type` filter param.

- [ ] **Step 3: Update all commands referencing SkillStore methods**

Every command that calls `store.upsert_skill()`, `store.list_skills()`, etc. must call the renamed methods. Commands that dereference `central_path` must handle `None` (skip for MCP/plugin assets).

- [ ] **Step 4: Update `set_central_repo_path` to skip non-skill assets**

Add guard: `if asset.central_path.is_none() { continue; }` in the skill migration loop.

- [ ] **Step 5: Update `backfill_skill_descriptions()` in `lib.rs`**

Filter by `WHERE asset_type = 'skill'` when querying for backfill.

- [ ] **Step 6: Register new command names in `generate_handler!`**

Add `get_managed_assets` (keep `get_managed_skills` as alias initially for frontend compatibility).

- [ ] **Step 7: Update command tests, run full check**

```bash
npm run check
git commit -m "feat: rename commands layer DTOs from Skill→Asset"
```

---

### Task 1.4: Update Frontend Types and Components

**Files:**
- Modify: `src/components/skills/types.ts`
- Modify: `src/App.tsx`
- Modify: `src/components/skills/SkillCard.tsx` → AssetCard
- Modify: `src/components/skills/SkillsList.tsx` → AssetList
- Modify: `src/components/skills/Header.tsx`
- Modify: `src/components/skills/FilterBar.tsx`
- Modify: `src/i18n/resources.ts`

- [ ] **Step 1: Update types.ts**

Rename `ManagedSkill` → `ManagedAsset`. Add `asset_type`, `config_json`, `security_status` fields. Keep `ManagedSkill` as type alias temporarily.

- [ ] **Step 2: Update App.tsx state and invoke calls**

- Change `invoke('get_managed_skills')` → `invoke('get_managed_assets')`
- Update state variable names where practical (can be incremental)
- Add `activeView` values: `'myskills'` becomes `'ecosystem'` (keep both working during transition)

- [ ] **Step 3: Update Header navigation**

Rename "My Skills" tab to "My Ecosystem" in both Header component and i18n. Keep the same nav structure for now (Ecosystem | Explore | Settings).

- [ ] **Step 4: Add type filter chips to FilterBar**

Add filter state for `asset_type`: `All` | `Skills` | `MCP Servers` | `Plugins` | `Executables`. Default to `All`. Pass filter to asset list.

- [ ] **Step 5: Generalize SkillCard → AssetCard**

Add type badge (color-coded by `asset_type`). Conditionally show/hide fields based on type (e.g., hide "Update from source" for plugins). Keep existing skill card behavior identical.

- [ ] **Step 6: Add i18n keys for new concepts**

Add EN and ZH translations for: `navEcosystem`, `filterAll`, `filterSkills`, `filterMcpServers`, `filterPlugins`, `filterExecutables`, `assetTypeSkill`, `assetTypeMcpServer`, `assetTypePlugin`, `assetTypeExecutable`, `securityTrusted`, `securityUnknown`, `securityFlagged`.

- [ ] **Step 7: Run `npm run check` and commit**

```bash
npm run check
git commit -m "feat: rename frontend from Skills→Assets, add type filters"
```

---

### Task 1.5: Extend SyncMode Enum

**Files:**
- Modify: `src-tauri/src/core/sync_engine.rs`
- Modify: `src-tauri/src/commands/mod.rs` (match blocks)

- [ ] **Step 1: Add JsonMerge and CliCommand variants to SyncMode**

```rust
pub enum SyncMode {
    Auto, Symlink, Junction, Copy, JsonMerge, CliCommand,
}
```

- [ ] **Step 2: Update all match blocks to handle new variants**

In `commands/mod.rs`, any match on `SyncMode` or `sync_mode` string needs arms for `"json_merge"` and `"cli_command"`. For now, these can return errors ("not yet implemented") since Phase 2/3 add the actual logic.

- [ ] **Step 3: Run `npm run check` and commit**

```bash
npm run check
git commit -m "feat: add JsonMerge and CliCommand sync modes (stubs)"
```

---

### Task 1.6: End-to-End Verification

- [ ] **Step 1: Run full test suite**

```bash
npm run check
```
All 69+ tests must pass. Zero clippy warnings.

- [ ] **Step 2: Run `npm run tauri:dev` and smoke test**

- Install a skill from local path → verify it appears as `asset_type: skill`
- Install a skill from git → verify it works
- Sync a skill to a tool → verify symlink/copy works
- Update a skill → verify update works
- Delete a skill → verify cascade to targets
- Check Settings page → verify all settings work

- [ ] **Step 3: Commit and push Phase 1**

```bash
git push origin main
```

---

## Phase 2: MCP Server Management

**Prerequisite:** Phase 1 complete.

### Task 2.1: Create `mcp_manager.rs` Core Module

**New file:** `src-tauri/src/core/mcp_manager.rs`

- Define `McpServerConfig` struct (command, args, env)
- `read_tool_mcp_config(tool_adapter) → HashMap<String, McpServerConfig>` — reads a tool's config file, parses mcpServers section
- `write_tool_mcp_config(tool_adapter, servers) → Result<()>` — writes back with backup/restore safety
- `scan_all_mcp_servers() → Vec<(ToolId, String, McpServerConfig)>` — scans all MCP-capable tools
- `detect_mcp_conflicts(scanned) → Vec<McpConflict>` — same name, different config across tools

**Test file:** `src-tauri/src/core/tests/mcp_manager.rs` — test read/write/backup with tempfiles

### Task 2.2: MCP Tauri Commands

**Modify:** `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

New commands:
- `get_mcp_servers` — list all MCP assets from DB
- `add_mcp_server(name, command, args, env)` — create asset record + sync to profile targets
- `sync_mcp_to_tool(asset_id, tool_key)` — write to tool's config file
- `unsync_mcp_from_tool(asset_id, tool_key)` — remove from tool's config file
- `import_mcp_servers` — scan all tools, create assets, return conflicts
- `delete_mcp_server(asset_id)` — remove from DB + all tool configs

### Task 2.3: MCP Frontend Components

**New file:** `src/components/skills/modals/McpConfigModal.tsx` — form for name/command/args/env
**New file:** `src/components/skills/modals/ImportConflictModal.tsx` — side-by-side conflict view
**Modify:** `src/App.tsx` — MCP state, handlers, modal states
**Modify:** `src/i18n/resources.ts` — MCP translation keys (EN + ZH)

### Task 2.4: MCP Onboarding/Import Flow

Import existing MCP configs from Claude Code, Cursor, Claude Desktop. Show conflicts. User resolves. Store in assets table.

---

## Phase 3: Plugin Management

**Prerequisite:** Phase 1 complete (Phase 2 not required).

### Task 3.1: Create `plugin_manager.rs` Core Module

**New file:** `src-tauri/src/core/plugin_manager.rs`

- `read_installed_plugins() → Vec<PluginInfo>` — parse `~/.claude/plugins/installed_plugins.json`
- `read_plugin_health(plugin) → PluginHealthReport` — validate PLUGIN.md, check referenced files, detect common errors
- `diagnose_all_plugins() → Vec<PluginDiagnostic>` — batch health check
- `install_plugin_via_cli(url) → Result<()>` — shell out to `claude` CLI
- `remove_plugin_via_cli(name) → Result<()>`

### Task 3.2: Plugin Tauri Commands

- `get_plugins` — list all plugin assets + health status
- `install_plugin(url)` — CLI wrapper + create asset record
- `remove_plugin(asset_id)` — CLI wrapper + delete asset
- `diagnose_plugins` — return health report for all plugins
- `import_plugins` — scan installed_plugins.json, create asset records

### Task 3.3: Plugin Frontend Components

Plugin cards in My Ecosystem with health indicators (green/yellow/red). Diagnostics modal showing specific errors and suggested fixes.

---

## Phase 4: Library Registry + Enhanced Explore

**Prerequisite:** Phase 1 complete.

### Task 4.1: Enable FTS5 in Cargo.toml

Change rusqlite feature from `"bundled"` to `"bundled-full"` (or add FTS5 flag).

### Task 4.2: Create `library_manager.rs` Core Module

**New file:** `src-tauri/src/core/library_manager.rs`

- `add_library(url, name, type) → LibraryRecord`
- `index_library(library_id) → Vec<LibraryItem>` — clone/fetch + walk + detect assets
- `search_library_items(query, filters) → Vec<LibraryItem>` — FTS5 search
- `seed_default_libraries()` — import from known_marketplaces.json + featured-skills.json
- Library CRUD (add, remove, refresh)

### Task 4.3: Library Tauri Commands

- `add_library`, `remove_library`, `index_library`, `get_libraries`
- `search_library_items(query, asset_type_filter, library_id_filter)`
- `get_library_items(library_id)`

### Task 4.4: Enhanced Explore Page

Library selector dropdown, unified search bar with FTS5, type filter chips, install flow triggered from Explore cards.

### Task 4.5: Library Management Modal

Add/remove/refresh library URLs from the Explore page.

---

## Phase 5: Security Gate

**Prerequisite:** Phase 4 (Explore install flow).

### Task 5.1: Create `security_gate.rs` Core Module

**New file:** `src-tauri/src/core/security_gate.rs`

- `tier1_check(asset_path, source_library) → SecurityResult` — file type validation, metadata check, blocklist, trust tier
- `tier2_deep_scan(asset_path) → Vec<SecurityFinding>` — Ghost integration or fallback scanner
- `get_trust_tier(library) → TrustTier` — trusted publisher / known / unknown / flagged

### Task 5.2: Trust Tier Management

Settings commands for trusted publishers: `get_trusted_publishers`, `add_trusted_publisher`, `remove_trusted_publisher`. Pre-seed with Anthropic marketplaces.

### Task 5.3: Security UI Integration

Shield icons on asset cards and Explore cards. Security scan results modal. Warning prompts before installing from unknown sources.

---

## Phase 6: Deploy Profiles + Polish

**Prerequisite:** Phases 1-5 for full functionality.

### Task 6.1: Deploy Profile CRUD

Commands: `get_deploy_profiles`, `create_deploy_profile`, `update_deploy_profile`, `delete_deploy_profile`, `set_default_profile`.

### Task 6.2: Deploy Profile Settings UI

Settings page section with tool checkboxes per asset type. Default profile toggle.

### Task 6.3: Auto-Deploy on Install

When installing any asset, resolve the default deploy profile and auto-sync to the specified tools.

### Task 6.4: Unified Onboarding Flow

Extend existing onboarding to scan skills + MCP servers + plugins in one pass. Present unified import plan.

### Task 6.5: i18n Completion

Ensure all new text across all phases has both EN and ZH translations. Run through every new UI element.

### Task 6.6: Final Polish

- Remove temporary type aliases (`SkillRecord = AssetRecord`)
- Remove backward-compatible command aliases
- Update CLAUDE.md with new architecture description
- Final `npm run check` across everything

---

## Session Handoff Notes

Each phase is independently shippable. To pick up a phase in a fresh session:

1. Read the spec: `docs/superpowers/specs/2026-03-30-skills-hub-v2-ecosystem-manager-design.md`
2. Read this plan, focus on the relevant phase
3. Read `CLAUDE.md` for project conventions
4. Run `npm run check` to verify clean starting state
5. Execute the phase tasks in order
6. Run `npm run check` + `npm run tauri:dev` smoke test after each task
7. Commit after each task, push after phase completion
