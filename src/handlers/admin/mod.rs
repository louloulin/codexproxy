//!
//! Admin UI handlers
//!
//! Provides web-based administration interface for:
//! - Provider status overview
//! - Request statistics
//! - Health checks
//! - Codex switch API
//! - User management

pub mod handlers;
pub mod templates;
pub mod codex_switch;
pub mod users;
pub mod extras;

pub use handlers::*;
pub use codex_switch::*;
pub use users::*;
pub use extras::*;
