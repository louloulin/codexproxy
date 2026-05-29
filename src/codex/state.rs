//!
//! Codex State Management Module
//! 
//! Rust implementation aligned with mimo2codex codex/state.test.ts
//! 
//! Features:
//! - Apply Codex configuration
//! - Backup existing files with timestamp
//! - Owner detection for auth.json

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::codex::{atomic_write, backup_file, codex_dir, exists};
use crate::setup::{build_cc_switch_files, HostConfig, ProviderTarget};

/// Owner of the auth.json file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthJsonOwner {
    /// File doesn't exist
    Missing,
    /// File is owned by mimo2codex (has sentinel)
    Mimo2Codex,
    /// File exists but is not owned by mimo2codex
    External,
}

/// Result of apply_codex operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyCodexResult {
    /// Path to backed up auth.json, or null if no backup was created
    pub auth_backup: Option<PathBuf>,
    /// Path to backed up config.toml, or null if no backup was created
    pub toml_backup: Option<PathBuf>,
    /// Timestamp used for backup naming
    pub backup_ts: i64,
    /// Owner of auth.json before the operation
    pub auth_json_owner_before: AuthJsonOwner,
    /// Target provider that was applied
    pub provider: String,
}

/// Detect owner of auth.json file
pub fn detect_auth_json_owner(path: &std::path::Path) -> AuthJsonOwner {
    if !path.exists() {
        return AuthJsonOwner::Missing;
    }
    
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return AuthJsonOwner::Missing,
    };
    
    // Check for mimo2codex sentinel
    if content.contains("mimo2codex-local") {
        AuthJsonOwner::Mimo2Codex
    } else {
        AuthJsonOwner::External
    }
}

/// Apply Codex configuration for a provider
/// Creates ~/.codex directory with auth.json and config.toml
/// Backs up existing files if present
pub fn apply_codex(target: ProviderTarget, host: &HostConfig) -> ApplyCodexResult {
    let codex_path = codex_dir();
    let auth_json_path = codex_path.join("auth.json");
    let config_toml_path = codex_path.join("config.toml");
    
    // Get current timestamp for backup naming
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    
    // Detect current owner of auth.json
    let auth_owner_before = detect_auth_json_owner(&auth_json_path);
    
    // Backup existing files if they exist
    let auth_backup = if exists(&auth_json_path) {
        backup_file(&auth_json_path, ts)
    } else {
        None
    };
    
    let toml_backup = if exists(&config_toml_path) {
        backup_file(&config_toml_path, ts)
    } else {
        None
    };
    
    // Build configuration files
    let files = build_cc_switch_files(host, &target);
    
    // Write new files
    if let Err(e) = atomic_write(&auth_json_path, &files.auth_json) {
        tracing::warn!("Failed to write auth.json: {}", e);
    }
    
    if let Err(e) = atomic_write(&config_toml_path, &files.config_toml) {
        tracing::warn!("Failed to write config.toml: {}", e);
    }
    
    ApplyCodexResult {
        auth_backup,
        toml_backup,
        backup_ts: ts,
        auth_json_owner_before: auth_owner_before,
        provider: match target {
            ProviderTarget::Mimo => "mimo".to_string(),
            ProviderTarget::DeepSeek => "ds".to_string(),
            ProviderTarget::MiniMax => "minimax".to_string(),
        },
    }
}

/// Restore Codex configuration from backup
pub fn restore_codex(backup_ts: i64) -> Result<(), String> {
    let codex_path = codex_dir();
    let auth_json_path = codex_path.join("auth.json");
    let config_toml_path = codex_path.join("config.toml");
    
    // Find backup files with matching timestamp
    let auth_backup = find_backup_with_ts(&auth_json_path, backup_ts)
        .ok_or("Auth backup not found")?;
    let toml_backup = find_backup_with_ts(&config_toml_path, backup_ts)
        .ok_or("Config backup not found")?;
    
    // Restore from backups
    fs::copy(&auth_backup, &auth_json_path)
        .map_err(|e| format!("Failed to restore auth.json: {}", e))?;
    fs::copy(&toml_backup, &config_toml_path)
        .map_err(|e| format!("Failed to restore config.toml: {}", e))?;
    
    // Clean up backups
    fs::remove_file(&auth_backup).ok();
    fs::remove_file(&toml_backup).ok();
    
    Ok(())
}

