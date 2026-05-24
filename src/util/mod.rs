//! Utility modules

mod cli_color;
mod dotenv;
mod redact;

pub use cli_color::{detect_color_level, fg, bg, reset};
pub use dotenv::{parse_dotenv, load_dotenv_file, DotenvParseResult, DotenvEntry, SkippedLine};
pub use redact::redact_sensitive;
