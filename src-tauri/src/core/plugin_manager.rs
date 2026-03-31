use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Information about a single installed plugin.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PluginInfo {
    pub name: String,
    pub marketplace: String,
    pub version: String,
    pub install_path: String,
    pub installed_at: String,
    pub scope: String,
    pub git_commit_sha: Option<String>,
}

/// Health check result for a plugin.
#[derive(Serialize, Clone, Debug)]
pub struct PluginHealthReport {
    pub name: String,
    pub healthy: bool,
    pub issues: Vec<String>,
}

/// Information about a marketplace.
#[derive(Serialize, Clone, Debug)]
pub struct MarketplaceInfo {
    pub name: String,
    pub plugin_count: usize,
    pub path: String,
}

/// Internal JSON structure for installed_plugins.json version 2.
#[derive(Deserialize)]
struct InstalledPluginsFile {
    #[allow(dead_code)]
    version: Option<u32>,
    plugins: HashMap<String, Vec<PluginEntry>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginEntry {
    scope: Option<String>,
    install_path: Option<String>,
    version: Option<String>,
    installed_at: Option<String>,
    #[serde(default)]
    git_commit_sha: Option<String>,
}

fn plugins_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("resolve home directory")?;
    Ok(home.join(".claude").join("plugins"))
}

/// Read all installed plugins from `~/.claude/plugins/installed_plugins.json`.
///
/// Returns an empty list if the file does not exist.
pub fn read_installed_plugins() -> Result<Vec<PluginInfo>> {
    read_installed_plugins_from(&plugins_dir()?)
}

/// Testable version that accepts a custom plugins directory.
pub fn read_installed_plugins_from(plugins_dir: &Path) -> Result<Vec<PluginInfo>> {
    let path = plugins_dir.join("installed_plugins.json");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path).with_context(|| format!("read {:?}", path))?;

    let file: InstalledPluginsFile =
        serde_json::from_str(&content).with_context(|| format!("parse {:?}", path))?;

    let mut result = Vec::new();
    for (key, entries) in &file.plugins {
        // key format: "name@marketplace"
        let (name, marketplace) = match key.rsplit_once('@') {
            Some((n, m)) => (n.to_string(), m.to_string()),
            None => (key.clone(), "unknown".to_string()),
        };

        for entry in entries {
            result.push(PluginInfo {
                name: name.clone(),
                marketplace: marketplace.clone(),
                version: entry.version.clone().unwrap_or_default(),
                install_path: entry.install_path.clone().unwrap_or_default(),
                installed_at: entry.installed_at.clone().unwrap_or_default(),
                scope: entry.scope.clone().unwrap_or_else(|| "user".to_string()),
                git_commit_sha: entry.git_commit_sha.clone(),
            });
        }
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// Check the health of a single plugin.
pub fn check_plugin_health(plugin: &PluginInfo) -> PluginHealthReport {
    let mut issues = Vec::new();

    let install_path = Path::new(&plugin.install_path);

    if !install_path.exists() {
        issues.push("install_path_missing".to_string());
        return PluginHealthReport {
            name: plugin.name.clone(),
            healthy: false,
            issues,
        };
    }

    if !install_path.join("PLUGIN.md").exists() {
        issues.push("missing_plugin_md".to_string());
    }

    let has_skills = install_path.join("skills").is_dir();
    let has_agents = install_path.join("agents").is_dir();
    if !has_skills && !has_agents {
        issues.push("no_skills_or_agents_dir".to_string());
    }

    // Check for empty directories.
    if has_skills {
        if let Ok(mut entries) = std::fs::read_dir(install_path.join("skills")) {
            if entries.next().is_none() {
                issues.push("empty_skills_dir".to_string());
            }
        }
    }
    if has_agents {
        if let Ok(mut entries) = std::fs::read_dir(install_path.join("agents")) {
            if entries.next().is_none() {
                issues.push("empty_agents_dir".to_string());
            }
        }
    }

    PluginHealthReport {
        name: plugin.name.clone(),
        healthy: issues.is_empty(),
        issues,
    }
}

/// Run health checks on all installed plugins.
pub fn diagnose_all_plugins() -> Result<Vec<PluginHealthReport>> {
    diagnose_all_plugins_from(&plugins_dir()?)
}

/// Testable version that accepts a custom plugins directory.
pub fn diagnose_all_plugins_from(plugins_dir: &Path) -> Result<Vec<PluginHealthReport>> {
    let plugins = read_installed_plugins_from(plugins_dir)?;
    Ok(plugins.iter().map(check_plugin_health).collect())
}

/// Read marketplace info from `~/.claude/plugins/known_marketplaces.json`
/// and scan `~/.claude/plugins/marketplaces/`.
pub fn get_marketplace_info() -> Result<Vec<MarketplaceInfo>> {
    get_marketplace_info_from(&plugins_dir()?)
}

/// Internal JSON structure for known_marketplaces.json.
#[derive(Deserialize)]
struct KnownMarketplacesFile {
    marketplaces: Option<Vec<KnownMarketplace>>,
}

#[derive(Deserialize)]
struct KnownMarketplace {
    name: Option<String>,
}

/// Testable version that accepts a custom plugins directory.
pub fn get_marketplace_info_from(plugins_dir: &Path) -> Result<Vec<MarketplaceInfo>> {
    let mut result = Vec::new();

    // Read known_marketplaces.json for names.
    let known_path = plugins_dir.join("known_marketplaces.json");
    let mut known_names: Vec<String> = Vec::new();
    if known_path.exists() {
        let content = std::fs::read_to_string(&known_path)
            .with_context(|| format!("read {:?}", known_path))?;
        if let Ok(file) = serde_json::from_str::<KnownMarketplacesFile>(&content) {
            if let Some(marketplaces) = file.marketplaces {
                for m in marketplaces {
                    if let Some(name) = m.name {
                        known_names.push(name);
                    }
                }
            }
        }
    }

    // Scan marketplaces directory.
    let marketplaces_dir = plugins_dir.join("marketplaces");
    if marketplaces_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&marketplaces_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let item_count = std::fs::read_dir(&path).map(|rd| rd.count()).unwrap_or(0);
                    result.push(MarketplaceInfo {
                        name,
                        plugin_count: item_count,
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }

    // Add known marketplaces that weren't found in the directory.
    let existing_names: std::collections::HashSet<String> =
        result.iter().map(|m| m.name.clone()).collect();
    for name in known_names {
        if !existing_names.contains(&name) {
            result.push(MarketplaceInfo {
                name,
                plugin_count: 0,
                path: String::new(),
            });
        }
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

#[cfg(test)]
#[path = "tests/plugin_manager.rs"]
mod tests;
