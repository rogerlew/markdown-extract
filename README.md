<div align="center">

# Markdown Extract

[![Crates.io](https://img.shields.io/crates/v/markdown-extract)](https://crates.io/crates/markdown-extract)
[![Docker Pulls](https://img.shields.io/docker/pulls/sean0x42/markdown-extract)](https://hub.docker.com/r/sean0x42/markdown-extract)
[![Build & Test](https://github.com/sean0x42/markdown-extract/actions/workflows/build_and_test.yml/badge.svg)](https://github.com/sean0x42/markdown-extract/actions/workflows/build_and_test.yml)

</div>

A suite of command-line tools for working with Markdown documentation: extract sections by heading, edit content programmatically, and maintain documentation quality.

---

## 🚀 Toolkit Overview

| Tool | Purpose | Key Use Cases |
|------|---------|---------------|
| **[`markdown-extract`](#usage)** | Read sections from Markdown files by heading pattern | Context extraction, release notes, API docs, agent workflows |
| **[`markdown-edit`](#companion-cli-markdown-edit)** | Modify Markdown with heading-aware operations | Automated updates, changelog maintenance, content injection |
| **[`markdown-doc`](#companion-cli-markdown-doc)** | Repository-wide documentation management | Catalog generation, link validation, quality enforcement |

**🤖 For AI Agents:** All three tools are pre-installed at `/usr/local/bin` on managed hosts. They're designed for automation with structured output formats (JSON/SARIF), predictable exit codes, and pipeline-friendly behavior.

---

## Usage

Given a document called `my-document.md`:

```markdown
# Welcome

This is my amazing markdown document.

## Extract me!

This section should be pulled out.
```

You can extract the second section with the following command:

```console
$ markdown-extract "Extract me!" my-document.md
## Extract me!

This section should be pulled out.
```

Pass `-` as the file argument to read Markdown from standard input:

```console
$ cat my-document.md | markdown-extract "Extract me!" -
```

### Quick Reference

| Flag | Short | Description |
|------|-------|-------------|
| `--all` | `-a` | Extract all matching sections (default: first only) |
| `--case-sensitive` | `-s` | Match pattern exactly (default: case-insensitive) |
| `--no-print-matched-heading` | `-n` | Omit heading line from output (body only) |
| `--help` | `-h` | Show help message |
| `--version` | `-V` | Show version |

**Basic syntax:**
```
markdown-extract [OPTIONS] <PATTERN> <FILE>
```

### Pattern Matching

Patterns are **case-insensitive regex** by default, matched against the heading text (without the `#` markers). Use `--case-sensitive` for exact matches.

```console
# Match any heading containing "install"
$ markdown-extract "install" docs.md

# Exact match with anchors
$ markdown-extract "^Installation$" docs.md

# Multiple possibilities with alternation
$ markdown-extract "Setup|Configuration" docs.md

# Subsections only (three or more #)
$ markdown-extract "^###" docs.md
```

### Extracting Multiple Sections

By default, `markdown-extract` returns only the **first match** and exits. Use `--all` to extract every matching section:

```console
# Get all "Usage" sections across the document
$ markdown-extract "Usage" docs.md --all

# Extract all troubleshooting subsections
$ markdown-extract "^### .* Error" docs.md --all
```

### Output Control

The `--no-print-matched-heading` (or `-n`) flag omits the heading line, returning only the section body:

```console
# Get just the content, no heading
$ markdown-extract "Installation" README.md -n

# Useful for extracting code blocks
$ markdown-extract "Example" docs.md -n | grep -A5 "```bash"
```

### Pipeline-Friendly Behavior

`markdown-extract` handles broken pipes gracefully—if you pipe output to `head`, `less`, or any command that closes early, the CLI exits quietly without error messages.

**Reading from stdin:**
```console
# Process generated content
$ generate-docs | markdown-extract "Installation" -

# Chain with other tools
$ curl https://example.com/docs.md | markdown-extract "API" -

# Filter before extracting
$ cat large-docs.md | grep -v "DRAFT" | markdown-extract "Section" -
```

**Piping output:**
```console
# Preview first 10 lines of a long section
$ markdown-extract "API Reference" docs.md | head -10

# Page through output
$ markdown-extract "Changelog" CHANGELOG.md | less
```

### Common Workflows

**Extract release notes for CI/CD:**
```console
$ markdown-extract "^v1\.2\.0$" CHANGELOG.md > release-notes.txt
```

**Pull API documentation into a script:**
```console
API_DOCS=$(markdown-extract "Authentication" docs/api.md)
echo "$API_DOCS" | process-documentation
```

**Combine with other tools:**
```console
# Count lines in a section
$ markdown-extract "Configuration" README.md | wc -l

# Search within a section
$ markdown-extract "Troubleshooting" docs.md | grep -i "error"

# Extract multiple sections and diff them
$ diff <(markdown-extract "Old API" docs.md) <(markdown-extract "New API" docs.md)
```

**Find all matching headings:**
```console
# See what matches without extracting
$ markdown-extract ".*" docs.md --all | grep "^#"
```

**Process dynamic content:**
```console
# Extract from HTTP response
$ curl -s https://api.example.com/docs | markdown-extract "Endpoints" -

# Extract from command output
$ ./generate-report | markdown-extract "Summary" -

# Chain multiple extractions
$ cat docs.md | markdown-extract "Chapter 1" - | markdown-extract "Summary" -
```

### Section Boundaries

A section includes:
- The matched heading line (unless `-n` is used)
- All content until the next heading of **equal or higher level**
- Subsections are included in their parent section

```markdown
## Parent Section     ← Matched heading
Content here.

### Child Section     ← Included (lower level)
More content.

### Another Child     ← Also included
Final content.

## Next Section        ← Boundary (same level)
Not included.
```

**Examples:**

```console
# Extracts "Parent" + both child sections
$ markdown-extract "Parent Section" doc.md

# Extracts only "Child Section" (no siblings)
$ markdown-extract "Child Section" doc.md
```

### Heading Support

Both **ATX** (`#`) and **Setext** (underline) headings are supported:

```markdown
ATX Heading
===========

Setext level 1 heading

Subheading
----------

Setext level 2 heading

### ATX Level 3

Standard markdown heading
```

**Note:** Headings inside fenced code blocks are ignored:

````markdown
## Real Heading

```markdown
## Not a heading (inside code block)
```
````

### Error Handling & Exit Codes

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success (match found and printed) |
| 1 | No matches found for pattern |
| 2 | File I/O error or invalid UTF-8 |

```console
# Check if a section exists
$ markdown-extract "Deprecated" docs.md > /dev/null 2>&1 && echo "Found"

# Fail CI build if required section is missing
$ markdown-extract "^License$" README.md || exit 1
```

### Limitations

- **UTF-8 only**: Non-UTF-8 files will error
- **Regex size limit**: Patterns over 100 KB are rejected
- **Full document scan**: Large files (>100 MB) may have performance implications

### For AI Agent Workflows

`markdown-extract` excels at keeping agent context windows lean:

**Pre-filter knowledge bases:**
```bash
# Extract only relevant sections for the agent
markdown-extract "API.*Auth" knowledge-base.md > context.txt
```

**Dynamic prompt assembly:**
```python
import subprocess

def get_section(pattern, file):
    result = subprocess.run(
        ["markdown-extract", pattern, file],
        capture_output=True, text=True
    )
    return result.stdout if result.returncode == 0 else None

# Pull targeted instructions
auth_docs = get_section("Authentication", "docs/api.md")
if auth_docs:
    prompt += f"\n\nRelevant documentation:\n{auth_docs}"
```

**Automated context refresh:**
```bash
# Update agent context when docs change (cron/GitHub Actions)
markdown-extract "Quick Start" README.md > agent-context/quickstart.md
markdown-extract "^v.*" CHANGELOG.md --all > agent-context/releases.md
```

**Validation in CI:**
```bash
# Ensure required sections exist before deployment
required_sections=("Installation" "Usage" "License")
for section in "${required_sections[@]}"; do
  markdown-extract "^$section$" README.md > /dev/null || {
    echo "Missing required section: $section"
    exit 1
  }
done
```

## Companion CLI: `markdown-edit`

Need to *change* Markdown after you've found the right section? The repository also ships `markdown-edit`, a heading-aware editor that understands the spans emitted by `markdown-extract`.

Key features:

- Operations scoped to headings: `replace`, `delete`, `append-to`, `prepend-to`, `insert-after`, `insert-before`.
- Payload sources via `--with <path>` (or `-` for stdin) and `--with-string "escaped\ntext"`.
- Safety-first by default: dry-run diffs (`--dry-run`), duplicate guards (`--allow-duplicate` to opt out), atomic writes with optional backups (`--backup` / `--no-backup`).
- Validation aligned with the [markdown-edit specification](./markdown-edit.spec.md): heading-level enforcement, single-section payloads, and friendly exit codes for automation.

Example workflow:

```console
# Preview an append without touching disk
$ markdown-edit README.md append-to "^Changelog$" \
    --with-string "- Documented markdown-edit release\n" \
    --dry-run

# Replace a section body but keep the original heading line
$ markdown-edit docs/guide.md replace "Integration Notes" \
    --with-string "New guidance goes here.\n" \
    --keep-heading
```

Use `--all` (optionally `--max-matches N`) for batched edits, `--quiet` for terse runs, and `--case-sensitive` when the default case-insensitive matching is too broad.

### CLI reference

| Command | Description | Common flags |
|---------|-------------|--------------|
| `markdown-edit <file> replace <pattern>` | Replace an entire section (heading + body) | `--with / --with-string`, `--keep-heading` (or `--body-only`), `--allow-duplicate` |
| `markdown-edit <file> delete <pattern>` | Remove the matching section | `--dry-run`, `--backup/--no-backup`, `--all`, `--max-matches` |
| `markdown-edit <file> append-to <pattern>` | Append payload to the end of the section body | `--with / --with-string`, `--allow-duplicate`, `--dry-run` |
| `markdown-edit <file> prepend-to <pattern>` | Insert payload after the heading, before existing content | Same as append |
| `markdown-edit <file> insert-after <pattern>` | Insert a new section after the matched section | `--with / --with-string`, `--allow-duplicate`, `--dry-run` |
| `markdown-edit <file> insert-before <pattern>` | Insert a new section before the matched section | Same as `insert-after` |

Global knobs: `--all`, `--max-matches N`, `--case-sensitive`, `--quiet`, `--dry-run`, `--backup` / `--no-backup`.

#### Escaped inline payloads

`--with-string` supports a small, predictable escape set for automation. Anything outside this list is rejected with exit code 5.

| Escape | Meaning |
|--------|---------|
| `\\n` | Newline |
| `\\t` | Horizontal tab |
| `\\\\` | Literal backslash |
| `\\"` | Double quote |

Examples:

```console
$ markdown-edit notes.md append-to "^Today$" --with-string "- sync status\\n"
$ markdown-edit notes.md replace "Summary" --with-string "All clear\\n" --keep-heading
```

#### Match limits

`--max-matches` caps how many sections can be touched in a single invocation. Pair it with `--all` when you expect multiple hits but want a hard ceiling.

```console
# Allow up to 3 replacements; fail with exit code 2 if more match
$ markdown-edit docs/*.md replace "^Changelog$" \
    --with updates.md \
    --all \
    --max-matches 3
```

#### Validation failures (examples)

The CLI surfaces actionable messages from the core engine:

- Heading depth mismatch: `insert-before payload heading depth 3 must match target depth 2`
- Duplicate sibling: `heading 'Release Notes' already exists at this level`
- Missing payload heading: `replacement payload must begin with a heading`

#### Performance & safety notes

- Large files: the engine streams once through the document, using byte offsets instead of line numbers (tested >=5 MB). Diff generation is the most expensive step in dry-run mode.
- Backups & atomic writes: every write goes to `file.tmp` and promotes to the original name only after fsync; `--no-backup` skips the `.bak` copy.
- Path hygiene: the CLI operates on user-supplied paths. In CI/CD, prefer repository-relative paths or sandboxed working directories when invoking the tool with untrusted input.

## Companion CLI: `markdown-doc`

**🤖 For AI Agents:** `markdown-doc` is pre-installed at `/usr/local/bin` on managed hosts. Use it to maintain documentation catalogs and enforce quality standards.

The `markdown-doc` toolkit provides repository-wide documentation management with two core commands:

### Commands

#### `catalog` - Generate Documentation Index

Creates a unified catalog of all Markdown files with their heading structure. Perfect for navigation, discovery, and keeping track of documentation coverage.

```console
# Generate markdown catalog (default: DOC_CATALOG.md)
$ markdown-doc catalog

# Output to JSON for programmatic processing
$ markdown-doc catalog --format json

# Catalog specific paths only
$ markdown-doc catalog --path docs/

# Check only staged files (great for pre-commit hooks)
$ markdown-doc catalog --staged
```

**Output formats:**
- `--format markdown` (default): Human-readable catalog with clickable links
- `--format json`: Structured data with file paths, heading levels, anchors, and timestamps

**JSON schema:**
```json
{
  "last_updated": "2025-10-25T17:17:48Z",
  "file_count": 227,
  "files": [
    {
      "path": "README.md",
      "headings": [
        {"level": 1, "text": "Title", "anchor": "title"},
        {"level": 2, "text": "Section", "anchor": "section"}
      ]
    }
  ]
}
```

#### `lint` - Quality Enforcement

Runs configurable lint rules to catch common documentation issues. MVP ships with `broken-links` detection.

```console
# Check all markdown files for broken links
$ markdown-doc lint

# Lint specific paths
$ markdown-doc lint --path docs/api/

# Check only staged files (pre-commit)
$ markdown-doc lint --staged

# JSON output for CI/CD pipelines
$ markdown-doc lint --format json

# SARIF format for GitHub Actions / IDE integration
$ markdown-doc lint --format sarif
```

**Output formats:**
- `--format plain` (default): Human-readable with emoji indicators (❌ errors, ⚠️ warnings)
- `--format json`: Structured findings with file/line/severity data
- `--format sarif`: SARIF 2.1.0 format for IDE integration and code scanning platforms

**Exit codes:**
- `0`: All checks passed (or only warnings)
- `1`: Errors found

**JSON schema:**
```json
{
  "summary": {
    "files_scanned": 227,
    "errors": 11,
    "warnings": 0
  },
  "findings": [
    {
      "rule": "broken-links",
      "severity": "error",
      "file": "docs/guide.md",
      "line": 42,
      "message": "Broken link to 'missing-file.md'"
    }
  ]
}
```

#### `toc` - Table of Contents Synchronization

Manages table-of-contents (TOC) blocks between `<!-- toc -->` and `<!-- tocstop -->` markers, ensuring they reflect the current document structure.

```console
# Check if TOC blocks are in sync (default mode)
$ markdown-doc toc --check

# Update TOC blocks in place
$ markdown-doc toc --update

# Preview changes with unified diffs
$ markdown-doc toc --diff

# Target specific files or directories
$ markdown-doc toc --path docs/ --update

# Check only staged files (pre-commit workflow)
$ markdown-doc toc --staged --check

# Ignore .markdown-doc-ignore filtering
$ markdown-doc toc --no-ignore --check
```

**Operation modes:**
- `--check` (default): Report files with out-of-sync or missing TOC markers; no modifications
- `--update`: Rewrite TOC blocks in place with current headings
- `--diff`: Show unified diffs of what would change without writing

**Output examples:**
```console
# Check mode (files with issues)
❌ docs/guide.md missing TOC markers
❌ README.md TOC out of sync

# No issues found
✅ 15 files validated, all TOCs in sync

# Diff mode (shows changes)
--- docs/guide.md
+++ docs/guide.md
@@ -5,7 +5,8 @@
 <!-- toc -->
 - [Overview](#overview)
 - [Installation](#installation)
+- [Configuration](#configuration)
 <!-- tocstop -->
```

**Configuration:**

TOC markers are configurable in `.markdown-doc.toml`:
```toml
[lint]
toc_start_marker = "<!-- toc -->"
toc_end_marker = "<!-- tocstop -->"
```

**Exit codes:**
- `0`: All TOCs in sync (or update succeeded)
- `1`: Out-of-sync TOCs found (check mode) or update failed

**Ignore filtering:**

By default, TOC respects `.markdown-doc-ignore` patterns. Use `--no-ignore` to process all files regardless of ignore rules.

#### `validate` - Template Conformance

Performs deep structural checks using schema definitions to ensure documents match required sections, heading hierarchy, and depth constraints.

```console
# Validate all documentation against configured schemas
$ markdown-doc validate

# Validate a specific file
$ markdown-doc validate --path docs/work-packages/20251025_markdown_doc_toolkit/package.md

# Force a schema by name
$ markdown-doc validate --schema agents AGENTS.md

# Emit machine-readable output
$ markdown-doc validate --format json

# Suppress success output, only show errors
$ markdown-doc validate --quiet

# Bypass .markdown-doc-ignore filtering
$ markdown-doc validate --no-ignore

# Check only staged files
$ markdown-doc validate --staged
```

**Output formats:**
- `--format plain` (default): Human-readable violations per file with schema names
- `--format json`: Structured findings for programmatic processing

**JSON schema:**
```json
{
  "summary": {
    "files_scanned": 42,
    "errors": 3
  },
  "findings": [
    {
      "schema": "readme",
      "file": "docs/api/README.md",
      "line": 0,
      "message": "Missing required section: 'Usage'"
    },
    {
      "schema": "default",
      "file": "docs/guide.md",
      "line": 15,
      "message": "Heading depth exceeds maximum (found 5, max 4)"
    }
  ]
}
```

**Common error messages:**
- `Missing required section: '<section>'` - Document lacks a required heading
- `Schema '<name>' not found in configuration` - Requested schema doesn't exist
- `Heading depth exceeds maximum (found X, max Y)` - Document violates depth constraint
- `Empty section not allowed: '<section>'` - Required section has no content
- `Top-level heading required but not found` - Schema requires H1, document starts with H2+

**Schema configuration example:**

Define schemas in `.markdown-doc.toml` to enforce document structure:

```toml
# Default schema applied to all files unless overridden
[schemas.default]
required_sections = ["Overview", "Details"]
allow_additional = true
allow_empty = false
max_heading_depth = 4

# Schema for README files
[schemas.readme]
patterns = ["**/README.md", "**/readme.md"]
required_sections = ["Overview", "Installation", "Usage"]
require_top_level_heading = true
allow_additional = true
max_heading_depth = 3

# Schema for agent documentation
[schemas.agents]
patterns = ["**/AGENTS.md"]
required_sections = ["Architecture", "Workflows", "Testing"]
allow_additional = true
allow_empty = false
```

**Exit codes:**
- `0`: All files conform to their schemas
- `1`: Validation errors were found
- `2`: Requested schema (`--schema`) was not found
- `3`: Configuration or runtime error occurred during validation

**Ignore filtering:**

By default, validate respects `.markdown-doc-ignore` patterns. Use `--no-ignore` to validate all files, and `--quiet` to suppress output when all checks pass.

### Configuration

`markdown-doc` uses a cascading configuration system with the following precedence (highest to lowest):

1. `--config <path>` CLI override
2. `.markdown-doc.toml` in working directory
3. `.markdown-doc.toml` at git repository root
4. Built-in defaults

**Example `.markdown-doc.toml`:**

```toml
[project]
name = "my-project"
root = "."
exclude = ["**/node_modules/**", "**/vendor/**"]

[catalog]
output = "DOC_CATALOG.md"
include_patterns = ["**/*.md"]
exclude_patterns = ["**/node_modules/**", "**/vendor/**", "**/target/**"]

[lint]
rules = ["broken-links", "toc-sync"]
max_heading_depth = 4
toc_start_marker = "<!-- toc -->"
toc_end_marker = "<!-- tocstop -->"

# Override severity for specific rules
[lint.severity]
broken-links = "error"

# Ignore patterns for specific rules
[[lint.ignore]]
path = "tests/**/*.md"
rules = ["broken-links"]

[[lint.ignore]]
path = "CHANGELOG.md"
rules = ["heading-hierarchy"]

[schemas.default]
required_sections = ["Overview", "Details"]
allow_additional = true
allow_empty = false

[schemas.readme]
patterns = ["**/README.md"]
required_sections = ["Overview", "Usage"]
allow_additional = true
require_top_level_heading = true
```

**Configuration reference:**

| Section | Key | Type | Default | Description |
|---------|-----|------|---------|-------------|
| `project` | `name` | string | (none) | Project display name |
| `project` | `root` | path | `"."` | Repository root directory |
| `project` | `exclude` | glob[] | `[]` | Paths to exclude globally |
| `catalog` | `output` | path | `"DOC_CATALOG.md"` | Where to write catalog |
| `catalog` | `include_patterns` | glob[] | `["**/*.md"]` | Files to include |
| `catalog` | `exclude_patterns` | glob[] | Common build dirs | Files to exclude |
| `lint` | `rules` | string[] | `["broken-links"]` | Active lint rules |
| `lint` | `max_heading_depth` | int (1-6) | `4` | Maximum heading level |
| `lint` | `toc_start_marker` | string | `"<!-- toc -->"` | Opening marker delimiting TOC blocks |
| `lint` | `toc_end_marker` | string | `"<!-- tocstop -->"` | Closing marker delimiting TOC blocks |
| `lint.severity` | `<rule>` | `error`/`warning`/`ignore` | `error` | Override rule severity |
| `lint.ignore` | `path` | glob | (required) | Pattern to ignore |
| `lint.ignore` | `rules` | string[] | (required) | Rules to disable for pattern |
| `schemas.<name>` | `patterns` | glob[] | `[]` | Paths matching the schema (empty applies only when explicitly selected) |
| `schemas.<name>` | `required_sections` | string[] | `[]` | Ordered list of required heading titles |
| `schemas.<name>` | `allow_additional` | bool | `true` | Allow headings beyond the required list |
| `schemas.<name>` | `allow_empty` | bool | `false` | Permit empty documents |
| `schemas.<name>` | `min_sections` | int | (none) | Minimum number of sections required |
| `schemas.<name>` | `min_heading_level` | int (1-6) | (none) | Minimum heading depth allowed |
| `schemas.<name>` | `max_heading_level` | int (1-6) | (none) | Maximum heading depth allowed |
| `schemas.<name>` | `require_top_level_heading` | bool | `true` (default schema) | Require at least one depth-1 heading |

### Available Lint Rules

| Rule | Description | Status |
|------|-------------|--------|
| `broken-links` | Detects internal markdown links to non-existent files | ✅ Available |
| `broken-anchors` | Verifies heading anchors in links exist (intra- and inter-file) | ✅ Available |
| `duplicate-anchors` | Flags duplicate heading IDs in same file | ✅ Available |
| `heading-hierarchy` | Ensures heading levels don't skip (e.g., H1→H3) and respects max depth | ✅ Available |
| `required-sections` | Enforces presence and order of schema-defined sections | ✅ Available |
| `toc-sync` | Validates declared TOC blocks match heading structure | ✅ Available |

### Integration Examples

#### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Only lint staged markdown files
if ! markdown-doc lint --staged --format plain; then
  echo "❌ Documentation lint failed. Fix errors or use 'git commit --no-verify'"
  exit 1
fi

exit 0
```

#### GitHub Actions

```yaml
name: Documentation Quality

on: [push, pull_request]

jobs:
  lint-docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Run markdown-doc lint
        run: |
          markdown-doc lint --format sarif > results.sarif
          
      - name: Upload SARIF results
        uses: github/codeql-action/upload-sarif@v2
        if: always()
        with:
          sarif_file: results.sarif
          
  catalog:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Generate catalog
        run: markdown-doc catalog
        
      - name: Check for changes
        run: |
          if ! git diff --exit-code DOC_CATALOG.md; then
            echo "❌ Catalog is out of date. Run 'markdown-doc catalog' locally."
            exit 1
          fi
```

#### CI/CD Pipeline (GitLab CI)

```yaml
lint:docs:
  stage: test
  script:
    - markdown-doc lint --format json > lint-results.json
  artifacts:
    reports:
      junit: lint-results.json
    when: always
```

### For AI Agent Workflows

**🤖 Both `markdown-doc` commands are installed at `/usr/local/bin` on managed systems.**

`markdown-doc` is designed for automated documentation maintenance and quality enforcement in agent workflows:

**Catalog use cases:**
```bash
# Discover all documentation for context gathering
docs=$(markdown-doc catalog --format json)

# Find files containing specific topics
markdown-doc catalog --format json | \
  jq -r '.files[] | select(.headings[].text | contains("API")) | .path'

# Verify documentation coverage
file_count=$(markdown-doc catalog --format json | jq '.file_count')
if [ "$file_count" -lt 10 ]; then
  echo "Warning: Only $file_count documentation files found"
fi
```

**Lint use cases:**
```bash
# Validate before documentation updates
if ! markdown-doc lint --staged > /dev/null 2>&1; then
  echo "Pre-commit validation failed, rolling back changes"
  git restore --staged .
  exit 1
fi

# Parse findings programmatically
findings=$(markdown-doc lint --format json)
error_count=$(echo "$findings" | jq '.summary.errors')

if [ "$error_count" -gt 0 ]; then
  echo "Found $error_count documentation errors"
  echo "$findings" | jq -r '.findings[] | "\(.file):\(.line): \(.message)"'
  exit 1
fi
```

**Python integration:**
```python
import subprocess
import json

def get_catalog():
    """Fetch structured documentation catalog."""
    result = subprocess.run(
        ["markdown-doc", "catalog", "--format", "json"],
        capture_output=True,
        text=True,
        check=True
    )
    return json.loads(result.stdout)

def lint_docs(paths=None):
    """Run lint checks and return structured findings."""
    cmd = ["markdown-doc", "lint", "--format", "json"]
    if paths:
        for path in paths:
            cmd.extend(["--path", path])
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    findings = json.loads(result.stdout)
    
    return {
        "success": result.returncode == 0,
        "errors": findings["summary"]["errors"],
        "findings": findings["findings"]
    }

# Usage in agent workflows
catalog = get_catalog()
print(f"Found {catalog['file_count']} documentation files")

# Target specific paths
lint_result = lint_docs(paths=["docs/api/"])
if not lint_result["success"]:
    print(f"❌ {lint_result['errors']} errors found")
    for finding in lint_result["findings"]:
        print(f"  {finding['file']}:{finding['line']}: {finding['message']}")
```

**Selective scanning patterns:**
```bash
# Check only API documentation
markdown-doc lint --path "docs/api/**/*.md"

# Catalog specific controller docs
markdown-doc catalog --path "controllers_js/**/README.md" --format json

# Lint work package documents only
markdown-doc lint --path "docs/work-packages/**/*.md"

# Pre-commit: only staged files
markdown-doc lint --staged
```

**Exit code handling:**
```bash
#!/bin/bash
# Robust error handling for automation

set +e  # Don't exit on error
markdown-doc lint --format json > lint-results.json
exit_code=$?
set -e

case $exit_code in
  0)
    echo "✅ All documentation checks passed"
    ;;
  1)
    echo "❌ Documentation errors found:"
    jq -r '.findings[] | "  \(.file):\(.line): \(.message)"' lint-results.json
    exit 1
    ;;
  *)
    echo "⚠️  Lint command failed (exit code $exit_code)"
    exit $exit_code
    ;;
esac
```

### Performance

Benchmarked on 734 Markdown files (~83k lines):
- **Catalog generation**: ~78ms
- **Lint (broken-links)**: ~77ms

The toolkit uses parallel processing and is suitable for pre-commit hooks and CI pipelines.

## Installation

> **⚠️ AI Agents:** On managed hosts, all three tools (`markdown-extract`, `markdown-edit`, `markdown-doc`) are pre-installed at `/usr/local/bin`. You can use them directly without installation.

### Using Cargo

```console
# Install the extractor from crates.io
$ cargo install markdown-extract-cli

# Build the editor from this workspace
$ cargo install --path crates/markdown-edit-cli

# Build the doc toolkit from this workspace
$ cargo install --path crates/markdown-doc-cli
```

Or run the tools in place:

```console
# Extract
$ cargo run -p markdown-extract-cli -- <args>

# Edit
$ cargo run -p markdown-edit-cli -- <args>

# Doc toolkit
$ cargo run -p markdown-doc-cli -- <args>
```

### Docker

A Docker image is also available, and can be installed with the following
command:

```console
$ docker pull sean0x42/markdown-extract:v2
```

You can then run the container with the following command:

```console
$ docker run -it sean0x42/markdown-extract:v2 --help
```

Note that because markdown-extract accesses the file system, you will need
to mount a volume if you want to access a file on the host. e.g.

``` console
$ docker run -v $PWD:/opt -it sean0x42/markdown-extract:v2 v2.0.0 /opt/CHANGELOG.md
```

If you know a better way of achieving this, please let me know!

## Github Action

This project can be used as a Github action.

Here is a sample workflow usage:

```yaml
- id: extract-changelog
  uses: sean0x42/markdown-extract@v4
  with:
   file: crates/markdown-extract/CHANGELOG.md
   pattern: 'v2.0.0'
- name: Write output to file
  run: |
    printf '${{ steps.extract-changelog.outputs.markdown }}' > CHANGELOG-extracted.txt
- uses: actions/upload-artifact@v3
  with:
   name: changelog
   path: CHANGELOG-extracted.txt
```

For a complete reference view the [action.yml](action.yml).

The action version corresponds to the version of the tool.

## Use Cases

`markdown-extract` shines in several scenarios:

1. **Release automation**: Extract version-specific patch notes from `CHANGELOG.md` for CI/CD pipelines
2. **Documentation generation**: The HashiCorp team uses it to extract API docs and inject them into OpenAPI schemas
3. **AI agent context management**: Pre-filter documentation to keep LLM context windows lean ([see examples above](#for-ai-agent-workflows))
4. **Content validation**: Verify required sections exist in documentation before publishing
5. **Documentation diffing**: Compare sections across versions or files
6. **Selective archiving**: Extract and save specific documentation sections for compliance

## AI Agent Tooling

> **See detailed examples in the [For AI Agent Workflows](#for-ai-agent-workflows) section above.**

LLM-based agents work best with focused context. `markdown-extract` helps you:

- **Reduce token usage**: Extract only relevant sections instead of sending entire documents
- **Improve accuracy**: Targeted context reduces hallucination and improves response quality  
- **Enable dynamic prompts**: Build context-aware prompts by pulling sections based on user queries
- **Automate freshness**: Schedule extraction jobs to keep agent knowledge bases current

The tool writes clean Markdown to stdout, making it trivial to integrate with any orchestration framework, shell script, or Python automation that can run subprocesses.

**Real-world integration patterns:**
- GitHub Actions workflows that refresh agent context on doc updates
- Python scripts using `subprocess.run()` to dynamically fetch relevant sections
- Bash orchestration for multi-agent systems with specialized knowledge domains
- CI validation ensuring documentation completeness before agent deployment