/// Find backup file with matching timestamp
fn find_backup_with_ts(original: &std::path::Path, ts: i64) -> Option<PathBuf> {
    let dir = original.parent()?;
    let stem = original.file_stem()?.to_str()?;
    let ext = original.extension().and_then(|e| e.to_str());
    
    let entries = fs::read_dir(dir).ok()?;
    
    for entry in entries.flatten() {
        let path = entry.path();
        let filename = path.file_name()?.to_str()?;
        
        // Check if filename matches pattern: {stem}.bak.{ts}.{pid}[.{ext}]
        let expected_prefix = format!("{}.bak.{}", stem, ts);
        if filename.starts_with(&expected_prefix) {
            // Verify it's not the original file itself
            if path != original {
                return Some(path);
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_clean_codex_dir() -> TempDir {
        // CRITICAL: Always clean up first to handle any pollution from other tests
        std::env::remove_var("CODEX_HOME");

        let temp_dir = TempDir::new().unwrap();
        let codex_home = temp_dir.path().to_string_lossy().to_string();

        // Double-check cleanup
        std::env::remove_var("CODEX_HOME");
        std::env::set_var("CODEX_HOME", &codex_home);

        // Verify CODEX_HOME is set correctly
        assert_eq!(
            std::env::var("CODEX_HOME").ok().as_ref(),
            Some(&codex_home),
            "CODEX_HOME not set correctly"
        );

        // Create the .codex subdirectory
        let codex_path = codex_dir();
        fs::create_dir_all(&codex_path).ok();

        temp_dir
    }

    fn cleanup_codex_env() {
        std::env::remove_var("CODEX_HOME");
    }

    #[test]
    fn test_detect_auth_json_owner_missing() {
        let temp_dir = setup_clean_codex_dir();

        let path = codex_dir().join("missing.json");
        assert_eq!(detect_auth_json_owner(&path), AuthJsonOwner::Missing);

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_detect_auth_json_owner_mimo2codex() {
        let temp_dir = setup_clean_codex_dir();

        let codex_path = codex_dir();
        let auth_path = codex_path.join("auth.json");
        let content = r#"{"OPENAI_API_KEY": "mimo2codex-local"}"#;

        // Create parent directory and write file
        fs::create_dir_all(&codex_path).ok();
        fs::write(&auth_path, content).unwrap();

        assert_eq!(detect_auth_json_owner(&auth_path), AuthJsonOwner::Mimo2Codex);

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_detect_auth_json_owner_external() {
        let temp_dir = setup_clean_codex_dir();

        let codex_path = codex_dir();
        let auth_path = codex_path.join("auth.json");
        let content = r#"{"OPENAI_API_KEY": "sk-real-key"}"#;

        // Create parent directory and write file
        fs::create_dir_all(&codex_path).ok();
        fs::write(&auth_path, content).unwrap();

        assert_eq!(detect_auth_json_owner(&auth_path), AuthJsonOwner::External);

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_apply_codex_creates_files() {
        let temp_dir = setup_clean_codex_dir();

        // Verify CODEX_HOME is set
        assert_eq!(
            std::env::var("CODEX_HOME").ok(),
            Some(temp_dir.path().to_string_lossy().to_string())
        );

        let host = HostConfig::new("127.0.0.1", 8788);
        let _result = apply_codex(ProviderTarget::Mimo, &host);

        // Verify files were created in the correct location
        let expected_codex_dir = temp_dir.path().to_path_buf();
        assert_eq!(codex_dir(), expected_codex_dir);

        // Verify files were created
        assert!(exists(&expected_codex_dir.join("auth.json")), "auth.json not created");
        assert!(exists(&expected_codex_dir.join("config.toml")), "config.toml not created");

        // Check auth.json has sentinel
        let auth_content = fs::read_to_string(expected_codex_dir.join("auth.json")).unwrap();
        assert!(auth_content.contains("mimo2codex-local"), "auth.json should have mimo2codex-local sentinel");

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_apply_codex_backs_up_existing() {
        let temp_dir = setup_clean_codex_dir();

        // Verify CODEX_HOME is set correctly
        let expected_codex_dir = temp_dir.path().to_path_buf();
        assert_eq!(codex_dir(), expected_codex_dir);

        // Write existing files
        let auth_path = expected_codex_dir.join("auth.json");
        let config_path = expected_codex_dir.join("config.toml");

        fs::create_dir_all(&expected_codex_dir).ok();
        fs::write(&auth_path, r#"{"OPENAI_API_KEY": "sk-old-backup"}"#).unwrap();
        fs::write(&config_path, "old config content").unwrap();

        let host = HostConfig::new("127.0.0.1", 8788);
        let result = apply_codex(ProviderTarget::Mimo, &host);

        // Verify backups were created
        assert!(result.auth_backup.is_some(), "auth backup should exist");
        assert!(result.toml_backup.is_some(), "toml backup should exist");

        // Read backup and verify it contains old content
        let auth_backup_content = fs::read_to_string(result.auth_backup.unwrap()).unwrap();
        assert!(auth_backup_content.contains("sk-old-backup"), "auth backup should contain old content");

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_apply_codex_records_timestamp() {
        let temp_dir = setup_clean_codex_dir();

        let host = HostConfig::new("127.0.0.1", 8788);
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let result = apply_codex(ProviderTarget::Mimo, &host);

        assert!(result.backup_ts >= before);

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_backup_ts_encoding_in_filename() {
        let temp_dir = setup_clean_codex_dir();

        let codex_path = codex_dir();
        let test_file = codex_path.join("test_file.txt");
        fs::create_dir_all(&codex_path).ok();
        fs::write(&test_file, "content").unwrap();

        let ts = 999999;
        let backup = backup_file(&test_file, ts).unwrap();

        let filename = backup.file_name().unwrap().to_str().unwrap();
        assert!(filename.contains(&format!(".bak.{}.", ts)));

        cleanup_codex_env();
        drop(temp_dir);
    }
}
