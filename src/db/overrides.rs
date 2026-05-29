//!
//! Override Module
//! 
//! Runtime model override management

use rusqlite::{params, Connection};

/// Active override for runtime model switching
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ActiveOverride {
    pub provider_id: String,
    pub model_id: String,
}

const KEY_PROVIDER: &str = "codex.activeOverride.providerId";
const KEY_MODEL: &str = "codex.activeOverride.modelId";

/// Get current active override from database
pub fn get_active_override(conn: &Connection) -> Option<ActiveOverride> {
    let provider_id: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![KEY_PROVIDER],
            |row| row.get(0),
        )
        .ok();
    
    let model_id: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![KEY_MODEL],
            |row| row.get(0),
        )
        .ok();
    
    match (provider_id, model_id) {
        (Some(p), Some(m)) => Some(ActiveOverride {
            provider_id: p,
            model_id: m,
        }),
        _ => None,
    }
}

/// Set active override in database
pub fn set_active_override(conn: &Connection, provider_id: &str, model_id: &str) {
    if let Err(e) = conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![KEY_PROVIDER, provider_id],
    ) {
        tracing::warn!("Failed to set override provider: {}", e);
        return;
    }
    
    if let Err(e) = conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![KEY_MODEL, model_id],
    ) {
        tracing::warn!("Failed to set override model: {}", e);
    }
}

/// Clear active override from database
pub fn clear_active_override(conn: &Connection) {
    conn.execute(
        "DELETE FROM settings WHERE key IN (?1, ?2)",
        params![KEY_PROVIDER, KEY_MODEL],
    ).ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn setup_test_db() -> (Connection, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let conn = Connection::open(&db_path).unwrap();
        
        conn.execute(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            [],
        ).unwrap();
        
        (conn, temp_dir)
    }
    
    #[test]
    fn test_get_active_override_empty() {
        let (conn, _temp) = setup_test_db();
        assert!(get_active_override(&conn).is_none());
    }
    
    #[test]
    fn test_set_and_get_active_override() {
        let (conn, _temp) = setup_test_db();
        
        set_active_override(&conn, "openai", "gpt-4");
        
        let opt = get_active_override(&conn);
        assert!(opt.is_some());
        let result = opt.unwrap();
        assert_eq!(result.provider_id, "openai");
        assert_eq!(result.model_id, "gpt-4");
    }
    
    #[test]
    fn test_clear_active_override() {
        let (conn, _temp) = setup_test_db();
        
        set_active_override(&conn, "openai", "gpt-4");
        clear_active_override(&conn);
        
        assert!(get_active_override(&conn).is_none());
    }
    
    #[test]
    fn test_override_preserves_across_replacement() {
        let (conn, _temp) = setup_test_db();
        
        set_active_override(&conn, "openai", "gpt-4");
        set_active_override(&conn, "claude", "claude-3-opus");
        
        let result = get_active_override(&conn).unwrap();
        assert_eq!(result.provider_id, "claude");
        assert_eq!(result.model_id, "claude-3-opus");
    }
    
    #[test]
    fn test_partial_override_not_returned() {
        let (conn, _temp) = setup_test_db();
        
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            params![KEY_PROVIDER, "openai"],
        ).unwrap();
        
        assert!(get_active_override(&conn).is_none());
    }
}
