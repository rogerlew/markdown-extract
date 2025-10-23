# markdown-edit Specification

> **Version**: 1.0.0  
> **Date**: 2025-10-23  
> **Status**: Specification (Not Implemented)

## Overview

`markdown-edit` is a command-line tool for structured editing of Markdown files using heading-based section identification. It enables atomic, heading-aware operations like replace, delete, insert, and append without manual line-number tracking or complex text matching.

## Goals

1. **Heading-Aware Editing**: Operate on logical sections identified by Markdown headings
2. **Safety First**: Default backups, dry-run support, validation before writing
3. **Agent-Friendly**: Clear semantics for AI coding assistants
4. **Human-Friendly**: Intuitive commands that match mental model of document structure
5. **Composable**: Support file, string, and stdin input for different workflows

## Non-Goals

- General-purpose text editor (use `sed`, `awk`, or IDE for that)
- Markdown AST manipulation (keep it simple)
- Cross-document operations (single file at a time)
- Formatting/linting (use dedicated tools)

## Command Interface

### Basic Syntax

```bash
markdown-edit <file> <operation> <section> [content-source] [options]
```

### Operations

#### 1. `replace`
Replace entire section content, optionally preserving the heading.

```bash
# Replace section content from file
markdown-edit doc.md replace "Section Title" --with new-content.md

# Replace with inline string
markdown-edit doc.md replace "Section Title" --with-string "New content"

# Replace from stdin
cat new-content.md | markdown-edit doc.md replace "Section Title" --with -

# Keep original heading, replace only body
markdown-edit doc.md replace "Section Title" --with new-body.md --keep-heading
```

**Behavior**:
- Finds section by heading match
- Replaces from heading line (inclusive) to next same-or-higher-level heading (exclusive)
- With `--keep-heading`: preserves original heading line, replaces everything after until next section

#### 2. `delete`
Remove entire section including heading.

```bash
# Delete section
markdown-edit doc.md delete "Deprecated Section"

# Case-sensitive match
markdown-edit doc.md delete "API Reference" --case-sensitive
```

**Behavior**:
- Removes heading line through end of section
- Collapses whitespace (removes trailing blank lines)

#### 3. `insert-after`
Insert new section immediately after target section.

```bash
# Insert from file
markdown-edit doc.md insert-after "Phase 5" --with new-phase.md

# Insert inline
markdown-edit doc.md insert-after "Introduction" \
  --with-string "## New Section\nContent here"
```

**Behavior**:
- Finds end of target section
- Inserts content at that boundary
- Ensures blank line separation between sections

#### 4. `insert-before`
Insert new section immediately before target section.

```bash
# Insert before existing section
markdown-edit doc.md insert-before "Conclusion" --with summary.md
```

**Behavior**:
- Finds start of target section
- Inserts content before heading line
- Ensures blank line separation

#### 5. `append-to`
Append content to end of section body (before next section).

```bash
# Append to change log
markdown-edit CHANGELOG.md append-to "Unreleased" \
  --with-string "- Fixed bug in parser"

# Append from file
markdown-edit roadmap.md append-to "Q4 Goals" --with new-items.md
```

**Behavior**:
- Finds end of section (before next heading or EOF)
- Appends content at that position
- Preserves existing blank line patterns

#### 6. `prepend-to`
Prepend content to start of section body (after heading).

```bash
# Prepend urgent item
markdown-edit TODO.md prepend-to "High Priority" \
  --with-string "- URGENT: Fix security issue"
```

**Behavior**:
- Finds heading line
- Inserts content immediately after heading
- Ensures blank line after heading

### Content Source Arguments

**Exactly one required** for operations needing content (not needed for `delete`):

```bash
--with <path>           # Read from file ("-" for stdin)
--with-string <text>    # Use literal string (supports \n, \t escapes)
```

