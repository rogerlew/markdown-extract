//! Shared test harness utilities for markdown-doc crates.

use markdown_doc_config::Config;

/// Returns a baseline configuration for tests.
pub fn test_config() -> Config {
    Config::default()
}
