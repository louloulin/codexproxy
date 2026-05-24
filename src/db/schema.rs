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
