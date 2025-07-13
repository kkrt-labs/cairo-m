//! Utility functions shared across the language server

use std::path::PathBuf;

use tower_lsp::lsp_types::Url;
use tracing::debug;

/// Convert path string to URI
///
/// Handles both regular file paths and URI strings starting with "file://"
pub fn get_uri_from_path_str(path_str: &str) -> Result<Url, String> {
    if path_str.starts_with("file://") {
        Url::parse(path_str).map_err(|e| format!("Failed to parse URI: {}", e))
    } else {
        Url::from_file_path(path_str)
            .map_err(|_| format!("Failed to convert path to URI: {}", path_str))
    }
}

/// Get PathBuf from diagnostic file path and URI
///
/// Handles conversion between URI and file path formats
pub fn get_path_from_diagnostic(uri: &Url, file_path: &str) -> Option<PathBuf> {
    if file_path.starts_with("file://") {
        match uri.to_file_path() {
            Ok(path) => Some(path),
            Err(_) => {
                debug!("Warning: Failed to convert URI to file path: {}", uri);
                None
            }
        }
    } else {
        Some(PathBuf::from(file_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_uri_from_path_str_regular_path() {
        let path = "/tmp/test.cm";
        let uri = get_uri_from_path_str(path).unwrap();
        assert_eq!(uri.scheme(), "file");
        assert!(uri.path().ends_with("/tmp/test.cm"));
    }

    #[test]
    fn test_get_uri_from_path_str_uri_format() {
        let uri_str = "file:///tmp/test.cm";
        let uri = get_uri_from_path_str(uri_str).unwrap();
        assert_eq!(uri.scheme(), "file");
        assert_eq!(uri.path(), "/tmp/test.cm");
    }

    #[test]
    fn test_get_path_from_diagnostic_uri_format() {
        let uri = Url::parse("file:///tmp/test.cm").unwrap();
        let file_path = "file:///tmp/test.cm";
        let path = get_path_from_diagnostic(&uri, file_path).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/test.cm"));
    }

    #[test]
    fn test_get_path_from_diagnostic_regular_path() {
        let uri = Url::parse("file:///tmp/test.cm").unwrap();
        let file_path = "/tmp/test.cm";
        let path = get_path_from_diagnostic(&uri, file_path).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/test.cm"));
    }
}
