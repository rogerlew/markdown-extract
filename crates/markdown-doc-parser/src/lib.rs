//! Markdown parsing helpers for markdown-doc.

use markdown_doc_config::Config;

/// Stub parser environment that will eventually wrap pulldown-cmark.
pub struct ParserContext {
    #[allow(dead_code)]
    config: Config,
}

impl ParserContext {
    /// Construct a new parser context from the provided configuration.
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}
