use std::fs;

use crate::core::security_gate::{
    determine_trust_tier, get_default_trusted_publishers, tier1_check, tier2_deep_scan, TrustTier,
};

#[test]
fn trusted_publisher_bypasses_all_checks() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Even with suspicious files, trusted publisher should pass
    fs::write(dir.path().join("malware.exe"), b"bad").unwrap();
    let tier = determine_trust_tier(true, "library");
    assert_eq!(tier, TrustTier::TrustedPublisher);

    let result = tier1_check(dir.path(), &tier);
    assert_eq!(result.status, "trusted");
    assert!(result.findings.is_empty());
}

#[test]
fn suspicious_file_types_detected() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("SKILL.md"), b"---\nname: test\n---\n# Test").unwrap();
    fs::write(dir.path().join("payload.exe"), b"MZ").unwrap();
    fs::write(dir.path().join("helper.sh"), b"#!/bin/bash").unwrap();

    let tier = TrustTier::UnknownSource;
    let result = tier1_check(dir.path(), &tier);

    let suspicious_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category == "suspicious_file")
        .collect();
    assert!(
        suspicious_findings.len() >= 2,
        "expected at least 2 suspicious file findings, got {}",
        suspicious_findings.len()
    );
    assert_eq!(result.status, "flagged");
}

#[test]
fn missing_skill_md_flagged() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("readme.txt"), b"hello").unwrap();

    let tier = TrustTier::UnknownSource;
    let result = tier1_check(dir.path(), &tier);

    let meta_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category == "missing_metadata")
        .collect();
    assert_eq!(meta_findings.len(), 1);
}

#[test]
fn large_files_flagged() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("SKILL.md"), b"---\nname: test\n---\n# Test").unwrap();

    // Create a file > 10MB
    let big = vec![0u8; 11 * 1024 * 1024];
    fs::write(dir.path().join("huge.bin"), &big).unwrap();

    let tier = TrustTier::KnownSource;
    let result = tier1_check(dir.path(), &tier);

    let large_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category == "large_file")
        .collect();
    assert_eq!(large_findings.len(), 1);
}

#[test]
fn tier2_detects_eval_exec_patterns() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("script.js"),
        "const x = eval('dangerous code');\nconst y = exec('cmd');",
    )
    .unwrap();

    let result = tier2_deep_scan(dir.path());

    let pattern_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category == "suspicious_pattern")
        .collect();
    assert!(
        pattern_findings.len() >= 2,
        "expected at least 2 pattern findings, got {}",
        pattern_findings.len()
    );
}

#[test]
fn clean_skill_passes_all_checks() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("SKILL.md"),
        "---\nname: my-clean-skill\n---\n# My Clean Skill\n\nA safe and clean skill.",
    )
    .unwrap();

    let tier = TrustTier::KnownSource;
    let result = tier1_check(dir.path(), &tier);

    assert_eq!(result.status, "trusted");
    assert!(result.findings.is_empty());

    let deep = tier2_deep_scan(dir.path());
    assert!(deep.findings.is_empty());
}

#[test]
fn default_trusted_publishers_not_empty() {
    let publishers = get_default_trusted_publishers();
    assert!(!publishers.is_empty());
    assert!(publishers.contains(&"anthropic-agent-skills"));
}

#[test]
fn determine_trust_tier_known_sources() {
    assert_eq!(determine_trust_tier(false, "local"), TrustTier::KnownSource);
    assert_eq!(
        determine_trust_tier(false, "library"),
        TrustTier::KnownSource
    );
    assert_eq!(determine_trust_tier(false, "git"), TrustTier::UnknownSource);
    assert_eq!(
        determine_trust_tier(true, "git"),
        TrustTier::TrustedPublisher
    );
}
