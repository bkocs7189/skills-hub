use std::fs;
use std::path::Path;

use serde::Serialize;
use walkdir::WalkDir;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[allow(dead_code)]
pub enum TrustTier {
    TrustedPublisher,
    KnownSource,
    UnknownSource,
    Flagged,
}

#[derive(Clone, Debug, Serialize)]
pub struct SecurityFinding {
    pub severity: String,
    pub category: String,
    pub description: String,
    pub file_path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SecurityResult {
    pub tier: TrustTier,
    pub status: String,
    pub findings: Vec<SecurityFinding>,
}

const SUSPICIOUS_EXTENSIONS: &[&str] = &[".exe", ".dll", ".so", ".dylib", ".sh", ".bat", ".ps1"];

const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB

const SUSPICIOUS_CODE_PATTERNS: &[(&str, &str)] = &[
    ("eval(", "eval() call detected"),
    ("exec(", "exec() call detected"),
    ("subprocess", "subprocess usage detected"),
    ("os.system", "os.system() call detected"),
    ("process.env", "environment variable access (process.env)"),
    ("std::env", "environment variable access (std::env)"),
    ("os.environ", "environment variable access (os.environ)"),
];

const TEXT_EXTENSIONS: &[&str] = &[
    ".md", ".txt", ".yaml", ".yml", ".json", ".toml", ".xml", ".html", ".css", ".js", ".ts", ".py",
    ".rs", ".go", ".rb", ".sh", ".bat", ".ps1", ".cfg", ".ini", ".conf",
];

pub fn get_default_trusted_publishers() -> Vec<&'static str> {
    vec![
        "claude-plugins-official",
        "anthropic-agent-skills",
        "claude-code-templates",
        "claude-canvas",
    ]
}

pub fn determine_trust_tier(library_trusted: bool, source_type: &str) -> TrustTier {
    if library_trusted {
        return TrustTier::TrustedPublisher;
    }
    if source_type == "local" {
        return TrustTier::KnownSource;
    }
    // "library" source_type means from a known (but not trusted) library
    if source_type == "library" {
        return TrustTier::KnownSource;
    }
    TrustTier::UnknownSource
}

pub fn tier1_check(asset_path: &Path, trust_tier: &TrustTier) -> SecurityResult {
    if *trust_tier == TrustTier::TrustedPublisher {
        return SecurityResult {
            tier: trust_tier.clone(),
            status: "trusted".to_string(),
            findings: vec![],
        };
    }

    let mut findings: Vec<SecurityFinding> = Vec::new();

    // Check for suspicious file types
    if let Ok(entries) = fs::read_dir(asset_path) {
        check_suspicious_files_recursive(asset_path, &mut findings);

        // We already walked above; this block is just for the read_dir guard.
        drop(entries);
    }

    // Check for valid metadata: SKILL.md or PLUGIN.md
    let has_skill_md = asset_path.join("SKILL.md").exists();
    let has_plugin_md = asset_path.join("PLUGIN.md").exists();
    if !has_skill_md && !has_plugin_md {
        findings.push(SecurityFinding {
            severity: "medium".to_string(),
            category: "missing_metadata".to_string(),
            description: "No SKILL.md or PLUGIN.md found — missing metadata file".to_string(),
            file_path: None,
        });
    }

    // Check file sizes
    check_large_files(asset_path, &mut findings);

    // Check for base64 encoded content in text files
    check_base64_blobs(asset_path, &mut findings);

    let status = if findings.iter().any(|f| f.severity == "high") {
        "flagged".to_string()
    } else if findings.is_empty() {
        "trusted".to_string()
    } else {
        "unknown_source".to_string()
    };

    SecurityResult {
        tier: trust_tier.clone(),
        status,
        findings,
    }
}