**Rules**:
- `--with <path>`: If file exists, read it; if "-", read stdin; otherwise error
- `--with-string <text>`: Treat as literal text, process escape sequences
- Cannot specify both

### Options

```bash
--keep-heading          # For 'replace': preserve original heading
--backup                # Create .bak file (default: true)
--no-backup             # Skip backup creation
--dry-run               # Show changes without writing file
--case-sensitive        # Exact heading match (default: case-insensitive)
--all                   # Apply to all matching sections (default: error on multiple matches)
--quiet                 # Suppress output except errors
--help                  # Show usage information
--version               # Show version
```

## Section Matching

### Heading Identification

Uses same regex as `markdown-extract`:

```rust
// ATX-style headings: # Heading
let atx_pattern = r"^(#{1,6})\s+(.+)$";

// Setext-style headings:
// Heading
// =======  (H1)
// -------  (H2)
let setext_h1_pattern = r"^(.+)\n=+$";
let setext_h2_pattern = r"^(.+)\n-+$";
```

**Matching Behavior**:
- Case-insensitive by default (use `--case-sensitive` for exact match)
- Matches heading text after `#` symbols
- Supports both ATX (`# Title`) and Setext (underlined) styles
- Regex-capable: pattern matching on heading text

### Section Boundaries

A section spans:
- **Start**: Heading line (inclusive)
- **End**: Next heading of same or higher level (exclusive), or EOF

**Examples**:

```markdown
# H1 Section          <- Start of "H1 Section"
Content...

## H2 Subsection      <- End of "H1 Section", Start of "H2 Subsection"
More content...

### H3 Deep          <- Part of "H2 Subsection"
Details...

## H2 Another        <- End of "H2 Subsection", Start of "H2 Another"
...

# H1 Next            <- End of "H1 Section", Start of "H1 Next"
```

**Matching Rules**:
1. Find first heading matching pattern
2. Determine heading level (count `#` symbols or detect underline style)
3. Section ends at next heading with level ≤ current level
4. If no subsequent heading, section ends at EOF

**Multiple Matches**:
- Default: Error if pattern matches multiple sections
- With `--all`: Apply operation to all matching sections in order

## Error Handling

### Exit Codes

```bash
0   Success
1   Section not found
2   Multiple sections matched (use --all or be more specific)
3   Invalid arguments
4   File I/O error
5   Content source error (file not found, stdin closed, etc.)
6   Validation error (would corrupt document structure)
```

### Validation Checks

Before writing, validate:
1. ✅ File exists and is readable
2. ✅ Section heading found (unless `insert-*` operations)
3. ✅ Content source accessible (file exists, stdin available)
4. ✅ Replacement content is valid UTF-8
5. ✅ Operation won't create malformed Markdown (e.g., double headings)

### Error Messages

Clear, actionable error messages:

```bash
# Section not found
Error: Section "Nonexistent" not found in document.md
Hint: Use `markdown-extract ".*" document.md --all` to list all sections

# Multiple matches
Error: Pattern "Benefits" matches 3 sections in document.md:
  Line 45: ## Benefits (AI Agents)
  Line 102: ## Benefits (Human Developers)
  Line 210: ### Long-term Benefits
Hint: Use more specific pattern or --all flag

# Invalid content source
Error: Content file 'missing.md' not found
Hint: Check path or use --with-string for inline content

# Validation failure
Error: Operation would create document with duplicate H1 headings at same level
Hint: Review section structure or use --keep-heading
```

## Safety Features

### Automatic Backups

**Default behavior**:
```bash
# Original file automatically backed up
markdown-edit doc.md replace "Section" --with new.md
# Creates: doc.md.bak (original content)
# Updates: doc.md (modified content)
```

**Backup naming**:
- Single backup: `<filename>.bak`
- Overwrites previous backup (no accumulation)
- Disabled with `--no-backup`

### Dry Run Mode

```bash
markdown-edit doc.md replace "Section" --with new.md --dry-run
```

