//! Output formatters for markdown-doc commands.

use markdown_doc_config::Config;

/// Placeholder renderer that will evolve into Markdown/JSON/SARIF emitters.
pub struct Renderer {
    #[allow(dead_code)]
    config: Config,
}

impl Renderer {
    /// Build a renderer from configuration.
    pub fn from_config(config: Config) -> Self {
        Self { config }
    }
}
