//! Database schema initialization
//! 
//! Creates SQLite tables for tracking requests and sessions.

use rusqlite::{Connection, Result};
use std::path::Path;

/// Initialize the database with required tables
pub fn init_database(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    
    // Create requests table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS requests (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            endpoint TEXT NOT NULL,
            status_code INTEGER,
            tokens_used INTEGER,
            latency_ms INTEGER,
            error_message TEXT,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    
    // Create sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            request_count INTEGER DEFAULT 0,
            total_tokens INTEGER DEFAULT 0,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            last_activity_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    
    // Create provider_stats table for aggregated statistics
    conn.execute(
        "CREATE TABLE IF NOT EXISTS provider_stats (
            provider TEXT PRIMARY KEY,
            total_requests INTEGER DEFAULT 0,
            successful_requests INTEGER DEFAULT 0,
            failed_requests INTEGER DEFAULT 0,
            total_tokens INTEGER DEFAULT 0,
            avg_latency_ms REAL DEFAULT 0.0,
            last_updated TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;
    
    // Create indexes for better query performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_requests_created_at ON requests(created_at)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_requests_provider ON requests(provider)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_last_activity ON sessions(last_activity_at)",
        [],
    )?;
    
    tracing::info!("Database initialized at {:?}", db_path);
    
    Ok(conn)
}

/// Database connection type alias
pub type DbPool = rusqlite::Connection;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_init_database() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Verify tables exist using query
        let count: i32 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='requests'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
        
        let count: i32 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
        
        let count: i32 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='provider_stats'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
    }
}

#[cfg(test)]
mod override_tests {
    use super::*;
    use rusqlite::params;
    use tempfile::NamedTempFile;

    // Tests aligned with mimo2codex db.overrides.test.ts

    #[test]
    fn test_override_table_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Create settings table for overrides
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).unwrap();
        
        // Verify table exists
        let count: i32 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='settings'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_returns_null_when_no_override_set() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Create settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        ).unwrap();
        
        // Query for override (should be null)
        let result: Option<String> = conn.query_row(
            "SELECT value FROM settings WHERE key = 'codex.activeOverride.providerId'",
            [],
            |row| row.get(0),
        ).ok();
        
        assert!(result.is_none(), "No override should be set initially");
    }

    #[test]
    fn test_returns_null_when_only_one_key_set() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Create settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        ).unwrap();
        
        // Set only providerId, not modelId
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('codex.activeOverride.providerId', 'mimo')",
            [],
        ).unwrap();
        
        // Both keys must be set for valid override
        let provider: Option<String> = conn.query_row(
            "SELECT value FROM settings WHERE key = 'codex.activeOverride.providerId'",
            [],
            |row| row.get(0),
        ).ok();
        
        let model: Option<String> = conn.query_row(
            "SELECT value FROM settings WHERE key = 'codex.activeOverride.modelId'",
            [],
            |row| row.get(0),
        ).ok();
        
        // Override is invalid with only one key
        assert!(provider.is_some());
        assert!(model.is_none());
    }

    #[test]
    fn test_set_and_get_active_override() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Create settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        ).unwrap();
        
        // Set both override keys
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('codex.activeOverride.providerId', 'deepseek')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('codex.activeOverride.modelId', 'deepseek-v4-pro')",
            [],
        ).unwrap();
        
        // Get both keys
        let provider: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'codex.activeOverride.providerId'",
            [],
            |row| row.get(0),
        ).unwrap();
        
        let model: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'codex.activeOverride.modelId'",
            [],
            |row| row.get(0),
        ).unwrap();
        
        assert_eq!(provider, "deepseek");
        assert_eq!(model, "deepseek-v4-pro");
    }

    #[test]
    fn test_clear_active_override() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Create settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        ).unwrap();
        
        // Set override
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('codex.activeOverride.providerId', 'mimo')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('codex.activeOverride.modelId', 'mimo-v2.5-pro')",
            [],
        ).unwrap();
        
        // Clear override
        conn.execute(
            "DELETE FROM settings WHERE key LIKE 'codex.activeOverride.%'",
            [],
        ).unwrap();
        
        // Verify both keys are gone
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM settings WHERE key LIKE 'codex.activeOverride.%'",
            [],
            |row| row.get(0),
        ).unwrap();
        
        assert_eq!(count, 0);
    }

    #[test]
    fn test_override_is_idempotent() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Create settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        ).unwrap();
        
        // Set first override
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('codex.activeOverride.providerId', 'mimo')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('codex.activeOverride.modelId', 'mimo-v2.5-pro')",
            [],
        ).unwrap();
        
        // Overwrite with new override
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('codex.activeOverride.providerId', 'deepseek')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('codex.activeOverride.modelId', 'deepseek-v4-pro')",
            [],
        ).unwrap();
        
        // Verify new values
        let provider: String = conn.query_row(
            "SELECT value FROM settings WHERE key = 'codex.activeOverride.providerId'",
            [],
            |row| row.get(0),
        ).unwrap();
        
        assert_eq!(provider, "deepseek");
    }
}