**Output shows**:
```diff
=== Dry Run: No changes will be written ===
File: doc.md
Operation: replace "Section"
Content source: new.md

--- Original
+++ Proposed
@@ -45,8 +45,6 @@
-## Section
-Old content here
-More old content
+## Section
+New content from file

=== End Dry Run ===
```

## Content Handling

### File Input

```bash
--with <path>
```

**Behavior**:
- Read entire file into memory
- Validate UTF-8 encoding
- Preserve exact content (no trimming, no normalization)
- Special case: `--with -` reads from stdin until EOF

**Example**:
```bash
markdown-edit doc.md replace "Metrics" --with new-metrics.md
```

### String Input

```bash
--with-string <text>
```

**Behavior**:
- Process escape sequences: `\n`, `\t`, `\\`, `\"`
- Preserve literal formatting
- Suitable for shell heredocs or quoted strings

**Examples**:
```bash
# Single line
markdown-edit doc.md replace "Status" --with-string "Status: ✅ Complete"

# Multi-line with escapes
markdown-edit doc.md append-to "TODO" --with-string "- Task 1\n- Task 2\n"

# Using heredoc (recommended for multi-line)
markdown-edit doc.md replace "Section" --with-string "$(cat <<'EOF'
## New Section
Multiple lines
of content
EOF
)"
```

### Stdin Input

```bash
--with -
```

**Behavior**:
- Read stdin until EOF
- Buffer entire input before processing
- Validate UTF-8

**Examples**:
```bash
# Pipe from file
cat new-content.md | markdown-edit doc.md replace "Section" --with -

# Pipe from command
python generate_metrics.py | markdown-edit report.md replace "Metrics" --with -

# Heredoc via stdin
markdown-edit doc.md replace "Section" --with - <<'EOF'
New section content
Multiple lines
EOF
```

## Implementation Guidelines

### Language & Dependencies

**Rust** (consistent with `markdown-extract`):
- `clap` for CLI parsing
- `regex` for heading matching
- `similar` or `diffy` for dry-run diffs (optional)

### Core Functions

#### 1. Section Finder
```rust
struct Section {
    heading: String,
    heading_level: usize,
    start_line: usize,
    start_byte: usize,
    end_line: usize,
    end_byte: usize,
}

fn find_sections(content: &str, pattern: &str, case_sensitive: bool) 
    -> Result<Vec<Section>>
```

#### 2. Content Replacer
```rust
fn replace_section(
    content: &str,
    section: &Section,
    replacement: &str,
    keep_heading: bool
) -> Result<String>
```

#### 3. Content Inserter
```rust
fn insert_content(
    content: &str,
    position: InsertPosition,
    new_content: &str
) -> Result<String>

enum InsertPosition {
    After(Section),
    Before(Section),
    AppendTo(Section),
    PrependTo(Section),
}
```

#### 4. Validator
```rust
fn validate_operation(
    original: &str,
    modified: &str,
    operation: &Operation
) -> Result<()>

// Checks:
// - Valid UTF-8
// - No malformed headings
// - Reasonable size (<100MB)
// - No corruption of surrounding sections
```

### File Operations

**Atomic writes**:
```rust
fn write_with_backup(path: &Path, content: &str, backup: bool) -> Result<()> {
    if backup {
        // Copy original to .bak
        std::fs::copy(path, path.with_extension("bak"))?;
    }
    
    // Write to temporary file first
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, content)?;
    
    // Atomic rename
    std::fs::rename(&tmp, path)?;
    
    Ok(())
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    // Section finding
    #[test] fn find_single_section()
    #[test] fn find_multiple_sections()
    #[test] fn find_nested_sections()
    #[test] fn case_insensitive_match()
    #[test] fn regex_pattern_match()
    
    // Content operations
    #[test] fn replace_preserves_heading()
    #[test] fn replace_entire_section()
    #[test] fn delete_removes_section()
    #[test] fn insert_after_adds_content()
    #[test] fn append_to_extends_section()
    
    // Edge cases
    #[test] fn section_at_eof()
    #[test] fn multiple_blank_lines()
    #[test] fn unicode_headings()
    #[test] fn setext_heading_support()
    
    // Error conditions
    #[test] fn section_not_found()
    #[test] fn multiple_matches_error()
    #[test] fn invalid_utf8()
}
```

