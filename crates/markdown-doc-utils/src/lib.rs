//! Shared utilities for markdown-doc crates.

use rayon::iter::{IntoParallelIterator, ParallelIterator};

/// Execute a function over an iterator in parallel.
pub fn parallel_for_each<T, F>(items: T, func: F)
where
    T: IntoParallelIterator,
    F: Fn(T::Item) + Send + Sync,
{
    items.into_par_iter().for_each(func);
}

/// Atomically write content to the target path (placeholder).
pub fn atomic_write(_path: &std::path::Path, _contents: &str) -> std::io::Result<()> {
    // Implementation will arrive with the real write pipeline.
    Ok(())
}
