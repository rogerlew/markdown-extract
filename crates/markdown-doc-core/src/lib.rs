//! Core orchestration layer for markdown-doc.

use markdown_doc_config::Config;
use markdown_doc_ops::Operations;

/// Entry point for higher-level consumers (CLI, wctl adapters, etc.).
pub struct MarkdownDoc {
    ops: Operations,
}

impl MarkdownDoc {
    /// Bootstrap the markdown-doc engine from configuration.
    pub fn bootstrap(config: Config) -> Self {
        Self {
            ops: Operations::new(config),
        }
    }

    /// Access the operation bundle.
    pub fn operations(&self) -> &Operations {
        &self.ops
    }
}
