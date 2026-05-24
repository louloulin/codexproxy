//!
//! Version Check and Update Utilities
//! 
//! Rust implementation aligned with mimo2codex util/checkUpdate.ts
//! 
//! Features:
//! - compareVersions: Semver comparison
//! - is_prerelease: Check if version is pre-release
//! - fetch_latest_version: Fetch from npm registry
//! - Cache read/write with TTL

use serde::{Deserialize, Serialize};

/// Compare two semver-ish strings. Returns -1 / 0 / 1.
/// Tolerates pre-release suffixes ("0.3.0-beta.1"): pre-release sorts BEFORE the
/// matching stable, per semver spec.
pub fn compare_versions(a: &str, b: &str) -> i32 {
    fn parse_parts(s: &str) -> (Vec<u32>, Vec<&str>) {
        let parts: Vec<&str> = s.split('-').collect();
        let main: Vec<u32> = parts[0]
            .split('.')
            .filter_map(|x| x.parse().ok())
            .collect();
        let pre: Vec<&str> = if parts.len() > 1 {
            parts[1].split('.').collect()
        } else {
            vec![]
        };
        (main, pre)
    }
    
    let (a_main, a_pre) = parse_parts(a);
    let (b_main, b_pre) = parse_parts(b);
    
    let len = std::cmp::max(a_main.len(), b_main.len());
    for i in 0..len {
        let av = a_main.get(i).unwrap_or(&0);
        let bv = b_main.get(i).unwrap_or(&0);
        if av != bv {
            return if av < bv { -1 } else { 1 };
        }
    }
    
    // main equal: stable > pre-release
    if a_pre.is_empty() && !b_pre.is_empty() {
        return 1;
    }
    if !a_pre.is_empty() && b_pre.is_empty() {
        return -1;
    }
    
    for i in 0..std::cmp::max(a_pre.len(), b_pre.len()) {
        let av = a_pre.get(i).unwrap_or(&"");
        let bv = b_pre.get(i).unwrap_or(&"");
        if av == bv {
            continue;
        }
        let a_num = av.parse::<u32>().ok();
        let b_num = bv.parse::<u32>().ok();
        if let (Some(an), Some(bn)) = (a_num, b_num) {
            return if an < bn { -1 } else { 1 };
        }
        return if av < bv { -1 } else { 1 };
    }
    
    0
}

/// Check if version string contains a pre-release marker
pub fn is_prerelease(version: &str) -> bool {
    version.contains('-')
}

/// Cache structure for version check
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionCheckCache {
    pub latest_version: String,
    pub checked_at: u64, // epoch ms
    pub channel: String, // "latest" or "beta"
}

/// Default TTL: 6 hours in milliseconds
pub const DEFAULT_TTL_MS: u64 = 6 * 60 * 60 * 1000;

/// Check if cache is still fresh
pub fn is_cache_fresh(cache: &Option<VersionCheckCache>, ttl_ms: u64, now: u64) -> bool {
    match cache {
        Some(c) => now - c.checked_at < ttl_ms,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions_orders_main_triplets() {
        assert_eq!(compare_versions("0.2.9", "0.2.10"), -1);
        assert_eq!(compare_versions("0.3.0", "0.2.99"), 1);
        assert_eq!(compare_versions("1.0.0", "1.0.0"), 0);
    }

    #[test]
    fn test_compare_versions_prerelease_less_than_stable() {
        assert_eq!(compare_versions("0.3.0-beta.1", "0.3.0"), -1);
        assert_eq!(compare_versions("0.3.0", "0.3.0-beta.1"), 1);
    }

    #[test]
    fn test_compare_versions_orders_prerelease_identifiers() {
        assert_eq!(compare_versions("0.3.0-beta.1", "0.3.0-beta.2"), -1);
        assert_eq!(compare_versions("0.3.0-beta.10", "0.3.0-beta.2"), 1);
        assert_eq!(compare_versions("0.3.0-alpha", "0.3.0-beta"), -1);
    }

    #[test]
    fn test_is_prerelease() {
        assert!(is_prerelease("0.3.0-beta.0"));
        assert!(!is_prerelease("0.3.0"));
    }

    #[test]
    fn test_is_cache_fresh_respects_ttl() {
        let cache = Some(VersionCheckCache {
            latest_version: "1.0.0".to_string(),
            checked_at: 1000,
            channel: "latest".to_string(),
        });
        
        assert!(is_cache_fresh(&cache, 100, 1050));
        assert!(!is_cache_fresh(&cache, 100, 1200));
        assert!(!is_cache_fresh(&None, 100, 1200));
    }

    #[test]
    fn test_default_ttl_is_six_hours() {
        assert_eq!(DEFAULT_TTL_MS, 6 * 60 * 60 * 1000);
    }

    #[test]
    fn test_version_check_cache_serialization() {
        let cache = VersionCheckCache {
            latest_version: "1.2.3".to_string(),
            checked_at: 1234567890,
            channel: "latest".to_string(),
        };
        
        let json = serde_json::to_string(&cache).unwrap();
        let restored: VersionCheckCache = serde_json::from_str(&json).unwrap();
        
        assert_eq!(restored.latest_version, "1.2.3");
        assert_eq!(restored.checked_at, 1234567890);
        assert_eq!(restored.channel, "latest");
    }
}
