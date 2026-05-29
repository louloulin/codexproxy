//!
//! Codex Types Module
//! 
//! Core type definitions for codex-switch functionality
//! Aligned with mimo2codex src/codex/state.ts and src/codex/files.ts
//!

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Owner of the auth.json file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthJsonOwner {
    /// File doesn't exist
    Missing,
    /// File is owned by mimo2codex (has sentinel)
    Mimo2Codex,
    /// File exists but is not owned by mimo2codex
    External,
}

impl Default for AuthJsonOwner {
    fn default() -> Self {
        AuthJsonOwner::Missing
    }
}

impl std::fmt::Display for AuthJsonOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthJsonOwner::Missing => write!(f, "missing"),
            AuthJsonOwner::Mimo2Codex => write!(f, "mimo2codex"),
            AuthJsonOwner::External => write!(f, "external"),
        }
    }
}

/// Backup entry from listBackups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    /// Full path to the backup file
    pub path: PathBuf,
    /// Timestamp from filename
    pub ts: i64,
    /// Whether this backup is preserved (exempt from pruning)
    pub preserved: bool,
}

/// Backup pair (auth + config paired by timestamp)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPair {
    /// Timestamp of this backup pair
    pub ts: i64,
    /// Path to auth.json backup, if exists
    pub auth_backup: Option<PathBuf>,
    /// Path to config.toml backup, if exists
    pub toml_backup: Option<PathBuf>,
    /// True when at least one half is tagged .preserve
    pub preserved: bool,
    /// Model extracted from config.toml backup (best-effort)
    pub model: Option<String>,
    /// Provider extracted from config.toml backup (best-effort)
    pub provider: Option<String>,
    /// Owner type inferred from auth.json backup content
    pub auth_backup_owner: AuthJsonOwner,
}

/// Result of apply_codex operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyCodexResult {
    /// Timestamp used for backup naming
    pub backup_ts: i64,
    /// Path to backed up auth.json, or null if no backup was created
    pub auth_backup: Option<PathBuf>,
    /// Path to backed up config.toml, or null if no backup was created
    pub toml_backup: Option<PathBuf>,
    /// Owner of auth.json before the operation
    pub auth_json_owner_before: AuthJsonOwner,
    /// True when this backup was tagged .preserve
    /// Happens when previous auth.json belonged to external owner
    pub preserved: bool,
}

/// Codex state for Admin UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexState {
    /// Codex directory path
    pub codex_dir: PathBuf,
    /// Auth json path
    pub auth_path: PathBuf,
    /// Config toml path
    pub toml_path: PathBuf,
    /// Owner of auth.json
    pub auth_json_owner: AuthJsonOwner,
    /// Whether auth.json exists
    pub auth_json_exists: bool,
    /// Whether config.toml exists
    pub config_toml_exists: bool,
    /// Raw config.toml content (best-effort)
    pub config_toml_text: Option<String>,
    /// List of all backup pairs
    pub backups: Vec<BackupPair>,
}

/// Active override for runtime model switching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveOverride {
    /// Provider ID for override
    pub provider_id: String,
    /// Model ID for override
    pub model_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_json_owner_display() {
        assert_eq!(AuthJsonOwner::Missing.to_string(), "missing");
        assert_eq!(AuthJsonOwner::Mimo2Codex.to_string(), "mimo2codex");
        assert_eq!(AuthJsonOwner::External.to_string(), "external");
    }

    #[test]
    fn test_backup_entry_serde() {
        let entry = BackupEntry {
            path: PathBuf::from("/test/auth.json.bak.123.456"),
            ts: 123,
            preserved: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: BackupEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.ts, 123);
        assert!(parsed.preserved);
    }

    #[test]
    fn test_backup_pair_serde() {
        let pair = BackupPair {
            ts: 456,
            auth_backup: Some(PathBuf::from("/test/auth.json.bak.456")),
            toml_backup: Some(PathBuf::from("/test/config.toml.bak.456")),
            preserved: true,
            model: Some("gpt-4".to_string()),
            provider: Some("openai".to_string()),
            auth_backup_owner: AuthJsonOwner::External,
        };
        let json = serde_json::to_string(&pair).unwrap();
        let parsed: BackupPair = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.ts, 456);
        assert_eq!(parsed.model.as_deref(), Some("gpt-4"));
    }
}
