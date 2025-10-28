use std::io;
use std::path::Path;

use regex::{Regex, RegexBuilder};

/// Build a regex with shared defaults used across bindings.
pub fn build_regex(pattern: &str, case_sensitive: bool) -> Result<Regex, regex::Error> {
    RegexBuilder::new(pattern)
        .case_insensitive(!case_sensitive)
        .unicode(true)
        .size_limit(1024 * 100)
        .build()
}

/// Produce a user-friendly message for I/O errors, optionally scoped to a path.
pub fn format_io_error(err: &io::Error, path: Option<&Path>) -> String {
    match (err.kind(), path) {
        (io::ErrorKind::NotFound, Some(p)) => format!("File not found: {}", p.display()),
        (io::ErrorKind::PermissionDenied, Some(p)) => {
            format!("Permission denied: {}", p.display())
        }
        _ => err.to_string(),
    }
}
