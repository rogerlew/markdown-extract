/// Compute byte offsets for the start of each line within the provided contents.
pub fn compute_line_offsets(contents: &str) -> Vec<usize> {
    let mut offsets = Vec::new();
    offsets.push(0);
    for (idx, ch) in contents.char_indices() {
        if ch == '\n' {
            offsets.push(idx + 1);
        }
    }
    offsets
}

/// Convert a byte offset into a 1-based line number using the provided offsets.
pub fn byte_to_line(byte: usize, offsets: &[usize]) -> usize {
    match offsets.binary_search(&byte) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    }
}
