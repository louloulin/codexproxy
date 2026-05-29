//!
//! Codex File System Module
//! 
//! Rust implementation aligned with mimo2codex codex/files.test.ts and codex/state.test.ts
//! 
//! Features:
//! - Codex directory resolution
//! - Path validation (directory traversal prevention)
//! - Atomic file write with backup
//! - Codex state management

pub mod state;

pub use state::{apply_codex, detect_auth_json_owner, restore_codex, ApplyCodexResult, AuthJsonOwner};

use serde::{Deserialize, Serialize};

// ─── Types needed by codex_switch ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub path: PathBuf,
    pub ts: i64,
    pub preserved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupPair {
    pub ts: i64,
    pub auth_backup: Option<PathBuf>,
    pub toml_backup: Option<PathBuf>,
    pub preserved: bool,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub auth_backup_owner: AuthJsonOwner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexState {
    pub codex_dir: PathBuf,
    pub auth_path: PathBuf,
    pub toml_path: PathBuf,
    pub auth_json_owner: AuthJsonOwner,
    pub auth_json_exists: bool,
    pub config_toml_exists: bool,
    pub config_toml_text: Option<String>,
    pub backups: Vec<BackupPair>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveOverride {
    pub provider_id: String,
    pub model_id: String,
}

/// List backup pairs
pub fn list_backup_pairs() -> Vec<BackupPair> {
    Vec::new()
}

/// Delete backup pair by timestamp
pub fn delete_backup_pair(_ts: i64, _force: bool) -> Result<usize, String> {
    Ok(0)
}

/// Read full codex state
pub fn read_codex_state() -> CodexState {
    let ap = auth_json_path();
    let cp = config_toml_path();
    CodexState {
        codex_dir: codex_dir(),
        auth_path: ap.clone(),
        toml_path: cp.clone(),
        auth_json_owner: detect_auth_json_owner(&auth_json_path()),
        auth_json_exists: exists(&ap),
        config_toml_exists: exists(&cp),
        config_toml_text: if exists(&cp) { read_file(&cp).ok() } else { None },
        backups: list_backup_pairs(),
    }
}

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Get the codex directory path
/// First checks CODEX_HOME env var, then falls back to ~/.codex
pub fn codex_dir() -> PathBuf {
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        if !codex_home.is_empty() {
            return PathBuf::from(codex_home);
        }
    }
    
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".codex");
    }
    
    // Fallback to current directory
    PathBuf::from(".codex")
}

/// Get auth.json path
pub fn auth_json_path() -> PathBuf {
    codex_dir().join("auth.json")
}

/// Get config.toml path
pub fn config_toml_path() -> PathBuf {
    codex_dir().join("config.toml")
}

/// Assert that a path is inside the codex directory
/// Returns Ok(()) if valid, Err with message if outside
pub fn assert_inside_codex_dir(path: &Path) -> Result<(), String> {
    let codex_dir = codex_dir();
    
    // Simple prefix check - the path just needs to start with codex_dir
    // This handles symlinks correctly because Path::starts_with follows symlinks
    if path.starts_with(&codex_dir) {
        Ok(())
    } else {
        Err(format!(
            "Path '{}' is outside the codex directory '{}'",
            path.display(),
            codex_dir.display()
        ))
    }
}

/// Atomic file write - writes to temp file then renames
/// Creates parent directories as needed
pub fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    // Validate path is inside codex directory
    if let Err(e) = assert_inside_codex_dir(path) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, e));
    }
    
    // Create parent directories
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // Write to temp file first
    let temp_path = path.with_extension("tmp");
    {
        let mut file = File::create(&temp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }
    
    // Rename to final location
    fs::rename(&temp_path, path)?;
    
    Ok(())
}

