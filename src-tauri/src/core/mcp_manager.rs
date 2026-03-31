use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::tool_adapters::ToolAdapter;

/// Canonical MCP server configuration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// A scanned MCP server entry found in a tool's config file.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScannedMcpServer {
    pub tool_key: String,
    pub name: String,
    pub config: McpServerConfig,
}

/// A conflict: same server name with different configs across tools.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(dead_code)]
pub struct McpConflict {
    pub name: String,
    pub entries: Vec<ScannedMcpServer>,
}

/// Read MCP server configs from a tool's config file.
///
/// Returns an empty map if the file doesn't exist or the key is missing.
pub fn read_mcp_config(
    config_path: &Path,
    config_key: &str,
) -> Result<HashMap<String, McpServerConfig>> {
    if !config_path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("read MCP config {:?}", config_path))?;

    let root: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("parse JSON in {:?}", config_path))?;

    let servers_val = match root.get(config_key) {
        Some(v) => v,
        None => return Ok(HashMap::new()),
    };

    let servers: HashMap<String, McpServerConfig> = serde_json::from_value(servers_val.clone())
        .with_context(|| format!("parse {} section in {:?}", config_key, config_path))?;

    Ok(servers)
}

/// Write MCP server configs to a tool's config file.
///
/// Safety: creates a `.bak` backup first. On failure, restores the backup.
/// Merges into existing JSON (preserves other keys).
pub fn write_mcp_config(
    config_path: &Path,
    config_key: &str,
    servers: &HashMap<String, McpServerConfig>,
) -> Result<()> {
    // Read existing JSON or start with empty object.
    let mut root: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(config_path)
            .with_context(|| format!("read {:?} for write", config_path))?;
        serde_json::from_str(&content)
            .with_context(|| format!("parse {:?} for write", config_path))?
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };

    // Create backup if file exists.
    let backup_path = config_path.with_extension("json.bak");
    let had_backup = if config_path.exists() {
        std::fs::copy(config_path, &backup_path)
            .with_context(|| format!("create backup {:?}", backup_path))?;
        true
    } else {
        false
    };

    // Merge the servers section.
    let servers_val = serde_json::to_value(servers).context("serialize MCP servers")?;
    if let serde_json::Value::Object(ref mut map) = root {
        map.insert(config_key.to_string(), servers_val);
    } else {
        anyhow::bail!("config file root is not a JSON object: {:?}", config_path);
    }

    // Validate output is well-formed by round-tripping.
    let output = serde_json::to_string_pretty(&root).context("pretty-print MCP config")?;
    let _: serde_json::Value = serde_json::from_str(&output).context("validate written JSON")?;

    // Ensure parent directory exists.
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create parent dir {:?}", parent))?;
    }

    // Write the file.
    if let Err(write_err) = std::fs::write(config_path, &output) {
        // Restore backup on failure.
        if had_backup {
            let _ = std::fs::copy(&backup_path, config_path);
        }
        return Err(write_err).with_context(|| format!("write {:?}", config_path));
    }

    Ok(())
}

/// Insert or update a single MCP server in a tool's config file.
pub fn upsert_mcp_server(
    config_path: &Path,
    config_key: &str,
    name: &str,
    config: &McpServerConfig,
) -> Result<()> {
    let mut servers = read_mcp_config(config_path, config_key)?;
    servers.insert(name.to_string(), config.clone());
    write_mcp_config(config_path, config_key, &servers)
}

/// Remove a single MCP server from a tool's config file.
pub fn remove_mcp_server(config_path: &Path, config_key: &str, name: &str) -> Result<()> {
    let mut servers = read_mcp_config(config_path, config_key)?;
    servers.remove(name);
    write_mcp_config(config_path, config_key, &servers)
}

/// Resolve an adapter's MCP config path (relative to home).
fn resolve_mcp_config_path(adapter: &ToolAdapter) -> Result<Option<std::path::PathBuf>> {
    let rel = match adapter.mcp_config_path {
        Some(p) => p,
        None => return Ok(None),
    };
    let home = dirs::home_dir().context("failed to resolve home directory")?;
    Ok(Some(home.join(rel)))
}

/// Scan all MCP-capable tool adapters and return a flat list of servers.
pub fn scan_all_mcp_servers(adapters: &[ToolAdapter]) -> Result<Vec<ScannedMcpServer>> {
    let mut results = Vec::new();

    for adapter in adapters {
        if !adapter.supports_mcp {
            continue;
        }
        let config_path = match resolve_mcp_config_path(adapter)? {
            Some(p) => p,
            None => continue,
        };
        let config_key = match adapter.mcp_config_key {
            Some(k) => k,
            None => continue,
        };

        let servers = match read_mcp_config(&config_path, config_key) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("skip MCP scan for {}: {}", adapter.id.as_key(), e);
                continue;
            }
        };

        for (name, config) in servers {
            results.push(ScannedMcpServer {
                tool_key: adapter.id.as_key().to_string(),
                name,
                config,
            });
        }
    }

    Ok(results)
}

/// Detect conflicts: same server name but different configs across tools.
#[allow(dead_code)]
pub fn detect_mcp_conflicts(scanned: &[ScannedMcpServer]) -> Vec<McpConflict> {
    let mut by_name: HashMap<String, Vec<ScannedMcpServer>> = HashMap::new();
    for entry in scanned {
        by_name
            .entry(entry.name.clone())
            .or_default()
            .push(entry.clone());
    }

    let mut conflicts = Vec::new();
    for (name, entries) in by_name {
        if entries.len() < 2 {
            continue;
        }
        // Check if configs differ.
        let first = &entries[0].config;
        let has_diff = entries
            .iter()
            .skip(1)
            .any(|e| e.config.command != first.command || e.config.args != first.args);
        if has_diff {
            conflicts.push(McpConflict { name, entries });
        }
    }

    conflicts
}

#[cfg(test)]
#[path = "tests/mcp_manager.rs"]
mod tests;
