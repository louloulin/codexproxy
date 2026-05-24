//! Request repository for database operations
//! 
//! Provides CRUD operations for requests, sessions, and statistics.

use rusqlite::{params, Connection, Result};
use std::sync::Mutex;

/// Request record for database storage
#[derive(Debug, Clone)]
pub struct RequestRecord {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub endpoint: String,
    pub status_code: Option<i32>,
    pub tokens_used: Option<i32>,
    pub latency_ms: Option<i32>,
    pub error_message: Option<String>,
}

/// Session record for database storage
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub request_count: i32,
    pub total_tokens: i64,
    pub created_at: String,
    pub last_activity_at: String,
}

/// Provider statistics
#[derive(Debug, Clone)]
pub struct ProviderStats {
    pub provider: String,
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub total_tokens: i64,
    pub avg_latency_ms: f64,
}

/// Repository for request operations
pub struct RequestRepository {
    conn: Mutex<Connection>,
}

impl RequestRepository {
    /// Create a new repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }
    
    /// Log a new request
    pub fn log_request(&self, record: &RequestRecord) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO requests (id, provider, model, endpoint, status_code, tokens_used, latency_ms, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.provider,
                record.model,
                record.endpoint,
                record.status_code,
                record.tokens_used,
                record.latency_ms,
                record.error_message,
            ],
        )?;
        Ok(())
    }
    
    /// Update request with response info
    pub fn update_request(&self, id: &str, status_code: i32, tokens_used: i32, latency_ms: i32) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE requests SET status_code = ?1, tokens_used = ?2, latency_ms = ?3 WHERE id = ?4",
            params![status_code, tokens_used, latency_ms, id],
        )?;
        Ok(())
    }
    
    /// Update request with error
    pub fn log_error(&self, id: &str, error_message: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE requests SET error_message = ?1 WHERE id = ?2",
            params![error_message, id],
        )?;
        Ok(())
    }
    
    /// Get provider statistics
    pub fn get_provider_stats(&self, provider: &str) -> Result<Option<ProviderStats>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT provider, total_requests, successful_requests, failed_requests, 
                    total_tokens, avg_latency_ms 
             FROM provider_stats WHERE provider = ?1"
        )?;
        
        let stats = stmt.query_row(params![provider], |row| {
            Ok(ProviderStats {
                provider: row.get(0)?,
                total_requests: row.get(1)?,
                successful_requests: row.get(2)?,
                failed_requests: row.get(3)?,
                total_tokens: row.get(4)?,
                avg_latency_ms: row.get(5)?,
            })
        }).optional()?;
        
        Ok(stats)
    }
    
    /// Update provider statistics
    pub fn update_provider_stats(&self, provider: &str, latency_ms: i32, success: bool, tokens: i32) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        // Use INSERT OR REPLACE to handle both insert and update
        conn.execute(
            "INSERT INTO provider_stats (provider, total_requests, successful_requests, failed_requests, total_tokens, avg_latency_ms, last_updated)
             VALUES (
                 ?1,
                 1,
                 ?2,
                 ?3,
                 ?4,
                 ?5,
                 CURRENT_TIMESTAMP
             )
             ON CONFLICT(provider) DO UPDATE SET
                 total_requests = total_requests + 1,
                 successful_requests = successful_requests + ?2,
                 failed_requests = failed_requests + ?3,
                 total_tokens = total_tokens + ?4,
                 avg_latency_ms = (avg_latency_ms * (total_requests - 1) + ?5) / total_requests,
                 last_updated = CURRENT_TIMESTAMP",
            params![
                provider,
                if success { 1 } else { 0 },
                if success { 0 } else { 1 },
                tokens,
                latency_ms,
            ],
        )?;
        
        Ok(())
    }
    
    /// Get recent requests for a provider
    pub fn get_recent_requests(&self, provider: &str, limit: i32) -> Result<Vec<RequestRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, provider, model, endpoint, status_code, tokens_used, latency_ms, error_message
             FROM requests 
             WHERE provider = ?1
             ORDER BY created_at DESC
             LIMIT ?2"
        )?;
        
        let records = stmt.query_map(params![provider, limit], |row| {
            Ok(RequestRecord {
                id: row.get(0)?,
                provider: row.get(1)?,
                model: row.get(2)?,
                endpoint: row.get(3)?,
                status_code: row.get(4)?,
                tokens_used: row.get(5)?,
                latency_ms: row.get(6)?,
                error_message: row.get(7)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        
        Ok(records)
    }
}

// Extension trait to make rusqlite query_row return Option
trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>>;
}

impl<T> OptionalExt<T> for Result<T> {
    fn optional(self) -> Result<Option<T>> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_log_request() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(temp_file.path()).unwrap();
        
        // Initialize schema
        conn.execute(
            "CREATE TABLE requests (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                endpoint TEXT NOT NULL,
                status_code INTEGER,
                tokens_used INTEGER,
                latency_ms INTEGER,
                error_message TEXT
            )",
            [],
        ).unwrap();
        
        let repo = RequestRepository::new(conn);
        let record = RequestRecord {
            id: "req_123".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            endpoint: "/v1/chat/completions".to_string(),
            status_code: None,
            tokens_used: None,
            latency_ms: None,
            error_message: None,
        };
        
        repo.log_request(&record).unwrap();
    }
}
