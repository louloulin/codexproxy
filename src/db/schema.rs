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

    // Auth tables: users, sessions, api keys
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            display_name TEXT,
            email TEXT,
            avatar_url TEXT,
            password_hash TEXT,
            is_admin INTEGER DEFAULT 0,
            status TEXT DEFAULT 'active',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_sessions (
            token TEXT PRIMARY KEY,
            user_id INTEGER NOT NULL,
            expires_at TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_api_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            key_prefix TEXT NOT NULL,
            key_hash TEXT NOT NULL,
            scopes TEXT DEFAULT 'api',
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            last_used_at TEXT,
            revoked_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS upstream_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            provider_id TEXT NOT NULL,
            encrypted_key TEXT NOT NULL,
            key_prefix TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            UNIQUE(user_id, provider_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS custom_models (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider_id TEXT NOT NULL,
            upstream_id TEXT NOT NULL,
            display_name TEXT,
            context_window INTEGER,
            max_output_tokens INTEGER,
            is_builtin INTEGER DEFAULT 0,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(provider_id, upstream_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS oauth_clients (
            provider TEXT PRIMARY KEY,
            client_id TEXT NOT NULL,
            client_secret TEXT,
            callback_url TEXT NOT NULL,
            enabled INTEGER DEFAULT 0,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS codex_config_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER,
            provider_id TEXT,
            model_id TEXT,
            auth_backup_path TEXT,
            config_backup_path TEXT,
            preserved INTEGER DEFAULT 0,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
        )",
        [],
    )?;

    // Indexes for auth tables
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_user_sessions_token ON user_sessions(token)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_user_api_keys_hash ON user_api_keys(key_hash)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_user_api_keys_user ON user_api_keys(user_id)",
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

#[cfg(test)]
mod auth_tests {
    use super::*;
    use rusqlite::params;
    use tempfile::NamedTempFile;

    // Tests aligned with mimo2codex db.auth.test.ts

    #[test]
    fn test_creates_all_auth_tables() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Create all auth tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                display_name TEXT,
                password_hash TEXT,
                is_admin INTEGER DEFAULT 0,
                status TEXT DEFAULT 'active',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_sessions (
                token TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                expires_at TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                key_prefix TEXT NOT NULL,
                key_hash TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                last_used_at TEXT,
                revoked_at TEXT
            )",
            [],
        ).unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS codex_config_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER,
                provider_id TEXT,
                model_id TEXT,
                auth_backup_path TEXT,
                config_backup_path TEXT,
                preserved INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).unwrap();
        
        // Verify tables exist via count query
        let users_exists: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='users'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(users_exists, 1);
        
        let sessions_exists: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='user_sessions'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(sessions_exists, 1);
        
        let api_keys_exists: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='user_api_keys'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(api_keys_exists, 1);
        
        let history_exists: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='codex_config_history'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(history_exists, 1);
    }

    #[test]
    fn test_user_crud_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Create users table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                display_name TEXT,
                password_hash TEXT,
                is_admin INTEGER DEFAULT 0,
                status TEXT DEFAULT 'active'
            )",
            [],
        ).unwrap();
        
        // Create user
        conn.execute(
            "INSERT INTO users (username, display_name, password_hash, is_admin) VALUES (?1, ?2, ?3, ?4)",
            params!["alice", "Alice", "scrypt$xxx", 1],
        ).unwrap();
        
        let user_id: i64 = conn.last_insert_rowid();
        assert!(user_id > 0, "User should be created");
        
        // Find by id
        let name: String = conn.query_row(
            "SELECT username FROM users WHERE id = ?1",
            params![user_id],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(name, "alice");
        
        // Find by username
        let id: i64 = conn.query_row(
            "SELECT id FROM users WHERE username = ?1",
            params!["alice"],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(id, user_id);
        
        // Update user
        conn.execute(
            "UPDATE users SET display_name = ?1, is_admin = ?2, status = ?3 WHERE id = ?4",
            params!["Alice Liddell", 0, "disabled", user_id],
        ).unwrap();
        
        // Verify update
        let (display, admin, status): (String, i32, String) = conn.query_row(
            "SELECT display_name, is_admin, status FROM users WHERE id = ?1",
            params![user_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert_eq!(display, "Alice Liddell");
        assert_eq!(admin, 0);
        assert_eq!(status, "disabled");
        
        // Count users
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM users",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
        
        // Delete user
        conn.execute("DELETE FROM users WHERE id = ?1", params![user_id]).unwrap();
        
        let count_after: i32 = conn.query_row(
            "SELECT COUNT(*) FROM users",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count_after, 0);
    }

    #[test]
    fn test_session_crud_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT,
                is_admin INTEGER DEFAULT 0
            )",
            [],
        ).unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_sessions (
                token TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                expires_at TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).unwrap();
        
        // Create user
        conn.execute(
            "INSERT INTO users (username, password_hash) VALUES (?1, ?2)",
            params!["alice", "scrypt$xxx"],
        ).unwrap();
        let user_id: i64 = conn.last_insert_rowid();
        
        // Create session
        conn.execute(
            "INSERT INTO user_sessions (token, user_id, expires_at) VALUES (?1, ?2, ?3)",
            params!["abc123token", user_id, "2025-01-01 00:00:00"],
        ).unwrap();
        
        // Find session by token
        let session_user_id: i64 = conn.query_row(
            "SELECT user_id FROM user_sessions WHERE token = ?1",
            params!["abc123token"],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(session_user_id, user_id);
        
        // Unknown token returns error
        let result: Result<i64, _> = conn.query_row(
            "SELECT user_id FROM user_sessions WHERE token = ?1",
            params!["unknown"],
            |row| row.get(0),
        );
        assert!(result.is_err());
        
        // Delete session
        conn.execute(
            "DELETE FROM user_sessions WHERE token = ?1",
            params!["abc123token"],
        ).unwrap();
        
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM user_sessions",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_api_key_crud_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL
            )",
            [],
        ).unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                key_prefix TEXT NOT NULL,
                key_hash TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                last_used_at TEXT,
                revoked_at TEXT
            )",
            [],
        ).unwrap();
        
        // Create user
        conn.execute(
            "INSERT INTO users (username) VALUES (?1)",
            params!["alice"],
        ).unwrap();
        let user_id: i64 = conn.last_insert_rowid();
        
        // Create API key
        conn.execute(
            "INSERT INTO user_api_keys (user_id, name, key_prefix, key_hash) VALUES (?1, ?2, ?3, ?4)",
            params![user_id, "laptop", "m2c_abc", "hash_xyz"],
        ).unwrap();
        
        let key_id: i64 = conn.last_insert_rowid();
        assert!(key_id > 0);
        
        // List keys for user
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM user_api_keys WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
        
        // Revoke key
        conn.execute(
            "UPDATE user_api_keys SET revoked_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![key_id],
        ).unwrap();
        
        // Verify revoked
        let revoked_at: Option<String> = conn.query_row(
            "SELECT revoked_at FROM user_api_keys WHERE id = ?1",
            params![key_id],
            |row| row.get(0),
        ).ok();
        assert!(revoked_at.is_some(), "Key should be revoked");
        
        // Delete key
        conn.execute("DELETE FROM user_api_keys WHERE id = ?1", params![key_id]).unwrap();
        
        let count_after: i32 = conn.query_row(
            "SELECT COUNT(*) FROM user_api_keys",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count_after, 0);
    }

    #[test]
    fn test_codex_history_crud_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL
            )",
            [],
        ).unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS codex_config_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER,
                provider_id TEXT,
                model_id TEXT,
                auth_backup_path TEXT,
                config_backup_path TEXT,
                preserved INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).unwrap();
        
        // Create user
        conn.execute(
            "INSERT INTO users (username) VALUES (?1)",
            params!["alice"],
        ).unwrap();
        let user_id: i64 = conn.last_insert_rowid();
        
        // Create history entry
        conn.execute(
            "INSERT INTO codex_config_history (user_id, provider_id, model_id, preserved) VALUES (?1, ?2, ?3, ?4)",
            params![user_id, "mimo", "mimo-v2.5-pro", 1],
        ).unwrap();
        
        let history_id: i64 = conn.last_insert_rowid();
        
        // Count entries for user (instead of collecting list)
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM codex_config_history WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
        
        // Verify preserved flag
        let preserved: i32 = conn.query_row(
            "SELECT preserved FROM codex_config_history WHERE id = ?1",
            params![history_id],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(preserved, 1);
    }

    #[test]
    fn test_user_count_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL
            )",
            [],
        ).unwrap();
        
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM users",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 0);
    }
}