### Integration Tests

```bash
# Test files in tests/fixtures/
tests/
  fixtures/
    simple.md
    nested.md
    complex.md
    unicode.md
  integration_tests.rs
  cli_tests.rs
```

**Test scenarios**:
1. Replace section in simple document
2. Delete middle section preserves structure
3. Insert after updates line numbers correctly
4. Backup file created with original content
5. Dry-run shows correct diff
6. Multiple matches with --all flag
7. Stdin input works correctly
8. Large files (>10MB) handle efficiently

### CLI Tests

Use `assert_cmd` crate:
```rust
#[test]
fn test_replace_section() {
    Command::cargo_bin("markdown-edit")
        .unwrap()
        .arg("test.md")
        .arg("replace")
        .arg("Section")
        .arg("--with")
        .arg("new.md")
        .assert()
        .success();
}
```

## Examples & Common Patterns

### Change Log Maintenance

```bash
# Add new entry to top of unreleased section
markdown-edit CHANGELOG.md prepend-to "Unreleased" \
  --with-string "- Fixed critical bug (#123)"

# Replace entire unreleased section with released version
cat > /tmp/release.md <<'EOF'
## [1.2.0] - 2025-10-23
### Added
- New feature X
### Fixed
- Bug Y
EOF

markdown-edit CHANGELOG.md replace "Unreleased" --with /tmp/release.md
```

### Living Documentation Updates

```bash
# Update metrics table
python scripts/generate_metrics.py > /tmp/metrics.md
markdown-edit docs/SURVEY.md replace "Quantitative Metrics" \
  --with /tmp/metrics.md --keep-heading

# Delete deprecated section
markdown-edit docs/ROADMAP.md delete "Old Goals"

# Add new quarterly section
markdown-edit docs/ROADMAP.md insert-after "Q3 2025" \
  --with sections/q4-2025.md
```

### README Maintenance

```bash
# Update installation instructions
markdown-edit README.md replace "Installation" \
  --with docs/install.md --keep-heading

# Add new contributor
markdown-edit README.md append-to "Contributors" \
  --with-string "- New Contributor (@username)"
```

### Multi-Document Updates

```bash
# Update same section across multiple files
for file in docs/*.md; do
    markdown-edit "$file" replace "Footer" --with common/footer.md
done

# Batch delete deprecated sections
find docs -name "*.md" -exec \
    markdown-edit {} delete "Deprecated" \;
```

## Performance Considerations

### Efficiency Targets

- Small files (<1MB): <10ms
- Medium files (1-10MB): <100ms
- Large files (10-100MB): <1s
- Memory usage: O(file size), single-pass where possible

### Optimizations

1. **Lazy Loading**: Read file only when needed
2. **Single Pass**: Find sections during initial parse
3. **Byte Offsets**: Use byte positions, not line numbers, for slicing
4. **Streaming Output**: Write modified content incrementally for large files
5. **Regex Compilation**: Compile patterns once, reuse

## Installation & Distribution

### Build

```bash
cd markdown-edit
cargo build --release
```

### Install

```bash
# Local install
cargo install --path .

# From crates.io (future)
cargo install markdown-edit

# Binary distribution
# Provide pre-built binaries for Linux, macOS, Windows
```

### Integration with wepppy

```bash
# Add to wepppy tools
cp target/release/markdown-edit /usr/local/bin/

# Or via wctl wrapper
wctl markdown-edit <args>  # Calls docker exec or host binary
```

