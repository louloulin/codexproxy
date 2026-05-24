//!
//! Codex History Module
//! 
//! Rust implementation aligned with mimo2codex db.codexHistory.test.ts
//! 
//! Features:
//! - Append codex history entries
//! - List history with retention policy
//! - Check for initial history
//! - History kind tracking (initial, apply, restore)

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// History entry kind
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryKind {
    /// Initial configuration snapshot
    Initial,
    /// Apply codex operation
    Apply,
    /// Restore from backup
    Restore,
}

impl std::fmt::Display for HistoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoryKind::Initial => write!(f, "initial"),
            HistoryKind::Apply => write!(f, "apply"),
            HistoryKind::Restore => write!(f, "restore"),
        }
    }
}

/// Codex history entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexHistoryEntry {
    pub id: i64,
    pub user_id: Option<i64>,
    pub kind: HistoryKind,
    pub auth_json: String,
    pub config_toml: String,
    pub note: Option<String>,
    pub created_at: i64,
}

/// In-memory codex history store (simulates DB for testing)
pub struct CodexHistoryStore {
    entries: Vec<CodexHistoryEntry>,
    next_id: i64,
}

impl Default for CodexHistoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexHistoryStore {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    /// Append a new history entry
    pub fn append(&mut self, user_id: Option<i64>, kind: HistoryKind, auth_json: String, config_toml: String, note: Option<String>) -> CodexHistoryEntry {
        let entry = CodexHistoryEntry {
            id: self.next_id,
            user_id,
            kind,
            auth_json,
            config_toml,
            note,
            created_at: chrono::Utc::now().timestamp_millis(),
        };
        self.next_id += 1;
        self.entries.push(entry.clone());
        entry
    }

    /// Check if user has initial history
    pub fn has_initial(&self, user_id: Option<i64>) -> bool {
        self.entries.iter().any(|e| {
            e.user_id == user_id && e.kind == HistoryKind::Initial
        })
    }

    /// List history for a user with retention policy
    /// Keeps: 1 initial + last 10 non-initial entries
    /// Follows mimo2codex logic: ORDER BY ts DESC, id DESC (newest first)
    pub fn list(&self, user_id: Option<i64>, _limit: usize) -> VecDeque<CodexHistoryEntry> {
        // Filter by user_id
        let mut user_entries: Vec<_> = self.entries.iter()
            .filter(|e| e.user_id == user_id)
            .cloned()
            .collect();
        
        // Sort by created_at DESC, id DESC (mimics SQL ORDER BY ts DESC, id DESC)
        // This ensures newest entries (highest id with same or later timestamp) come first
        user_entries.sort_by(|a, b| {
            // Primary: created_at DESC (newest first)
            match b.created_at.cmp(&a.created_at) {
                std::cmp::Ordering::Equal => {
                    // Secondary: id DESC (newest ID first within same timestamp)
                    b.id.cmp(&a.id)
                }
                ord => ord,
            }
        });
        
        // Find initial entry (we need the oldest one, so search from the end)
        let initial_entry = user_entries.iter().rev().find(|e| e.kind == HistoryKind::Initial).cloned();
        
        // Get non-initial entries (newest first) - take only 10
        let non_initial: Vec<_> = user_entries.iter()
            .filter(|e| e.kind != HistoryKind::Initial)
            .take(10)
            .cloned()
            .collect();
        
        // Combine: initial (oldest) + last 10 non-initial (newest)
        let mut result: VecDeque<CodexHistoryEntry> = non_initial.into_iter().collect();
        if let Some(initial) = initial_entry {
            result.push_front(initial);
        }
        
        result
    }

    /// Get entry by ID
    pub fn get_by_id(&self, id: i64) -> Option<CodexHistoryEntry> {
        self.entries.iter().find(|e| e.id == id).cloned()
    }

