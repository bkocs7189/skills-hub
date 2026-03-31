use crate::core::plugin_manager::{
    check_plugin_health, diagnose_all_plugins_from, get_marketplace_info_from,
    read_installed_plugins_from, PluginInfo,
};

#[test]
fn parse_installed_plugins_json() {
    let dir = tempfile::tempdir().unwrap();
    let content = r#"{
        "version": 2,
        "plugins": {
            "superpowers@claude-plugins-official": [{
                "scope": "user",
                "installPath": "/tmp/test-plugins/superpowers",
                "version": "1.0.0",
                "installedAt": "2025-01-15T10:00:00Z",
                "gitCommitSha": "abc123"
            }],
            "my-tool@custom-market": [{
                "scope": "project",
                "installPath": "/tmp/test-plugins/my-tool",
                "version": "2.1.0",
                "installedAt": "2025-02-20T12:00:00Z"
            }]
        }
    }"#;
    std::fs::write(dir.path().join("installed_plugins.json"), content).unwrap();

    let plugins = read_installed_plugins_from(dir.path()).unwrap();
    assert_eq!(plugins.len(), 2);

    let sp = plugins.iter().find(|p| p.name == "superpowers").unwrap();
    assert_eq!(sp.marketplace, "claude-plugins-official");
    assert_eq!(sp.version, "1.0.0");
    assert_eq!(sp.scope, "user");
    assert_eq!(sp.git_commit_sha.as_deref(), Some("abc123"));

    let mt = plugins.iter().find(|p| p.name == "my-tool").unwrap();
    assert_eq!(mt.marketplace, "custom-market");
    assert_eq!(mt.scope, "project");
    assert!(mt.git_commit_sha.is_none());
}

#[test]
fn missing_file_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let plugins = read_installed_plugins_from(dir.path()).unwrap();
    assert!(plugins.is_empty());
}

#[test]
fn health_check_detects_missing_install_path() {
    let plugin = PluginInfo {
        name: "test-plugin".to_string(),
        marketplace: "test".to_string(),
        version: "1.0.0".to_string(),
        install_path: "/nonexistent/path/that/does/not/exist".to_string(),
        installed_at: String::new(),
        scope: "user".to_string(),
        git_commit_sha: None,
    };

    let report = check_plugin_health(&plugin);
    assert!(!report.healthy);
    assert!(report.issues.contains(&"install_path_missing".to_string()));
}

#[test]
fn health_check_passes_for_valid_plugin() {
    let dir = tempfile::tempdir().unwrap();
    let plugin_dir = dir.path().join("my-plugin");
    std::fs::create_dir_all(plugin_dir.join("skills")).unwrap();
    std::fs::write(plugin_dir.join("PLUGIN.md"), "# My Plugin").unwrap();
    std::fs::write(plugin_dir.join("skills").join("main.md"), "skill content").unwrap();

    let plugin = PluginInfo {
        name: "my-plugin".to_string(),
        marketplace: "test".to_string(),
        version: "1.0.0".to_string(),
        install_path: plugin_dir.to_string_lossy().to_string(),
        installed_at: String::new(),
        scope: "user".to_string(),
        git_commit_sha: None,
    };

    let report = check_plugin_health(&plugin);
    assert!(report.healthy);
    assert!(report.issues.is_empty());
}

#[test]
fn health_check_detects_missing_plugin_md() {
    let dir = tempfile::tempdir().unwrap();
    let plugin_dir = dir.path().join("no-md");
    std::fs::create_dir_all(plugin_dir.join("skills")).unwrap();
    std::fs::write(plugin_dir.join("skills").join("main.md"), "content").unwrap();

    let plugin = PluginInfo {
        name: "no-md".to_string(),
        marketplace: "test".to_string(),
        version: "1.0.0".to_string(),
        install_path: plugin_dir.to_string_lossy().to_string(),
        installed_at: String::new(),
        scope: "user".to_string(),
        git_commit_sha: None,
    };

    let report = check_plugin_health(&plugin);
    assert!(!report.healthy);
    assert!(report.issues.contains(&"missing_plugin_md".to_string()));
}

#[test]
fn diagnose_all_returns_reports() {
    let dir = tempfile::tempdir().unwrap();
    let plugin_dir = dir.path().join("plugins-install").join("good-plugin");
    std::fs::create_dir_all(plugin_dir.join("skills")).unwrap();
    std::fs::write(plugin_dir.join("PLUGIN.md"), "# Plugin").unwrap();
    std::fs::write(plugin_dir.join("skills").join("a.md"), "content").unwrap();

    let content = serde_json::json!({
        "version": 2,
        "plugins": {
            "good-plugin@test-market": [{
                "scope": "user",
                "installPath": plugin_dir.to_string_lossy().to_string(),
                "version": "1.0.0",
                "installedAt": "2025-01-01T00:00:00Z"
            }]
        }
    });
    std::fs::write(
        dir.path().join("installed_plugins.json"),
        serde_json::to_string(&content).unwrap(),
    )
    .unwrap();

    let reports = diagnose_all_plugins_from(dir.path()).unwrap();
    assert_eq!(reports.len(), 1);
    assert!(reports[0].healthy);
}

#[test]
fn marketplace_info_reads_directory() {
    let dir = tempfile::tempdir().unwrap();
    let mp_dir = dir.path().join("marketplaces");
    std::fs::create_dir_all(mp_dir.join("official")).unwrap();
    std::fs::create_dir_all(mp_dir.join("community")).unwrap();
    // Add some items to official.
    std::fs::write(mp_dir.join("official").join("plugin-a"), "").unwrap();
    std::fs::write(mp_dir.join("official").join("plugin-b"), "").unwrap();

    let infos = get_marketplace_info_from(dir.path()).unwrap();
    assert_eq!(infos.len(), 2);

    let official = infos.iter().find(|m| m.name == "official").unwrap();
    assert_eq!(official.plugin_count, 2);

    let community = infos.iter().find(|m| m.name == "community").unwrap();
    assert_eq!(community.plugin_count, 0);
}
