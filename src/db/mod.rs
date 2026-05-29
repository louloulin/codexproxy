//!
//! Database module for rcodex
//! 
//! This module provides SQLite database operations for:
//! - Request logging and tracking
//! - Session management
//! - Provider statistics
//! - Settings (key-value store)
//! - Override management

pub mod schema;
pub mod repository;
pub mod overrides;
pub mod user_repository;

pub use schema::{DbPool, init_database};
pub use repository::RequestRepository;
pub use overrides::{ActiveOverride, get_active_override, set_active_override, clear_active_override};
pub use user_repository::{UserRepository, UserRecord, ApiKeyRecord, ApiKeyInfo, SessionRecord};
pub mod codex_history;
pub use codex_history::{CodexHistoryStore, CodexHistoryEntry, HistoryKind};