    /// Delete entry by ID
    pub fn delete(&mut self, id: i64) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_id = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_payload(seq: i64) -> (String, String, Option<String>) {
        let auth = format!(r#"{{"OPENAI_API_KEY": "tok-{}"}}"#, seq);
        let config = format!(r#"# config {}\n[model_providers.mimo]"#, seq);
        (auth, config, None)
    }

    #[test]
    fn test_appends_initial_snapshot_and_detects_it() {
        let mut store = CodexHistoryStore::new();
        let (auth, config, _) = make_payload(0);
        
        assert!(!store.has_initial(Some(1)));
        
        store.append(Some(1), HistoryKind::Initial, auth.clone(), config.clone(), None);
        
        assert!(store.has_initial(Some(1)));
    }

    #[test]
    fn test_retention_keeps_earliest_initial_plus_last_10_non_initial() {
        let mut store = CodexHistoryStore::new();
        let (auth, config, _) = make_payload(0);
        
        store.append(Some(1), HistoryKind::Initial, auth.clone(), config.clone(), None);
        
        // 15 apply entries
        for i in 1..=15 {
            let (auth, config, _) = make_payload(i);
            let note = format!("apply {}", i);
            store.append(Some(1), HistoryKind::Apply, auth, config, Some(note));
        }
        
        let rows = store.list(Some(1), 100);
        // 1 initial + 10 most recent non-initial = 11 total
        assert_eq!(rows.len(), 11);
        assert_eq!(rows.front().map(|r| r.kind.clone()), Some(HistoryKind::Initial));
        
        let notes: Vec<_> = rows.iter()
            .filter(|r| r.kind == HistoryKind::Apply)
            .filter_map(|r| r.note.clone())
            .collect();
        
        // Oldest 5 should be dropped (applies 1-5)
        assert!(!notes.contains(&"apply 1".to_string()));
        assert!(!notes.contains(&"apply 2".to_string()));
        assert!(!notes.contains(&"apply 3".to_string()));
        assert!(!notes.contains(&"apply 4".to_string()));
        assert!(!notes.contains(&"apply 5".to_string()));
        
        // Newest should be kept
        assert!(notes.contains(&"apply 6".to_string()));
        assert!(notes.contains(&"apply 11".to_string()));
        assert!(notes.contains(&"apply 15".to_string()));
    }

    #[test]
    fn test_restore_entries_count_toward_retention() {
        let mut store = CodexHistoryStore::new();
        let (auth0, config0, _) = make_payload(0);
        
        store.append(Some(1), HistoryKind::Initial, auth0, config0, None);
        
        for i in 1..=9 {
            let (auth, config, _) = make_payload(i);
            store.append(Some(1), HistoryKind::Apply, auth, config, None);
        }
        
        let (auth, config, _) = make_payload(99);
        store.append(Some(1), HistoryKind::Restore, auth, config, Some("rollback".to_string()));
        
        let (auth, config, _) = make_payload(100);
        store.append(Some(1), HistoryKind::Apply, auth, config, None);
        
        let rows = store.list(Some(1), 100);
        // 1 initial + 10 most recent non-initial = 11 total
        assert_eq!(rows.len(), 11);
        
        // Restore entry should be present
        assert!(rows.iter().any(|r| r.kind == HistoryKind::Restore));
    }

    #[test]
    fn test_local_mode_has_own_timeline() {
        let mut store = CodexHistoryStore::new();
        let (auth0, config0, _) = make_payload(0);
        
        // Null user (local mode)
        store.append(None, HistoryKind::Initial, auth0.clone(), config0.clone(), None);
        
        let (auth1, config1, _) = make_payload(1);
        store.append(None, HistoryKind::Apply, auth1, config1, None);
        
        // User 2 has separate timeline
        let (auth2, config2, _) = make_payload(2);
        store.append(Some(2), HistoryKind::Initial, auth2, config2, None);
        
        // Local mode has 2 entries
        let local_rows = store.list(None, 100);
        assert_eq!(local_rows.len(), 2);
        
        // User 2 has 1 entry
        let user_rows = store.list(Some(2), 100);
        assert_eq!(user_rows.len(), 1);
        
        // User 1 has no entries
        let user1_rows = store.list(Some(1), 100);
        assert_eq!(user1_rows.len(), 0);
    }

    #[test]
    fn test_get_by_id() {
        let mut store = CodexHistoryStore::new();
        let (auth, config, _) = make_payload(0);
        
        let entry = store.append(Some(1), HistoryKind::Initial, auth, config, None);
        
        let found = store.get_by_id(entry.id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, entry.id);
        
        let not_found = store.get_by_id(999);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_delete() {
        let mut store = CodexHistoryStore::new();
        let (auth, config, _) = make_payload(0);
        
        let entry = store.append(Some(1), HistoryKind::Initial, auth, config, None);
        
        assert!(store.has_initial(Some(1)));
        
        let deleted = store.delete(entry.id);
        assert!(deleted);
        assert!(!store.has_initial(Some(1)));
        
        let not_deleted = store.delete(entry.id);
        assert!(!not_deleted);
    }
}
