//! High-level operations shared by markdown-doc commands.

use markdown_doc_config::Config;
use markdown_doc_format::Renderer;
use markdown_doc_parser::ParserContext;
use markdown_doc_utils::{atomic_write, parallel_for_each};

/// Placeholder operation bundle giving the CLI something to hook into.
pub struct Operations {
    #[allow(dead_code)]
    config: Config,
}

impl Operations {
    /// Assemble the operation layer from config by wiring parser + renderer.
    pub fn new(config: Config) -> Self {
        let _parser = ParserContext::new(config.clone());
        let _renderer = Renderer::from_config(config.clone());
        parallel_for_each(vec![()], |_| {});
        let _ = (_parser, _renderer);
        Self { config }
    }

    /// Placeholder catalog write to demonstrate atomic writer usage.
    pub fn write_catalog_stub(&self, path: &std::path::Path) -> std::io::Result<()> {
        atomic_write(path, "# catalog placeholder\n")
    }
}
