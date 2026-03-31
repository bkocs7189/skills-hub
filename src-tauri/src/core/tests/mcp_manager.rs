use std::collections::HashMap;

use crate::core::mcp_manager::{
    detect_mcp_conflicts, read_mcp_config, remove_mcp_server, upsert_mcp_server, write_mcp_config,
    McpServerConfig, ScannedMcpServer,
};

#[test]
fn read_nonexistent_file_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does_not_exist.json");
    let result = read_mcp_config(&path, "mcpServers").unwrap();
    assert!(result.is_empty());
}

#[test]
fn read_config_with_mcp_servers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let content = r#"{
        "mcpServers": {
            "test-server": {
                "command": "npx",
                "args": ["-y", "test-server"],
                "env": {"API_KEY": "abc123"}
            }
        }
    }"#;
    std::fs::write(&path, content).unwrap();

    let servers = read_mcp_config(&path, "mcpServers").unwrap();
    assert_eq!(servers.len(), 1);
    let server = servers.get("test-server").unwrap();
    assert_eq!(server.command, "npx");
    assert_eq!(server.args, vec!["-y", "test-server"]);
    assert_eq!(server.env.get("API_KEY").unwrap(), "abc123");
}

#[test]
fn read_config_missing_key_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, r#"{"otherKey": {}}"#).unwrap();

    let servers = read_mcp_config(&path, "mcpServers").unwrap();
    assert!(servers.is_empty());
}

#[test]
fn write_config_creates_backup_and_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let original = r#"{"existingKey": "value"}"#;
    std::fs::write(&path, original).unwrap();

    let mut servers = HashMap::new();
    servers.insert(
        "my-server".to_string(),
        McpServerConfig {
            command: "node".to_string(),
            args: vec!["server.js".to_string()],
            env: HashMap::new(),
        },
    );

    write_mcp_config(&path, "mcpServers", &servers).unwrap();

    // Backup should exist.
    let backup = dir.path().join("config.json.bak");
    assert!(backup.exists());

    // Read back and verify.
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(written["existingKey"], "value");
    assert!(written["mcpServers"]["my-server"].is_object());
    assert_eq!(written["mcpServers"]["my-server"]["command"], "node");
}

#[test]
fn write_config_creates_file_if_not_exists() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("subdir").join("config.json");

    let mut servers = HashMap::new();
    servers.insert(
        "new-server".to_string(),
        McpServerConfig {
            command: "python".to_string(),
            args: vec!["-m".to_string(), "server".to_string()],
            env: HashMap::new(),
        },
    );

    write_mcp_config(&path, "mcpServers", &servers).unwrap();
    assert!(path.exists());

    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(written["mcpServers"]["new-server"]["command"], "python");
}

#[test]
fn upsert_adds_and_updates_server() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, "{}").unwrap();

    let config1 = McpServerConfig {
        command: "npx".to_string(),
        args: vec!["server-a".to_string()],
        env: HashMap::new(),
    };
    upsert_mcp_server(&path, "mcpServers", "server-a", &config1).unwrap();

    let servers = read_mcp_config(&path, "mcpServers").unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(servers["server-a"].command, "npx");

    // Update existing.
    let config2 = McpServerConfig {
        command: "node".to_string(),
        args: vec!["new-server-a.js".to_string()],
        env: HashMap::new(),
    };
    upsert_mcp_server(&path, "mcpServers", "server-a", &config2).unwrap();

    let servers = read_mcp_config(&path, "mcpServers").unwrap();
    assert_eq!(servers.len(), 1);
    assert_eq!(servers["server-a"].command, "node");

    // Add another.
    upsert_mcp_server(&path, "mcpServers", "server-b", &config1).unwrap();
    let servers = read_mcp_config(&path, "mcpServers").unwrap();
    assert_eq!(servers.len(), 2);
}

#[test]
fn remove_deletes_server_entry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");

    let config = McpServerConfig {
        command: "npx".to_string(),
        args: vec!["server".to_string()],
        env: HashMap::new(),
    };
    upsert_mcp_server(&path, "mcpServers", "to-remove", &config).unwrap();
    upsert_mcp_server(&path, "mcpServers", "to-keep", &config).unwrap();

    let servers = read_mcp_config(&path, "mcpServers").unwrap();
    assert_eq!(servers.len(), 2);

    remove_mcp_server(&path, "mcpServers", "to-remove").unwrap();

    let servers = read_mcp_config(&path, "mcpServers").unwrap();
    assert_eq!(servers.len(), 1);
    assert!(servers.contains_key("to-keep"));
    assert!(!servers.contains_key("to-remove"));
}

#[test]
fn detect_conflicts_finds_differing_configs() {
    let scanned = vec![
        ScannedMcpServer {
            tool_key: "cursor".to_string(),
            name: "shared-server".to_string(),
            config: McpServerConfig {
                command: "npx".to_string(),
                args: vec!["v1".to_string()],
                env: HashMap::new(),
            },
        },
        ScannedMcpServer {
            tool_key: "claude_code".to_string(),
            name: "shared-server".to_string(),
            config: McpServerConfig {
                command: "npx".to_string(),
                args: vec!["v2".to_string()],
                env: HashMap::new(),
            },
        },
        ScannedMcpServer {
            tool_key: "cursor".to_string(),
            name: "unique-server".to_string(),
            config: McpServerConfig {
                command: "node".to_string(),
                args: vec![],
                env: HashMap::new(),
            },
        },
    ];

    let conflicts = detect_mcp_conflicts(&scanned);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].name, "shared-server");
    assert_eq!(conflicts[0].entries.len(), 2);
}

#[test]
fn detect_conflicts_ignores_matching_configs() {
    let config = McpServerConfig {
        command: "npx".to_string(),
        args: vec!["same".to_string()],
        env: HashMap::new(),
    };
    let scanned = vec![
        ScannedMcpServer {
            tool_key: "cursor".to_string(),
            name: "same-server".to_string(),
            config: config.clone(),
        },
        ScannedMcpServer {
            tool_key: "claude_code".to_string(),
            name: "same-server".to_string(),
            config,
        },
    ];

    let conflicts = detect_mcp_conflicts(&scanned);
    assert!(conflicts.is_empty());
}
