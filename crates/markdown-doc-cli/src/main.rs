//! Temporary CLI entrypoint that will evolve with the real command surface.

use markdown_doc_core::MarkdownDoc;
use markdown_doc_config::Config;

fn main() {
    let engine = MarkdownDoc::bootstrap(Config::default());
    let _ops = engine.operations();
    println!("markdown-doc CLI scaffolding ready");
}
