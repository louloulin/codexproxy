pub mod router;
pub mod static_files;

pub use router::create_server;
pub use static_files::{serve_spa, has_embedded_spa};
