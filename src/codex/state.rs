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

    #[test]
    fn test_detect_auth_json_owner_missing() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());
        
        let path = codex_dir().join("missing.json");
        assert_eq!(detect_auth_json_owner(&path), AuthJsonOwner::Missing);
        
        std::env::remove_var("CODEX_HOME");
        drop(temp_dir);
    }

    #[test]
    fn test_detect_auth_json_owner_mimo2codex() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());
        
        let path = codex_dir().join("auth.json");
        let content = r#"{"OPENAI_API_KEY": "mimo2codex-local"}"#;
        fs::write(&path, content).unwrap();

        assert_eq!(detect_auth_json_owner(&path), AuthJsonOwner::Mimo2Codex);
        
        std::env::remove_var("CODEX_HOME");
        drop(temp_dir);
    }

    #[test]
    fn test_detect_auth_json_owner_external() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());
        
        let path = codex_dir().join("auth.json");
        let content = r#"{"OPENAI_API_KEY": "sk-real-key"}"#;
        fs::write(&path, content).unwrap();

        assert_eq!(detect_auth_json_owner(&path), AuthJsonOwner::External);
        
        std::env::remove_var("CODEX_HOME");
        drop(temp_dir);
    }

    #[test]
    fn test_apply_codex_creates_files() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());
        
        let host = HostConfig::new("127.0.0.1", 8788);
        let result = apply_codex(ProviderTarget::Mimo, &host);

        assert_eq!(result.auth_json_owner_before, AuthJsonOwner::Missing);
        assert!(result.auth_backup.is_none());
        assert!(result.toml_backup.is_none());

        let codex_path = codex_dir();
        assert!(exists(&codex_path.join("auth.json")), "auth.json not created at {:?}", codex_path);
        assert!(exists(&codex_path.join("config.toml")));

        let auth_content = fs::read_to_string(codex_path.join("auth.json")).unwrap();
        assert!(auth_content.contains("mimo2codex-local"));

        let toml_content = fs::read_to_string(codex_path.join("config.toml")).unwrap();
        assert!(toml_content.contains("mimo-v2.5-pro"));
        assert!(toml_content.contains("127.0.0.1:8788"));
        
        std::env::remove_var("CODEX_HOME");
        drop(temp_dir);
    }

    #[test]
    fn test_apply_codex_backs_up_existing() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());
        
        let codex_path = codex_dir();
        fs::create_dir_all(&codex_path).unwrap();

        fs::write(codex_path.join("auth.json"), r#"{"OPENAI_API_KEY": "sk-old"}"#).unwrap();
        fs::write(codex_path.join("config.toml"), "old config").unwrap();

        let host = HostConfig::new("127.0.0.1", 8788);
        let result = apply_codex(ProviderTarget::Mimo, &host);

        assert_eq!(result.auth_json_owner_before, AuthJsonOwner::External);
        assert!(result.auth_backup.is_some());
        assert!(result.toml_backup.is_some());

        let auth_backup_content = fs::read_to_string(result.auth_backup.unwrap()).unwrap();
        assert!(auth_backup_content.contains("sk-old"));
        
        std::env::remove_var("CODEX_HOME");
        drop(temp_dir);
    }

    #[test]
    fn test_apply_codex_records_timestamp() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());
        
        let host = HostConfig::new("127.0.0.1", 8788);
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let result = apply_codex(ProviderTarget::Mimo, &host);

        assert!(result.backup_ts >= before);
        
        std::env::remove_var("CODEX_HOME");
        drop(temp_dir);
    }

    #[test]
    fn test_backup_ts_encoding_in_filename() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());
        
        let codex_path = codex_dir();
        let test_file = codex_path.join("test_file.txt");
        fs::write(&test_file, "content").unwrap();

        let ts = 999999;
        let backup = backup_file(&test_file, ts).unwrap();

        let filename = backup.file_name().unwrap().to_str().unwrap();
        assert!(filename.contains(&format!(".bak.{}.", ts)));
        
        std::env::remove_var("CODEX_HOME");
        drop(temp_dir);
    }
}
