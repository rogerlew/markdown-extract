use std::path::{Component, Path, PathBuf};

/// Return true if the provided target points to an external resource (http/mailto/etc.).
pub fn is_external(target: &str) -> bool {
    let lower = target.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("data:")
}

/// Determine whether the provided string looks like a Markdown file path.
pub fn is_markdown_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".md") || lower.ends_with(".markdown")
}

/// Split a Markdown link target into its path and optional anchor components.
pub fn split_link_target(target: &str) -> (&str, Option<&str>) {
    if let Some((path, anchor)) = target.split_once('#') {
        if path.is_empty() {
            ("", Some(anchor))
        } else {
            (path, Some(anchor))
        }
    } else if let Some(stripped) = target.strip_prefix('#') {
        ("", Some(stripped))
    } else {
        (target, None)
    }
}

/// Resolve a link target relative to the given source path and project root.
pub fn resolve_relative_path(base: &Path, target: &str, root: &Path) -> ResolvedPath {
    let base_absolute = root.join(base);
    let base_dir = base_absolute.parent().unwrap_or(root);
    let mut combined = if target.starts_with('/') {
        root.join(target.trim_start_matches('/'))
    } else {
        base_dir.join(target)
    };
    combined = normalize_path(combined);
    let relative = combined
        .strip_prefix(root)
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|_| combined.clone());
    ResolvedPath {
        relative,
        absolute: combined,
    }
}

/// Canonicalise `.` and `..` path segments without touching the filesystem.
pub fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

/// Resolved path result containing both relative and absolute forms.
#[derive(Clone, Debug)]
pub struct ResolvedPath {
    pub relative: PathBuf,
    pub absolute: PathBuf,
}

/// Compute a relative path from `from` to `to`. Returns `None` when the paths
/// reside on different filesystem roots (e.g., different Windows drives).
pub fn relative_path(from: &Path, to: &Path) -> Option<PathBuf> {
    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();

    if has_mismatched_prefix(&from_components, &to_components) {
        return None;
    }

    let mut common = 0usize;
    while common < from_components.len()
        && common < to_components.len()
        && components_equal(from_components[common], to_components[common])
    {
        common += 1;
    }

    let mut result = PathBuf::new();
    for component in from_components.iter().skip(common) {
        match component {
            Component::RootDir | Component::Prefix(_) => {}
            Component::CurDir => {}
            _ => result.push(".."),
        }
    }

    for component in to_components.iter().skip(common) {
        match component {
            Component::CurDir => {}
            _ => result.push(component.as_os_str()),
        }
    }

    if result.as_os_str().is_empty() {
        result.push(".");
    }

    Some(result)
}

fn has_mismatched_prefix(from: &[Component<'_>], to: &[Component<'_>]) -> bool {
    match (from.first(), to.first()) {
        (Some(Component::Prefix(fp)), Some(Component::Prefix(tp))) => fp.kind() != tp.kind(),
        (Some(Component::Prefix(_)), _) | (_, Some(Component::Prefix(_))) => true,
        _ => false,
    }
}

fn components_equal(a: Component<'_>, b: Component<'_>) -> bool {
    match (a, b) {
        (Component::Prefix(pa), Component::Prefix(pb)) => pa.kind() == pb.kind(),
        (Component::RootDir, Component::RootDir) => true,
        (Component::CurDir, Component::CurDir) => true,
        (Component::ParentDir, Component::ParentDir) => true,
        _ => a.as_os_str() == b.as_os_str(),
    }
}
