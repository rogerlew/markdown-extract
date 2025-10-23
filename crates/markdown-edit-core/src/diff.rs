use similar::TextDiff;

pub fn build_unified_diff(original: &str, modified: &str, path: &str) -> Option<String> {
    if original == modified {
        return None;
    }

    let diff = TextDiff::from_lines(original, modified);
    let mut output = Vec::new();
    let header_old = format!("a/{path}");
    let header_new = format!("b/{path}");

    diff.unified_diff()
        .header(&header_old, &header_new)
        .to_writer(&mut output)
        .expect("writing diff to string never fails");

    Some(String::from_utf8(output).expect("diff output is valid utf-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_none_for_identical_content() {
        assert!(build_unified_diff("abc", "abc", "file.txt").is_none());
    }

    #[test]
    fn produces_diff_for_changes() {
        let diff = build_unified_diff("a\n", "b\n", "file.txt").unwrap();
        assert!(diff.contains("-a"));
        assert!(diff.contains("+b"));
    }
}
