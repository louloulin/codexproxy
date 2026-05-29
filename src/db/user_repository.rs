//! User repository for database operations
//!
//! Provides CRUD operations for users, sessions, and API keys.

use rusqlite::{params, Connection, Result};
use std::sync::Mutex;

/// User record
#[derive(Debug, Clone)]
pub struct UserRecord {
    pub id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub password_hash: Option<String>,
    pub is_admin: bool,
    pub status: String,
}

/// API key record
#[derive(Debug, Clone)]
pub struct ApiKeyRecord {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub scopes: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
}

/// API key info for display (no hash)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiKeyInfo {
    pub id: i64,
    pub name: String,
    pub key_prefix: String,
    pub scopes: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub is_revoked: bool,
}

/// Session record
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub token: String,
    pub user_id: i64,
    pub expires_at: String,
}

/// User with usage statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct UserWithStats {
    pub id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub request_count: i64,
    pub total_tokens: i64,
    pub last_activity: Option<String>,
}

/// Upstream key info for display (no ciphertext)
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamKeyInfo {
    pub id: i64,
    pub provider_id: String,
    pub key_prefix: String,
    pub created_at: String,
    pub updated_at: String,
}

/// OAuth client record
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthClient {
    pub provider: String,
    pub client_id: String,
    pub callback_url: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Custom model record
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModel {
    pub id: i64,
    pub provider_id: String,
    pub upstream_id: String,
    pub display_name: Option<String>,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
    pub is_builtin: bool,
    pub created_at: String,
}

/// User repository
pub struct UserRepository {
    conn: Mutex<Connection>,
}

impl UserRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    // --- User operations ---

    /// Create a new user
    pub fn create_user(
        &self,
        username: &str,
        password_hash: &str,
        display_name: Option<&str>,
        email: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO users (username, password_hash, display_name, email) VALUES (?1, ?2, ?3, ?4)",
            params![username, password_hash, display_name, email],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Find user by username
    pub fn find_by_username(&self, username: &str) -> Result<Option<UserRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, username, display_name, email, avatar_url, password_hash, is_admin, status FROM users WHERE username = ?1",
            params![username],
            |row| {
                Ok(UserRecord {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    email: row.get(3)?,
                    avatar_url: row.get(4)?,
                    password_hash: row.get(5)?,
                    is_admin: row.get::<_, i32>(6)? != 0,
                    status: row.get(7)?,
                })
            },
        ).optional()
    }

    /// Find user by ID
    pub fn find_by_id(&self, id: i64) -> Result<Option<UserRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, username, display_name, email, avatar_url, password_hash, is_admin, status FROM users WHERE id = ?1",
            params![id],
            |row| {
                Ok(UserRecord {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    email: row.get(3)?,
                    avatar_url: row.get(4)?,
                    password_hash: row.get(5)?,
                    is_admin: row.get::<_, i32>(6)? != 0,
                    status: row.get(7)?,
                })
            },
        ).optional()
    }

    /// List all users
    pub fn list_users(&self) -> Result<Vec<UserRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, username, display_name, email, avatar_url, password_hash, is_admin, status FROM users ORDER BY id",
        )?;
        let users = stmt.query_map([], |row| {
            Ok(UserRecord {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                email: row.get(3)?,
                avatar_url: row.get(4)?,
                password_hash: row.get(5)?,
                is_admin: row.get::<_, i32>(6)? != 0,
                status: row.get(7)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        Ok(users)
    }

    /// Update user status
    pub fn update_user_status(&self, id: i64, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE users SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    /// Update user profile
    pub fn update_user_profile(
        &self,
        id: i64,
        display_name: Option<&str>,
        email: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE users SET display_name = COALESCE(?1, display_name), email = COALESCE(?2, email), avatar_url = COALESCE(?3, avatar_url), updated_at = CURRENT_TIMESTAMP WHERE id = ?4",
            params![display_name, email, avatar_url, id],
        )?;
        Ok(())
    }

    /// Count users
    pub fn count_users(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
    }

    /// Update user (full update)
    pub fn update_user(
        &self,
        id: i64,
        display_name: Option<&str>,
        email: Option<&str>,
        is_admin: Option<bool>,
        status: Option<&str>,
        password_hash: Option<&str>,
    ) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let mut updates = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(dn) = display_name {
            updates.push("display_name = ?");
            params_vec.push(Box::new(dn.to_string()));
        }
        if let Some(e) = email {
            updates.push("email = ?");
            params_vec.push(Box::new(e.to_string()));
        }
        if let Some(ia) = is_admin {
            updates.push("is_admin = ?");
            params_vec.push(Box::new(if ia { 1 } else { 0 }));
        }
        if let Some(s) = status {
            updates.push("status = ?");
            params_vec.push(Box::new(s.to_string()));
        }
        if let Some(ph) = password_hash {
            updates.push("password_hash = ?");
            params_vec.push(Box::new(ph.to_string()));
        }

        if updates.is_empty() {
            return Ok(false);
        }

        updates.push("updated_at = CURRENT_TIMESTAMP");
        params_vec.push(Box::new(id));

        let sql = format!(
            "UPDATE users SET {} WHERE id = ?",
            updates.join(", ")
        );

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let affected = conn.execute(&sql, params_refs.as_slice())?;
        Ok(affected > 0)
    }

    /// Delete user
    pub fn delete_user(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM users WHERE id = ?", params![id])?;
        Ok(affected > 0)
    }

    /// Get user with usage stats
    pub fn get_user_with_stats(&self, id: i64) -> Result<Option<UserWithStats>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, username, display_name, email, avatar_url, is_admin, status,
                    created_at, updated_at
             FROM users WHERE id = ?",
            params![id],
            |row| {
                Ok(UserWithStats {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    email: row.get(3)?,
                    avatar_url: row.get(4)?,
                    is_admin: row.get::<_, i32>(5)? != 0,
                    status: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                    request_count: 0,
                    total_tokens: 0,
                    last_activity: None,
                })
            },
        ).optional()
    }

    /// List users with usage stats
    pub fn list_users_with_stats(&self) -> Result<Vec<UserWithStats>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, username, display_name, email, avatar_url, is_admin, status,
                    created_at, updated_at
             FROM users ORDER BY id"
        )?;
        let users = stmt.query_map([], |row| {
            Ok(UserWithStats {
                id: row.get(0)?,
                username: row.get(1)?,
                display_name: row.get(2)?,
                email: row.get(3)?,
                avatar_url: row.get(4)?,
                is_admin: row.get::<_, i32>(5)? != 0,
                status: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                request_count: 0,
                total_tokens: 0,
                last_activity: None,
            })
        })?.collect::<Result<Vec<_>>>()?;
        Ok(users)
    }

    // --- Session operations ---

    /// Create a new session
    pub fn create_session(&self, token: &str, user_id: i64, expires_at: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO user_sessions (token, user_id, expires_at) VALUES (?1, ?2, ?3)",
            params![token, user_id, expires_at],
        )?;
        Ok(())
    }

    /// Find session by token
    pub fn find_session(&self, token: &str) -> Result<Option<SessionRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT token, user_id, expires_at FROM user_sessions WHERE token = ?1",
            params![token],
            |row| {
                Ok(SessionRecord {
                    token: row.get(0)?,
                    user_id: row.get(1)?,
                    expires_at: row.get(2)?,
                })
            },
        ).optional()
    }

    /// Delete session
    pub fn delete_session(&self, token: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM user_sessions WHERE token = ?1",
            params![token],
        )?;
        Ok(())
    }

    /// Clean expired sessions
    pub fn clean_expired_sessions(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM user_sessions WHERE expires_at < datetime('now')",
            [],
        )
    }

    // --- API key operations ---

    /// Create API key
    pub fn create_api_key(
        &self,
        user_id: i64,
        name: &str,
        key_prefix: &str,
        key_hash: &str,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO user_api_keys (user_id, name, key_prefix, key_hash) VALUES (?1, ?2, ?3, ?4)",
            params![user_id, name, key_prefix, key_hash],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// List API keys for user
    pub fn list_api_keys(&self, user_id: i64) -> Result<Vec<ApiKeyInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, key_prefix, scopes, created_at, last_used_at, revoked_at FROM user_api_keys WHERE user_id = ?1 ORDER BY id",
        )?;
        let keys = stmt.query_map(params![user_id], |row| {
            Ok(ApiKeyInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                key_prefix: row.get(2)?,
                scopes: row.get(3)?,
                created_at: row.get(4)?,
                last_used_at: row.get(5)?,
                is_revoked: row.get::<_, Option<String>>(6)?.is_some(),
            })
        })?.collect::<Result<Vec<_>>>()?;
        Ok(keys)
    }

    /// Find API key by hash
    pub fn find_api_key_by_hash(&self, key_hash: &str) -> Result<Option<ApiKeyRecord>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, user_id, name, key_prefix, key_hash, scopes, created_at, last_used_at, revoked_at FROM user_api_keys WHERE key_hash = ?1",
            params![key_hash],
            |row| {
                Ok(ApiKeyRecord {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    key_prefix: row.get(3)?,
                    key_hash: row.get(4)?,
                    scopes: row.get(5)?,
                    created_at: row.get(6)?,
                    last_used_at: row.get(7)?,
                    revoked_at: row.get(8)?,
                })
            },
        ).optional()
    }

    /// Revoke API key
    pub fn revoke_api_key(&self, id: i64, user_id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "UPDATE user_api_keys SET revoked_at = CURRENT_TIMESTAMP WHERE id = ?1 AND user_id = ?2 AND revoked_at IS NULL",
            params![id, user_id],
        )?;
        Ok(affected > 0)
    }

    /// Delete API key
    pub fn delete_api_key(&self, id: i64, user_id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM user_api_keys WHERE id = ?1 AND user_id = ?2",
            params![id, user_id],
        )?;
        Ok(affected > 0)
    }

    /// Update API key last used time
    pub fn touch_api_key(&self, key_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE user_api_keys SET last_used_at = CURRENT_TIMESTAMP WHERE key_hash = ?1 AND revoked_at IS NULL",
            params![key_hash],
        )?;
        Ok(())
    }

    // --- Upstream Key operations ---

    /// List upstream keys for a user
    pub fn list_upstream_keys(&self, user_id: i64) -> Result<Vec<UpstreamKeyInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, provider_id, key_prefix, created_at, updated_at FROM upstream_keys WHERE user_id = ?1 ORDER BY provider_id"
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(UpstreamKeyInfo {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                key_prefix: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>>>()
    }

    /// Set or update an upstream key for a user
    pub fn set_upstream_key(&self, user_id: i64, provider_id: &str, encrypted_key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let key_prefix = &encrypted_key.chars().take(8).collect::<String>();
        conn.execute(
            "INSERT INTO upstream_keys (user_id, provider_id, encrypted_key, key_prefix, updated_at)
             VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)
             ON CONFLICT(user_id, provider_id) DO UPDATE SET
                encrypted_key = excluded.encrypted_key,
                key_prefix = excluded.key_prefix,
                updated_at = CURRENT_TIMESTAMP",
            params![user_id, provider_id, encrypted_key, key_prefix],
        )?;
        Ok(())
    }

    /// Delete an upstream key
    pub fn delete_upstream_key(&self, user_id: i64, provider_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM upstream_keys WHERE user_id = ?1 AND provider_id = ?2",
            params![user_id, provider_id],
        )?;
        Ok(affected > 0)
    }

    /// Get decrypted upstream key (returns encrypted value - decryption is caller's concern)
    pub fn get_upstream_key(&self, user_id: i64, provider_id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT encrypted_key FROM upstream_keys WHERE user_id = ?1 AND provider_id = ?2",
            params![user_id, provider_id],
            |row| row.get(0),
        ).optional()
    }

    // --- Custom Model operations ---

    /// List custom models for a provider
    pub fn list_custom_models(&self, provider_id: &str) -> Result<Vec<CustomModel>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, provider_id, upstream_id, display_name, context_window, max_output_tokens, is_builtin, created_at
             FROM custom_models WHERE provider_id = ?1 ORDER BY upstream_id"
        )?;
        let rows = stmt.query_map(params![provider_id], |row| {
            Ok(CustomModel {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                upstream_id: row.get(2)?,
                display_name: row.get(3)?,
                context_window: row.get(4)?,
                max_output_tokens: row.get(5)?,
                is_builtin: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>>>()
    }

    /// Insert a custom model
    pub fn insert_custom_model(
        &self,
        provider_id: &str,
        upstream_id: &str,
        display_name: Option<&str>,
        context_window: Option<i64>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO custom_models (provider_id, upstream_id, display_name, context_window)
             VALUES (?1, ?2, ?3, ?4)",
            params![provider_id, upstream_id, display_name, context_window],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Update a custom model
    pub fn update_custom_model(
        &self,
        id: i64,
        display_name: Option<&str>,
        context_window: Option<i64>,
    ) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "UPDATE custom_models SET display_name = COALESCE(?1, display_name),
                 context_window = COALESCE(?2, context_window)
             WHERE id = ?3",
            params![display_name, context_window, id],
        )?;
        Ok(affected > 0)
    }

    /// Delete a custom model
    pub fn delete_custom_model(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM custom_models WHERE id = ?1 AND is_builtin = 0",
            params![id],
        )?;
        Ok(affected > 0)
    }

    // --- OAuth Client operations ---

    /// List all OAuth clients
    pub fn list_oauth_clients(&self) -> Result<Vec<OAuthClient>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT provider, client_id, callback_url, enabled, created_at, updated_at FROM oauth_clients ORDER BY provider"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(OAuthClient {
                provider: row.get(0)?,
                client_id: row.get(1)?,
                callback_url: row.get(2)?,
                enabled: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>>>()
    }

    /// Upsert (create or update) an OAuth client
    pub fn upsert_oauth_client(
        &self,
        provider: &str,
        client_id: &str,
        client_secret: Option<&str>,
        callback_url: &str,
        enabled: bool,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let enabled_int = if enabled { 1 } else { 0 };
        conn.execute(
            "INSERT INTO oauth_clients (provider, client_id, client_secret, callback_url, enabled, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)
             ON CONFLICT(provider) DO UPDATE SET
                client_id = excluded.client_id,
                client_secret = CASE WHEN excluded.client_secret IS NOT NULL THEN excluded.client_secret ELSE oauth_clients.client_secret END,
                callback_url = excluded.callback_url,
                enabled = excluded.enabled,
                updated_at = CURRENT_TIMESTAMP",
            params![provider, client_id, client_secret, callback_url, enabled_int],
        )?;
        Ok(())
    }

    /// Delete an OAuth client
    pub fn delete_oauth_client(&self, provider: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute(
            "DELETE FROM oauth_clients WHERE provider = ?1",
            params![provider],
        )?;
        Ok(affected > 0)
    }

    // --- Settings operations ---

    /// Get setting value
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        ).optional()
    }

    /// Set setting value
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Delete setting
    pub fn delete_setting(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(())
    }
}

