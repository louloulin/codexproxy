//! Image Materialization Utilities
//!
//! This module provides utilities for handling images in LLM requests:
//! - Converting data: URLs to http(s): URLs
//! - Downloading and caching remote images
//! - Validating image formats

use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("Invalid image URL format: {0}")]
    InvalidUrl(String),
    #[error("Unsupported image format: {0}")]
    UnsupportedFormat(String),
    #[error("Download failed: {0}")]
    DownloadFailed(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
}

/// Image content types supported by LLMs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    Webp,
}

impl ImageFormat {
    /// Detect format from magic bytes
    pub fn from_magic_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }
        match (bytes[0], bytes[1], bytes[2], bytes[3]) {
            (0xFF, 0xD8, 0xFF, _) => Some(ImageFormat::Jpeg),
            (0x89, 0x50, 0x4E, 0x47) => Some(ImageFormat::Png),
            (0x47, 0x49, 0x46, 0x38) => Some(ImageFormat::Gif), // GIF8
            (0x52, 0x49, 0x46, 0x46) => Some(ImageFormat::Webp), // RIFF...WEBP
            _ => None,
        }
    }
    
    /// Get MIME type string
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Webp => "image/webp",
        }
    }
    
    /// Get file extension
    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Png => "png",
            ImageFormat::Gif => "gif",
            ImageFormat::Webp => "webp",
        }
    }
}

/// Image URL types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageUrlType {
    /// data:image/png;base64,... format
    DataUrl,
    /// https://example.com/image.png format
    HttpUrl,
    /// file:///path/to/image.png format
    FileUrl,
}

/// Parse an image URL and determine its type
pub fn parse_image_url(url: &str) -> (ImageUrlType, String, Option<String>) {
    if url.starts_with("data:") {
        // Parse data URL: data:image/png;base64,...
        if let Some(comma_pos) = url.find(',') {
            let media_type = url[5..comma_pos].to_string();
            let data = url[comma_pos + 1..].to_string();
            (ImageUrlType::DataUrl, data, Some(media_type))
        } else {
            (ImageUrlType::DataUrl, url[5..].to_string(), None)
        }
    } else if url.starts_with("https://") || url.starts_with("http://") {
        (ImageUrlType::HttpUrl, url.to_string(), None)
    } else if url.starts_with("file://") {
        (ImageUrlType::FileUrl, url[7..].to_string(), None)
    } else {
        // Assume it's a relative path or just a URL
        (ImageUrlType::HttpUrl, url.to_string(), None)
    }
}

/// Materialize a data: URL to a temporary file
/// Returns the path to the cached file
pub async fn materialize_data_url(
    data_url: &str,
    cache_dir: &Path,
) -> Result<String, ImageError> {
    let (url_type, data, media_type) = parse_image_url(data_url);
    
    if url_type != ImageUrlType::DataUrl {
        return Err(ImageError::InvalidUrl(
            "Expected data: URL".to_string()
        ));
    }
    
    // Create cache directory if it doesn't exist
    std::fs::create_dir_all(cache_dir)?;
    
    // Determine file extension from media type
    let extension = if let Some(mime) = &media_type {
        if mime.contains("png") {
            "png"
        } else if mime.contains("jpeg") || mime.contains("jpg") {
            "jpg"
        } else if mime.contains("gif") {
            "gif"
        } else if mime.contains("webp") {
            "webp"
        } else {
            "bin"
        }
    } else {
        // Try to detect from base64 data
        "bin"
    };
    
    // Generate unique filename
    let filename = format!(
        "image_{}.{}",
        uuid::Uuid::new_v4(),
        extension
    );
    let filepath = cache_dir.join(&filename);
    
    // Decode base64 and write to file
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| ImageError::InvalidUrl(format!("Base64 decode error: {}", e)))?;
    
    std::fs::write(&filepath, &bytes)?;
    
    Ok(filepath.to_string_lossy().to_string())
}

/// Download a remote image and cache it locally
pub async fn download_and_cache_image(
    url: &str,
    cache_dir: &Path,
) -> Result<String, ImageError> {
    let (url_type, url, _) = parse_image_url(url);
    
    if url_type == ImageUrlType::DataUrl {
        return materialize_data_url(&url, cache_dir).await;
    }
    
    // Create cache directory if it doesn't exist
    std::fs::create_dir_all(cache_dir)?;
    
    // Download the image
    let response = reqwest::get(&url).await?;
    let bytes = response.bytes().await?;
    
    // Detect format from magic bytes
    let extension = ImageFormat::from_magic_bytes(&bytes)
        .map(|f| f.extension())
        .unwrap_or("bin");
    
    // Generate unique filename
    let filename = format!(
        "image_{}.{}",
        uuid::Uuid::new_v4(),
        extension
    );
    let filepath = cache_dir.join(&filename);
    
    std::fs::write(&filepath, &bytes)?;
    
    Ok(filepath.to_string_lossy().to_string())
}

/// Check if an image URL needs materialization
pub fn needs_materialization(url: &str) -> bool {
    let (url_type, _, _) = parse_image_url(url);
    matches!(url_type, ImageUrlType::DataUrl | ImageUrlType::FileUrl)
}

/// Convert any image URL to a local file path
/// Downloads remote images or extracts data URLs to files
pub async fn materialize_image(
    url: &str,
    cache_dir: &Path,
) -> Result<String, ImageError> {
    let (url_type, _, _) = parse_image_url(url);
    
    match url_type {
        ImageUrlType::DataUrl => materialize_data_url(url, cache_dir).await,
        ImageUrlType::HttpUrl => download_and_cache_image(url, cache_dir).await,
        ImageUrlType::FileUrl => {
            // Just validate the file exists
            let path = Path::new(url);
            if path.exists() {
                Ok(url.to_string())
            } else {
                Err(ImageError::InvalidUrl(format!("File not found: {}", url)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data_url() {
        let url = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
        let (url_type, data, media_type) = parse_image_url(url);
        
        assert_eq!(url_type, ImageUrlType::DataUrl);
        assert!(media_type.unwrap().contains("image/png"));
    }

    #[test]
    fn test_parse_http_url() {
        let url = "https://example.com/image.png";
        let (url_type, parsed, _) = parse_image_url(url);
        
        assert_eq!(url_type, ImageUrlType::HttpUrl);
        assert_eq!(parsed, url);
    }

    #[test]
    fn test_needs_materialization() {
        assert!(needs_materialization("data:image/png;base64,..."));
        assert!(needs_materialization("file:///path/to/image.png"));
        assert!(!needs_materialization("https://example.com/image.png"));
    }

    #[test]
    fn test_image_format_detection() {
        // PNG magic bytes
        assert_eq!(
            ImageFormat::from_magic_bytes(&[0x89, 0x50, 0x4E, 0x47]),
            Some(ImageFormat::Png)
        );
        
        // JPEG magic bytes
        assert_eq!(
            ImageFormat::from_magic_bytes(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(ImageFormat::Jpeg)
        );
        
        // Unknown
        assert_eq!(
            ImageFormat::from_magic_bytes(&[0x00, 0x00, 0x00, 0x00]),
            None
        );
    }
}