pub fn tier2_deep_scan(asset_path: &Path) -> SecurityResult {
    let mut findings: Vec<SecurityFinding> = Vec::new();

    for entry in WalkDir::new(asset_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();

        if !TEXT_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let relative = path
            .strip_prefix(asset_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Check for suspicious code patterns
        for (line_num, line) in content.lines().enumerate() {
            for (pattern, desc) in SUSPICIOUS_CODE_PATTERNS {
                if line.contains(pattern) {
                    findings.push(SecurityFinding {
                        severity: "medium".to_string(),
                        category: "suspicious_pattern".to_string(),
                        description: format!("{} (line {})", desc, line_num + 1),
                        file_path: Some(relative.clone()),
                    });
                }
            }

            // Check for hardcoded IP address URLs
            if contains_ip_url(line) {
                findings.push(SecurityFinding {
                    severity: "high".to_string(),
                    category: "suspicious_pattern".to_string(),
                    description: format!(
                        "Hardcoded URL with IP address detected (line {})",
                        line_num + 1
                    ),
                    file_path: Some(relative.clone()),
                });
            }
        }

        // Check for obfuscated code (high entropy strings > 100 chars)
        check_high_entropy_strings(&content, &relative, &mut findings);
    }

    let status = if findings.iter().any(|f| f.severity == "high") {
        "flagged".to_string()
    } else if findings.is_empty() {
        "trusted".to_string()
    } else {
        "unknown_source".to_string()
    };

    SecurityResult {
        tier: TrustTier::UnknownSource,
        status,
        findings,
    }
}

fn check_suspicious_files_recursive(dir: &Path, findings: &mut Vec<SecurityFinding>) {
    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let name_lower = name.to_lowercase();
        for ext in SUSPICIOUS_EXTENSIONS {
            if name_lower.ends_with(ext) {
                let relative = path
                    .strip_prefix(dir)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                findings.push(SecurityFinding {
                    severity: "high".to_string(),
                    category: "suspicious_file".to_string(),
                    description: format!("Suspicious file type: {}", ext),
                    file_path: Some(relative),
                });
            }
        }
    }
}

fn check_large_files(dir: &Path, findings: &mut Vec<SecurityFinding>) {
    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if meta.len() > MAX_FILE_SIZE {
                let relative = entry
                    .path()
                    .strip_prefix(dir)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .to_string();
                findings.push(SecurityFinding {
                    severity: "medium".to_string(),
                    category: "large_file".to_string(),
                    description: format!(
                        "File exceeds 10MB ({:.1}MB)",
                        meta.len() as f64 / (1024.0 * 1024.0)
                    ),
                    file_path: Some(relative),
                });
            }
        }
    }
}

fn check_base64_blobs(dir: &Path, findings: &mut Vec<SecurityFinding>) {
    for entry in WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        if !TEXT_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Look for long base64-like strings (>1KB = >1365 base64 chars)
        for (line_num, line) in content.lines().enumerate() {
            if has_base64_blob(line, 1365) {
                let relative = path
                    .strip_prefix(dir)
                    .unwrap_or(path)
                    .to_string_lossy()
                    .to_string();
                findings.push(SecurityFinding {
                    severity: "medium".to_string(),
                    category: "encoded_content".to_string(),
                    description: format!("Large base64 blob detected (line {})", line_num + 1),
                    file_path: Some(relative),
                });
                break; // one finding per file is enough
            }
        }
    }
}

fn has_base64_blob(line: &str, min_len: usize) -> bool {
    // Look for contiguous base64-like sequences
    let mut run = 0usize;
    for ch in line.chars() {
        if ch.is_ascii_alphanumeric() || ch == '+' || ch == '/' || ch == '=' {
            run += 1;
            if run >= min_len {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

fn contains_ip_url(line: &str) -> bool {
    // Match http(s)://N.N.N.N patterns
    let lower = line.to_lowercase();
    for prefix in ["http://", "https://"] {
        if let Some(idx) = lower.find(prefix) {
            let rest = &lower[idx + prefix.len()..];
            // Check if what follows looks like an IP address (digit.digit...)
            let host_end = rest
                .find(|c: char| c == '/' || c == ':' || c == '?' || c == '#' || c.is_whitespace())
                .unwrap_or(rest.len());
            let host = &rest[..host_end];
            if looks_like_ip(host) {
                return true;
            }
        }
    }
    false
}

fn looks_like_ip(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|p| p.parse::<u8>().is_ok())
}

fn check_high_entropy_strings(content: &str, relative: &str, findings: &mut Vec<SecurityFinding>) {
    for (line_num, line) in content.lines().enumerate() {
        // Look for long strings of non-whitespace that appear random
        for word in line.split_whitespace() {
            if word.len() > 100 && is_high_entropy(word) {
                findings.push(SecurityFinding {
                    severity: "medium".to_string(),
                    category: "obfuscated_code".to_string(),
                    description: format!(
                        "High-entropy string ({} chars) may indicate obfuscated code (line {})",
                        word.len(),
                        line_num + 1
                    ),
                    file_path: Some(relative.to_string()),
                });
                return; // one finding per file
            }
        }
    }
}

fn is_high_entropy(s: &str) -> bool {
    // Shannon entropy estimation
    let len = s.len() as f64;
    if len == 0.0 {
        return false;
    }
    let mut freq = [0u32; 256];
    for &b in s.as_bytes() {
        freq[b as usize] += 1;
    }
    let mut entropy = 0.0f64;
    for &count in &freq {
        if count == 0 {
            continue;
        }
        let p = count as f64 / len;
        entropy -= p * p.log2();
    }
    // Threshold: natural language ~4-5 bits, random ~6+ bits
    entropy > 4.5
}

#[cfg(test)]
#[path = "tests/security_gate.rs"]
mod tests;
