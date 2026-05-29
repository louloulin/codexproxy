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

    /// Get the database connection (for external use)
    pub fn get_connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, std::sync::PoisonError<std::sync::MutexGuard<'_, Connection>>> {
        self.conn.lock()
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

    /// Get provider health stats (error rate per provider within a time window)
    pub fn get_provider_health(&self, range_ms: i64) -> Result<Vec<ProviderHealthRow>> {
        let since = chrono::Utc::now().timestamp_millis() - range_ms;
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT provider,
                    COUNT(*) as requests,
                    SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) as errors,
                    CASE WHEN COUNT(*) = 0 THEN -1.0
                         ELSE ROUND(100.0 * SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) / COUNT(*), 1)
                    END as error_rate,
                    MAX(created_at) as last_seen
             FROM requests
             WHERE created_at >= ?1
             GROUP BY provider
             ORDER BY requests DESC"
        )?;

        let rows = stmt.query_map(params![since], |row| {
            Ok(ProviderHealthRow {
                provider_id: row.get(0)?,
                requests: row.get(1)?,
                errors: row.get(2)?,
                error_rate: row.get(3)?,
                last_seen: row.get(4)?,
            })
        })?;

        let result = rows.collect::<Result<Vec<_>>>()?;
        Ok(result)
    }

    /// Get recent logs with pagination
    pub fn get_logs(&self, provider: Option<&str>, limit: i32, offset: i32) -> Result<Vec<RequestRecord>> {
        let conn = self.conn.lock().unwrap();

        let records: Vec<RequestRecord> = if let Some(p) = provider {
            let mut stmt = conn.prepare(
                "SELECT id, provider, model, endpoint, status_code, tokens_used, latency_ms, error_message
                 FROM requests WHERE provider = ?1 ORDER BY created_at DESC LIMIT ?2 OFFSET ?3"
            )?;
            let rows = stmt.query_map(params![p, limit, offset], |row| {
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
            })?;
            rows.collect::<Result<Vec<_>>>()?
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, provider, model, endpoint, status_code, tokens_used, latency_ms, error_message
                 FROM requests ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"
            )?;
            let rows = stmt.query_map(params![limit, offset], |row| {
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
            })?;
            rows.collect::<Result<Vec<_>>>()?
        };

        Ok(records)
    }

    /// Delete logs older than a timestamp
    pub fn delete_logs_before(&self, before_ts: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM requests WHERE created_at < ?1",
            params![before_ts],
        )?;
        Ok(affected)
    }

    /// Aggregate error stats
    pub fn aggregate_errors(&self, range: &str) -> Result<Vec<ErrorBucket>> {
        let conn = self.conn.lock().unwrap();
        let interval = range_to_seconds(range);
        let mut stmt = conn.prepare(
            "SELECT strftime('%Y-%m-%d %H:00:00', created_at) as bucket,
                    COUNT(*) as total,
                    SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) as errors
             FROM requests
             WHERE created_at >= datetime('now', ?1)
             GROUP BY bucket ORDER BY bucket DESC"
        )?;
        let rows = stmt.query_map(params![interval], |row| {
            Ok(ErrorBucket {
                bucket: row.get(0)?,
                total: row.get(1)?,
                errors: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>>>()
    }

    /// Aggregate latency stats
    pub fn aggregate_latency(&self, range: &str) -> Result<LatencyStats> {
        let conn = self.conn.lock().unwrap();
        let interval = range_to_seconds(range);
        let mut stmt = conn.prepare(
            "SELECT COALESCE(AVG(latency_ms), 0) as avg_ms,
                    COALESCE(MIN(latency_ms), 0) as min_ms,
                    COALESCE(MAX(latency_ms), 0) as max_ms,
                    COUNT(*) as count
             FROM requests
             WHERE created_at >= datetime('now', ?1)"
        )?;
        stmt.query_row(params![interval], |row| {
            Ok(LatencyStats {
                avg_ms: row.get(0)?,
                min_ms: row.get(1)?,
                max_ms: row.get(2)?,
                count: row.get(3)?,
            })
        })
    }

    /// Aggregate token timeseries
    pub fn aggregate_token_timeseries(&self, range: &str) -> Result<Vec<TokenBucket>> {
        let conn = self.conn.lock().unwrap();
        let interval = range_to_seconds(range);
        let mut stmt = conn.prepare(
            "SELECT strftime('%Y-%m-%d', created_at) as day,
                    COALESCE(SUM(tokens_used), 0) as total_tokens,
                    COUNT(*) as requests
             FROM requests
             WHERE created_at >= datetime('now', ?1)
             GROUP BY day ORDER BY day ASC"
        )?;
        let rows = stmt.query_map(params![interval], |row| {
            Ok(TokenBucket {
                day: row.get(0)?,
                total_tokens: row.get(1)?,
                requests: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>>>()
    }

    /// Aggregate per-model stats
    pub fn aggregate_per_model(&self, range: &str) -> Result<Vec<ModelStats>> {
        let conn = self.conn.lock().unwrap();
        let interval = range_to_seconds(range);
        let mut stmt = conn.prepare(
            "SELECT model, COUNT(*) as requests,
                    COALESCE(SUM(tokens_used), 0) as total_tokens,
                    COALESCE(AVG(latency_ms), 0) as avg_latency,
                    SUM(CASE WHEN status_code >= 400 THEN 1 ELSE 0 END) as errors
             FROM requests
             WHERE created_at >= datetime('now', ?1)
             GROUP BY model ORDER BY requests DESC"
        )?;
        let rows = stmt.query_map(params![interval], |row| {
            Ok(ModelStats {
                model: row.get(0)?,
                requests: row.get(1)?,
                total_tokens: row.get(2)?,
                avg_latency: row.get(3)?,
                errors: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>>>()
    }
}

fn range_to_seconds(range: &str) -> String {
    match range {
        "1h" => "-1 hours".to_string(),
        "6h" => "-6 hours".to_string(),
        "24h" => "-24 hours".to_string(),
        "7d" => "-7 days".to_string(),
        "30d" => "-30 days".to_string(),
        _ => "-24 hours".to_string(),
    }
}

/// Error bucket for aggregate stats
#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorBucket {
    pub bucket: String,
    pub total: i64,
    pub errors: i64,
}

/// Latency statistics
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyStats {
    pub avg_ms: f64,
    pub min_ms: i64,
    pub max_ms: i64,
    pub count: i64,
}

/// Token bucket for timeseries
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBucket {
    pub day: String,
    pub total_tokens: i64,
    pub requests: i64,
}

/// Per-model statistics
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStats {
    pub model: String,
    pub requests: i64,
    pub total_tokens: i64,
    pub avg_latency: f64,
    pub errors: i64,
}

/// Provider health statistics row
#[derive(Debug, Clone)]
pub struct ProviderHealthRow {
    pub provider_id: String,
    pub requests: i64,
    pub errors: i64,
    pub error_rate: f64,
    pub last_seen: Option<String>,
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