## Future Enhancements (Post-MVP)

### Phase 2 Features

- [ ] `rename` operation: Change section heading text
- [ ] `move` operation: Relocate section to different position
- [ ] `merge` operation: Combine multiple sections
- [ ] `split` operation: Break section into subsections
- [ ] Table-aware operations (update specific cells/rows)
- [ ] List manipulation (add/remove items)
- [ ] Front matter support (YAML/TOML metadata)

### Phase 3 Features

- [ ] Interactive mode: Prompt for section selection
- [ ] Batch mode: Apply multiple operations from script
- [ ] Template expansion: Replace placeholders in content
- [ ] Validation rules: Custom checks via config file
- [ ] Git integration: Auto-commit after changes
- [ ] Undo support: Restore from backup

## Success Criteria

### For AI Agents

✅ Can update living documents without reading entire file  
✅ Clear operation semantics (no ambiguous string matching)  
✅ Atomic operations (all-or-nothing)  
✅ Reliable error messages (actionable, not cryptic)  
✅ Supports programmatic workflows (stdin, exit codes)

### For Human Developers

✅ Intuitive commands that match mental model  
✅ Safe defaults (backups, validation)  
✅ Clear dry-run output  
✅ Fast enough for interactive use (<100ms typical)  
✅ Good error messages with hints

### For wepppy Project

✅ Reduces errors in documentation maintenance  
✅ Enables automated documentation updates  
✅ Supports quarterly review workflows  
✅ Integrates with existing tooling (`wctl`, scripts)  
✅ Low maintenance burden (minimal dependencies)

## Non-Functional Requirements

### Reliability

- No data loss (atomic writes, backups)
- Idempotent operations (same input → same output)
- Graceful degradation (validate, then fail early)

### Maintainability

- Clear code structure (separate concerns)
- Comprehensive tests (>80% coverage)
- Good error messages (developer experience)

### Compatibility

- UTF-8 text files only (no binary)
- Unix/Windows line endings (normalize on read)
- CommonMark Markdown (relaxed parsing)

## CLI Quick Reference

| Command | Description | Key Flags |
|---------|-------------|-----------|
| `markdown-edit <file> replace <pattern>` | Replace the matched section using a new payload | `--with`, `--with-string`, `--keep-heading` / `--body-only`, `--allow-duplicate` |
| `markdown-edit <file> delete <pattern>` | Remove the section entirely | `--dry-run`, `--backup` / `--no-backup`, `--all`, `--max-matches` |
| `markdown-edit <file> append-to <pattern>` | Append payload text to the end of the section body | `--with`, `--with-string`, `--allow-duplicate`, `--dry-run` |
| `markdown-edit <file> prepend-to <pattern>` | Insert payload immediately after the heading line | Same as append |
| `markdown-edit <file> insert-after <pattern>` | Insert a new section after the matched section | `--with`, `--with-string`, `--allow-duplicate`, `--dry-run` |
| `markdown-edit <file> insert-before <pattern>` | Insert a new section before the matched section | Same as insert-after |

Global flags: `--all`, `--max-matches`, `--case-sensitive`, `--dry-run`, `--backup` / `--no-backup`, `--quiet`, `--allow-duplicate`.

Companion note: the `markdown-extract` CLI now accepts `-` as the file argument to read Markdown from `stdin`, enabling pipelines that feed directly into the editor.

## References

- [CommonMark Spec](https://spec.commonmark.org/)
- [markdown-extract source](https://github.com/rogerlew/markdown-extract) (sibling tool)
- [clap CLI framework](https://docs.rs/clap/)
- [similar diff library](https://docs.rs/similar/)

---

**Document Status**: Ready for implementation  
**Estimated Effort**: 2-3 days (MVP), 5-7 days (full Phase 1)  
**Author**: GitHub Copilot (AI Agent)  
**Last Updated**: 2025-10-23
