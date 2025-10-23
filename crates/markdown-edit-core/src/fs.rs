use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::EditResult;

pub fn write_atomic(path: &Path, content: &str, backup: bool) -> EditResult<()> {
    let tmp_path = unique_tmp_path(path);
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
    }

    if backup {
        let backup_path = path.with_extension("bak");
        if let Err(err) = fs::copy(path, &backup_path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(err.into());
        }
    }

    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(err.into());
    }

    Ok(())
}

fn unique_tmp_path(path: &Path) -> PathBuf {
    let mut counter = 0u32;
    loop {
        let candidate = if counter == 0 {
            path.with_extension("tmp")
        } else {
            path.with_extension(format!("tmp{counter}"))
        };

        if !candidate.exists() {
            return candidate;
        }

        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn writes_atomically_with_backup() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("doc.md");
        fs::write(&file_path, "hello").unwrap();

        write_atomic(&file_path, "updated", true).unwrap();

        assert_eq!(fs::read_to_string(&file_path).unwrap(), "updated");
        assert_eq!(
            fs::read_to_string(file_path.with_extension("bak")).unwrap(),
            "hello"
        );
    }
}
