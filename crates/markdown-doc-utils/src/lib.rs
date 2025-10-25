//! Shared utilities for markdown-doc crates.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tempfile::Builder;

/// Execute a function over an iterator in parallel.
pub fn parallel_for_each<T, F>(items: T, func: F)
where
    T: IntoParallelIterator,
    F: Fn(T::Item) + Send + Sync,
{
    items.into_par_iter().for_each(func);
}

/// Atomically write the provided string to `path`, ensuring readers never observe
/// partial content. The write is performed via a temporary file in the same
/// directory followed by an atomic rename.
pub fn atomic_write(path: &Path, contents: &str) -> io::Result<()> {
    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| Path::new(".").to_path_buf());
    fs::create_dir_all(&parent)?;

    let mut tmp = Builder::new()
        .prefix(".markdown-doc")
        .tempfile_in(&parent)?;

    tmp.as_file_mut().write_all(contents.as_bytes())?;
    tmp.as_file_mut().sync_all()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let perm = metadata.permissions().mode();
            let _ = fs::set_permissions(tmp.path(), fs::Permissions::from_mode(perm));
        }
    }

    tmp.persist(path).map(|_| ()).map_err(|err| err.error)
}
