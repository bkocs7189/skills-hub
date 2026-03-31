use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolId {
    Cursor,
    ClaudeCode,
    ClaudeDesktop,
    Codex,
    OpenCode,
    Antigravity,
    Amp,
    KimiCli,
    Augment,
    OpenClaw,
    Cline,
    CodeBuddy,
    CommandCode,
    Continue,
    Crush,
    Junie,
    IflowCli,
    KiroCli,
    Kode,
    McpJam,
    MistralVibe,
    Mux,
    OpenClaude,
    OpenHands,
    Pi,
    Qoder,
    QoderWork,
    QwenCode,
    Trae,
    TraeCn,
    Zencoder,
    Neovate,
    Pochi,
    AdaL,
    KiloCode,
    RooCode,
    Goose,
    GeminiCli,
    GithubCopilot,
    Clawdbot,
    Droid,
    Windsurf,
    Moltbot,
}

impl ToolId {
    pub fn as_key(&self) -> &'static str {
        match self {
            ToolId::Cursor => "cursor",
            ToolId::ClaudeCode => "claude_code",
            ToolId::ClaudeDesktop => "claude_desktop",
            ToolId::Codex => "codex",
            ToolId::OpenCode => "opencode",
            ToolId::Antigravity => "antigravity",
            ToolId::Amp => "amp",
            ToolId::KimiCli => "kimi_cli",
            ToolId::Augment => "augment",
            ToolId::OpenClaw => "openclaw",
            ToolId::Cline => "cline",
            ToolId::CodeBuddy => "codebuddy",
            ToolId::CommandCode => "command_code",
            ToolId::Continue => "continue",
            ToolId::Crush => "crush",
            ToolId::Junie => "junie",
            ToolId::IflowCli => "iflow_cli",
            ToolId::KiroCli => "kiro_cli",
            ToolId::Kode => "kode",
            ToolId::McpJam => "mcpjam",
            ToolId::MistralVibe => "mistral_vibe",
            ToolId::Mux => "mux",
            ToolId::OpenClaude => "openclaude",
            ToolId::OpenHands => "openhands",
            ToolId::Pi => "pi",
            ToolId::Qoder => "qoder",
            ToolId::QoderWork => "qoderwork",
            ToolId::QwenCode => "qwen_code",
            ToolId::Trae => "trae",
            ToolId::TraeCn => "trae_cn",
            ToolId::Zencoder => "zencoder",
            ToolId::Neovate => "neovate",
            ToolId::Pochi => "pochi",
            ToolId::AdaL => "adal",
            ToolId::KiloCode => "kilo_code",
            ToolId::RooCode => "roo_code",
            ToolId::Goose => "goose",
            ToolId::GeminiCli => "gemini_cli",
            ToolId::GithubCopilot => "github_copilot",
            ToolId::Clawdbot => "clawdbot",
            ToolId::Droid => "droid",
            ToolId::Windsurf => "windsurf",
            ToolId::Moltbot => "moltbot",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToolAdapter {
    pub id: ToolId,
    pub display_name: &'static str,
    /// Global skill directory under user home (aligned with add-skill docs).
    pub relative_skills_dir: &'static str,
    /// Directory used to detect whether the tool is installed (aligned with add-skill docs).
    pub relative_detect_dir: &'static str,
    /// Whether this tool supports MCP server configuration.
    #[allow(dead_code)]
    pub supports_mcp: bool,
    /// Relative path (from home) to MCP config file, e.g. ".cursor/mcp.json".
    #[allow(dead_code)]
    pub mcp_config_path: Option<&'static str>,
    /// JSON key that holds MCP servers in the config file, e.g. "mcpServers".
    #[allow(dead_code)]
    pub mcp_config_key: Option<&'static str>,
    /// Whether this tool supports plugins.
    #[allow(dead_code)]
    pub supports_plugins: bool,
    /// CLI binary name for plugin operations, e.g. "claude".
    #[allow(dead_code)]
    pub plugin_cli: Option<&'static str>,
}

impl ToolAdapter {
    /// Create a skill-only adapter (no MCP/plugin support). Used by the majority of tools.
    pub const fn skill_only(
        id: ToolId,
        display_name: &'static str,
        relative_skills_dir: &'static str,
        relative_detect_dir: &'static str,
    ) -> Self {
        Self {
            id,
            display_name,
            relative_skills_dir,
            relative_detect_dir,
            supports_mcp: false,
            mcp_config_path: None,
            mcp_config_key: None,
            supports_plugins: false,
            plugin_cli: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DetectedSkill {
    pub tool: ToolId,
    pub name: String,
    pub path: PathBuf,
    pub is_link: bool,
    pub link_target: Option<PathBuf>,
}

pub fn default_tool_adapters() -> Vec<ToolAdapter> {
    vec![
        // Tools with MCP and/or plugin support
        ToolAdapter {
            id: ToolId::Cursor,
            display_name: "Cursor",
            relative_skills_dir: ".cursor/skills",
            relative_detect_dir: ".cursor",
            supports_mcp: true,
            mcp_config_path: Some(".cursor/mcp.json"),
            mcp_config_key: Some("mcpServers"),
            supports_plugins: false,
            plugin_cli: None,
        },
        ToolAdapter {
            id: ToolId::ClaudeCode,
            display_name: "Claude Code",
            relative_skills_dir: ".claude/skills",
            relative_detect_dir: ".claude",
            supports_mcp: true,
            mcp_config_path: Some(".claude/config/config.json"),
            mcp_config_key: Some("mcpServers"),
            supports_plugins: true,
            plugin_cli: Some("claude"),
        },
        ToolAdapter {
            id: ToolId::ClaudeDesktop,
            display_name: "Claude Desktop",
            relative_skills_dir: "Library/Application Support/Claude/skills",
            relative_detect_dir: "Library/Application Support/Claude",
            supports_mcp: true,
            mcp_config_path: Some("Library/Application Support/Claude/claude_desktop_config.json"),
            mcp_config_key: Some("mcpServers"),
            supports_plugins: false,
            plugin_cli: None,
        },
        // Skill-only tools
        ToolAdapter::skill_only(ToolId::Codex, "Codex", ".codex/skills", ".codex"),
        ToolAdapter::skill_only(
            ToolId::OpenCode,
            "OpenCode",
            ".config/opencode/skills",
            ".config/opencode",
        ),
        ToolAdapter::skill_only(
            ToolId::Antigravity,
            "Antigravity",
            ".gemini/antigravity/global_skills",
            ".gemini/antigravity",
        ),
        ToolAdapter::skill_only(
            ToolId::Amp,
            "Amp",
            ".config/agents/skills",
            ".config/agents",
        ),
        ToolAdapter::skill_only(
            ToolId::KimiCli,
            "Kimi Code CLI",
            ".config/agents/skills",
            ".config/agents",
        ),
        ToolAdapter::skill_only(ToolId::Augment, "Augment", ".augment/rules", ".augment"),
        ToolAdapter::skill_only(
            ToolId::OpenClaw,
            "OpenClaw",
            ".openclaw/skills",
            ".openclaw",
        ),
        ToolAdapter::skill_only(ToolId::Cline, "Cline", ".cline/skills", ".cline"),
        ToolAdapter::skill_only(
            ToolId::CodeBuddy,
            "CodeBuddy",
            ".codebuddy/skills",
            ".codebuddy",
        ),
        ToolAdapter::skill_only(
            ToolId::CommandCode,
            "Command Code",
            ".commandcode/skills",
            ".commandcode",
        ),
        ToolAdapter::skill_only(
            ToolId::Continue,
            "Continue",
            ".continue/skills",
            ".continue",
        ),
        ToolAdapter::skill_only(
            ToolId::Crush,
            "Crush",
            ".config/crush/skills",
            ".config/crush",
        ),
        ToolAdapter::skill_only(ToolId::Junie, "Junie", ".junie/skills", ".junie"),
        ToolAdapter::skill_only(ToolId::IflowCli, "iFlow CLI", ".iflow/skills", ".iflow"),
        ToolAdapter::skill_only(ToolId::KiroCli, "Kiro CLI", ".kiro/skills", ".kiro"),
        ToolAdapter::skill_only(ToolId::Kode, "Kode", ".kode/skills", ".kode"),
        ToolAdapter::skill_only(ToolId::McpJam, "MCPJam", ".mcpjam/skills", ".mcpjam"),
        ToolAdapter::skill_only(ToolId::MistralVibe, "Mistral Vibe", ".vibe/skills", ".vibe"),
        ToolAdapter::skill_only(ToolId::Mux, "Mux", ".mux/skills", ".mux"),
        ToolAdapter::skill_only(
            ToolId::OpenClaude,
            "OpenClaude IDE",
            ".openclaude/skills",
            ".openclaude",
        ),
        ToolAdapter::skill_only(
            ToolId::OpenHands,
            "OpenHands",
            ".openhands/skills",
            ".openhands",
        ),
        ToolAdapter::skill_only(ToolId::Pi, "Pi", ".pi/agent/skills", ".pi"),
        ToolAdapter::skill_only(ToolId::Qoder, "Qoder", ".qoder/skills", ".qoder"),
        ToolAdapter::skill_only(
            ToolId::QoderWork,
            "QoderWork",
            ".qoderwork/skills",
            ".qoderwork",
        ),
        ToolAdapter::skill_only(ToolId::QwenCode, "Qwen Code", ".qwen/skills", ".qwen"),
        ToolAdapter::skill_only(ToolId::Trae, "Trae", ".trae/skills", ".trae"),
        ToolAdapter::skill_only(ToolId::TraeCn, "Trae CN", ".trae-cn/skills", ".trae-cn"),
        ToolAdapter::skill_only(
            ToolId::Zencoder,
            "Zencoder",
            ".zencoder/skills",
            ".zencoder",
        ),
        ToolAdapter::skill_only(ToolId::Neovate, "Neovate", ".neovate/skills", ".neovate"),
        ToolAdapter::skill_only(ToolId::Pochi, "Pochi", ".pochi/skills", ".pochi"),
        ToolAdapter::skill_only(ToolId::AdaL, "AdaL", ".adal/skills", ".adal"),
        ToolAdapter::skill_only(
            ToolId::KiloCode,
            "Kilo Code",
            ".kilocode/skills",
            ".kilocode",
        ),
        ToolAdapter::skill_only(ToolId::RooCode, "Roo Code", ".roo/skills", ".roo"),
        ToolAdapter::skill_only(
            ToolId::Goose,
            "Goose",
            ".config/goose/skills",
            ".config/goose",
        ),
        ToolAdapter::skill_only(ToolId::GeminiCli, "Gemini CLI", ".gemini/skills", ".gemini"),
        ToolAdapter::skill_only(
            ToolId::GithubCopilot,
            "GitHub Copilot",
            ".copilot/skills",
            ".copilot",
        ),
        ToolAdapter::skill_only(
            ToolId::Clawdbot,
            "Clawdbot",
            ".clawdbot/skills",
            ".clawdbot",
        ),
        ToolAdapter::skill_only(ToolId::Droid, "Droid", ".factory/skills", ".factory"),
        ToolAdapter::skill_only(
            ToolId::Windsurf,
            "Windsurf",
            ".codeium/windsurf/skills",
            ".codeium/windsurf",
        ),
        ToolAdapter::skill_only(ToolId::Moltbot, "MoltBot", ".moltbot/skills", ".moltbot"),
    ]
}

/// Resolve the MCP config file path for a tool.
#[allow(dead_code)]
pub fn resolve_mcp_config_path(adapter: &ToolAdapter) -> Result<Option<PathBuf>> {
    match adapter.mcp_config_path {
        Some(rel) => {
            let home = dirs::home_dir().context("failed to resolve home directory")?;
            Ok(Some(home.join(rel)))
        }
        None => Ok(None),
    }
}

/// Tools can share the same global skills directory (e.g. Amp and Kimi Code CLI).
/// Use this to coordinate UI warnings and avoid duplicate filesystem operations.
pub fn adapters_sharing_skills_dir(adapter: &ToolAdapter) -> Vec<ToolAdapter> {
    default_tool_adapters()
        .into_iter()
        .filter(|a| a.relative_skills_dir == adapter.relative_skills_dir)
        .collect()
}

pub fn adapter_by_key(key: &str) -> Option<ToolAdapter> {
    default_tool_adapters()
        .into_iter()
        .find(|adapter| adapter.id.as_key() == key)
}

pub fn resolve_default_path(adapter: &ToolAdapter) -> Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory")?;
    Ok(home.join(adapter.relative_skills_dir))
}

pub fn resolve_detect_path(adapter: &ToolAdapter) -> Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory")?;
    Ok(home.join(adapter.relative_detect_dir))
}

pub fn is_tool_installed(adapter: &ToolAdapter) -> Result<bool> {
    Ok(resolve_detect_path(adapter)?.exists())
}

pub fn scan_tool_dir(tool: &ToolAdapter, dir: &Path) -> Result<Vec<DetectedSkill>> {
    let mut results = Vec::new();
    if !dir.exists() {
        return Ok(results);
    }

    let ignore_hint = "Application Support/com.tauri.dev/skills";

    for entry in std::fs::read_dir(dir).with_context(|| format!("read dir {:?}", dir))? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        let is_dir = file_type.is_dir() || (file_type.is_symlink() && path.is_dir());
        if !is_dir {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if tool.id == ToolId::Codex && name == ".system" {
            continue;
        }
        let (is_link, link_target) = detect_link(&path);
        if path.to_string_lossy().contains(ignore_hint)
            || link_target
                .as_ref()
                .map(|p| p.to_string_lossy().contains(ignore_hint))
                .unwrap_or(false)
        {
            continue;
        }
        results.push(DetectedSkill {
            tool: tool.id.clone(),
            name,
            path,
            is_link,
            link_target,
        });
    }

    Ok(results)
}

fn detect_link(path: &Path) -> (bool, Option<PathBuf>) {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let target = std::fs::read_link(path).ok();
            (true, target)
        }
        _ => {
            let target = std::fs::read_link(path).ok();
            if target.is_some() {
                (true, target)
            } else {
                (false, None)
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/tool_adapters.rs"]
mod tests;
