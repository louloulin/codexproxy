//!
//! OpenAI Proxy Library

pub mod auth;
pub mod codex;
pub mod config;
pub mod db;
pub use db::codex_history::{CodexHistoryStore, CodexHistoryEntry, HistoryKind};
pub mod error;
pub mod error_response;
pub mod handlers;
pub mod integration;
pub mod logging;
pub mod models;
pub mod protocol;
pub mod providers;
pub mod server;
pub mod setup;
pub mod upstream;
pub mod sse;
pub mod streaming_new;
pub mod transform;
pub mod util;

pub use auth::{AuthContext, AuthMode, AuthGuardResult, check_auth};
pub use codex::{apply_codex, codex_dir, atomic_write, backup_file};
pub use config::Config;
pub use error::Error;
pub use handlers::AppState;
pub use setup::{build_snippet_bundle, build_cc_switch_files, resolve_snippet_target, CcSwitchFiles, HostConfig, ProviderTarget, SnippetBundle};
pub use upstream::{install_proxy_dispatcher_from_env, redact_proxy_url, ProxyStatus};
pub use streaming_new::{SseEventBuilder, StreamingState};