/// Extension trait for Optional query results
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
    use crate::db::schema::init_database;
    use tempfile::NamedTempFile;

    fn setup_repo() -> (NamedTempFile, UserRepository) {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = init_database(temp_file.path()).unwrap();
        let repo = UserRepository::new(conn);
        (temp_file, repo)
    }

    #[test]
    fn test_create_and_find_user() {
        let (_tmp, repo) = setup_repo();
        let id = repo.create_user("alice", "hash123", Some("Alice"), Some("alice@test.com")).unwrap();
        assert!(id > 0);

        let user = repo.find_by_username("alice").unwrap().unwrap();
        assert_eq!(user.username, "alice");
        assert_eq!(user.display_name, Some("Alice".to_string()));
        assert_eq!(user.email, Some("alice@test.com".to_string()));
    }

    #[test]
    fn test_find_nonexistent_user() {
        let (_tmp, repo) = setup_repo();
        let user = repo.find_by_username("nobody").unwrap();
        assert!(user.is_none());
    }

    #[test]
    fn test_list_users() {
        let (_tmp, repo) = setup_repo();
        repo.create_user("alice", "h1", None, None).unwrap();
        repo.create_user("bob", "h2", None, None).unwrap();

        let users = repo.list_users().unwrap();
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn test_update_user_status() {
        let (_tmp, repo) = setup_repo();
        let id = repo.create_user("alice", "h1", None, None).unwrap();
        repo.update_user_status(id, "disabled").unwrap();

        let user = repo.find_by_id(id).unwrap().unwrap();
        assert_eq!(user.status, "disabled");
    }

    #[test]
    fn test_session_crud() {
        let (_tmp, repo) = setup_repo();
        let uid = repo.create_user("alice", "h1", None, None).unwrap();
        repo.create_session("token123", uid, "2099-01-01 00:00:00").unwrap();

        let session = repo.find_session("token123").unwrap().unwrap();
        assert_eq!(session.user_id, uid);

        repo.delete_session("token123").unwrap();
        assert!(repo.find_session("token123").unwrap().is_none());
    }

    #[test]
    fn test_api_key_crud() {
        let (_tmp, repo) = setup_repo();
        let uid = repo.create_user("alice", "h1", None, None).unwrap();

        let key_id = repo.create_api_key(uid, "laptop", "m2c_abc", "hash_xyz").unwrap();
        assert!(key_id > 0);

        let keys = repo.list_api_keys(uid).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "laptop");
        assert!(!keys[0].is_revoked);

        // Revoke
        repo.revoke_api_key(key_id, uid).unwrap();
        let keys = repo.list_api_keys(uid).unwrap();
        assert!(keys[0].is_revoked);

        // Delete
        repo.delete_api_key(key_id, uid).unwrap();
        let keys = repo.list_api_keys(uid).unwrap();
        assert_eq!(keys.len(), 0);
    }

    #[test]
    fn test_settings() {
        let (_tmp, repo) = setup_repo();
        repo.set_setting("test.key", "value1").unwrap();

        let val = repo.get_setting("test.key").unwrap().unwrap();
        assert_eq!(val, "value1");

        repo.set_setting("test.key", "value2").unwrap();
        let val = repo.get_setting("test.key").unwrap().unwrap();
        assert_eq!(val, "value2");

        repo.delete_setting("test.key").unwrap();
        assert!(repo.get_setting("test.key").unwrap().is_none());
    }
}
