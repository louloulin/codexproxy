//! Utility modules

mod check_update;
mod cli_color;
mod dotenv;
mod encryption;
mod redact;

pub use check_update::{compare_versions, is_prerelease, is_cache_fresh, DEFAULT_TTL_MS, VersionCheckCache};
pub use cli_color::{detect_color_level, fg, bg, reset};
pub use dotenv::{parse_dotenv, load_dotenv_file, DotenvParseResult, DotenvEntry, SkippedLine};
pub use encryption::{encrypt_string, decrypt_string, SealedSecret};
pub use redact::redact_sensitive;
