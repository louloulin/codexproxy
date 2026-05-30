use rust_embed::Embed;
use axum::{
    http::{StatusCode, header, Uri},
    response::Response,
};

/// Embedded admin SPA files (from rcodex-admin/dist)
#[derive(Embed)]
#[folder = "rcodex-admin/dist"]
struct Assets;

/// Serve a file from embedded assets
fn serve_file(path: &str) -> Response {
    let path = path.trim_start_matches('/');

    // Try exact match first
    if let Some(file) = Assets::get(path) {
        return make_response(path, file);
    }

    // Try index.html for SPA routes
    if path.is_empty() || path.ends_with('/') {
        if let Some(file) = Assets::get("index.html") {
            return make_response("index.html", file);
        }
    }

    // For SPA, try serving index.html for any non-file path
    let index_path = if !path.contains('.') {
        "index.html"
    } else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(axum::body::Body::empty())
            .unwrap();
    };

    if let Some(file) = Assets::get(index_path) {
        make_response(index_path, file)
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(axum::body::Body::empty())
            .unwrap()
    }
}

fn make_response(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .header(header::CACHE_CONTROL, "public, max-age=31536000")
        .body(axum::body::Body::from(file.data.into_owned()))
        .unwrap()
}

/// Serve SPA for admin routes
pub async fn serve_spa(uri: Uri) -> Response {
    let path = uri.path();

    // Handle /admin and /admin/* routes
    let spa_path = if path.starts_with("/admin") {
        path.strip_prefix("/admin").unwrap_or("")
    } else if path == "/admin" {
        ""
    } else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(axum::body::Body::empty())
            .unwrap();
    };

    // Strip leading slash for the file lookup
    let file_path = spa_path.trim_start_matches('/');

    // Try to serve the exact file
    if !file_path.is_empty() {
        if let Some(file) = Assets::get(file_path) {
            return make_response(file_path, file);
        }
    }

    // Fallback to index.html for SPA routing
    if let Some(file) = Assets::get("index.html") {
        make_response("index.html", file)
    } else {
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(axum::body::Body::from("Admin SPA not embedded. Please run `just build-admin` first."))
            .unwrap()
    }
}

/// Check if assets are embedded (used for debugging)
pub fn has_embedded_spa() -> bool {
    Assets::get("index.html").is_some()
}