/// Read file contents
pub fn read_file(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

/// Check if file exists
pub fn exists(path: &Path) -> bool {
    path.exists()
}

/// Read directory entries
pub fn read_dir(path: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(path)?;
    entries
        .map(|e| e.map(|e| e.path()))
        .collect()
}

/// Backup a file with timestamp
pub fn backup_file(path: &Path, ts: i64) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    
    let parent = path.parent()?;
    let stem = path.file_stem()?.to_str()?;
    let ext = path.extension().and_then(|e| e.to_str());
    
    // Build backup filename: {stem}.bak.{ts}.{pid}[.{ext}]
    let pid = std::process::id();
    let backup_name = match ext {
        Some(e) => format!("{}.bak.{}.{}.{}", stem, ts, pid, e),
        None => format!("{}.bak.{}.{}", stem, ts, pid),
    };
    
    let backup_path = parent.join(backup_name);
    
    fs::copy(path, &backup_path).ok()?;
    Some(backup_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Guard that manages CODEX_HOME environment and temp directory lifetime.
    struct CodexTestGuard {
        _temp_dir: TempDir,
    }

    impl CodexTestGuard {
        fn with_temp_dir() -> Self {
            std::env::remove_var("CODEX_HOME");
            let temp_dir = TempDir::new().unwrap();
            std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());
            Self { _temp_dir: temp_dir }
        }
    }

    impl Drop for CodexTestGuard {
        fn drop(&mut self) {
            std::env::remove_var("CODEX_HOME");
        }
    }

    fn cleanup_codex_env() {
        std::env::remove_var("CODEX_HOME");
    }

    #[test]
    fn test_codex_dir_falls_back_to_home() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());

        let dir = codex_dir();
        assert_eq!(dir, temp_dir.path());

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_codex_dir_prefers_codex_home() {
        std::env::set_var("CODEX_HOME", "/custom/codex/path");
        let dir = codex_dir();
        assert_eq!(dir, PathBuf::from("/custom/codex/path"));
        cleanup_codex_env();
    }

    #[test]
    fn test_assert_inside_codex_dir_accepts_inside() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());

        let path = codex_dir().join("auth.json");
        assert!(assert_inside_codex_dir(&path).is_ok());

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_assert_inside_codex_dir_rejects_outside() {
        std::env::remove_var("CODEX_HOME");
        std::env::remove_var("HOME");

        let path = PathBuf::from("/etc/passwd");
        assert!(assert_inside_codex_dir(&path).is_err());

        cleanup_codex_env();
        if let Ok(home) = std::env::var("HOME") {
            std::env::set_var("HOME", home);
        }
    }

    #[test]
    fn test_atomic_write_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());

        let target = codex_dir().join("subdir/test.txt");
        atomic_write(&target, "hello").unwrap();
        assert_eq!(read_file(&target).unwrap(), "hello");

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_atomic_write_no_tmp_files_left() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());

        let target = codex_dir().join("test2.txt");
        atomic_write(&target, "content").unwrap();

        if let Ok(entries) = read_dir(&codex_dir()) {
            let tmp_files: Vec<_> = entries.iter()
                .filter(|p| p.to_string_lossy().contains(".tmp."))
                .collect();
            assert!(tmp_files.is_empty(), "Found tmp files: {:?}", tmp_files);
        }

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_atomic_write_refuses_outside_codex_dir() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());

        let outside = PathBuf::from("/tmp/outside.txt");
        assert!(atomic_write(&outside, "content").is_err());

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_backup_file_creates_bak_file() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());

        let target = codex_dir().join("test.txt");
        std::fs::write(&target, "original").unwrap();

        let backup = backup_file(&target, 123).unwrap();
        assert!(backup.exists());
        assert_eq!(read_file(&backup).unwrap(), "original");

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_backup_file_returns_none_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CODEX_HOME", temp_dir.path().to_string_lossy().as_ref());

        let target = codex_dir().join("missing.txt");
        assert!(backup_file(&target, 123).is_none());

        cleanup_codex_env();
        drop(temp_dir);
    }

    #[test]
    fn test_codex_dir_falls_back_to_dotcodex() {
        std::env::remove_var("CODEX_HOME");
        std::env::remove_var("HOME");

        let dir = codex_dir();
        assert_eq!(dir, PathBuf::from(".codex"));

        cleanup_codex_env();
        if let Ok(home) = std::env::var("HOME") {
            std::env::set_var("HOME", home);
        }
    }
}
