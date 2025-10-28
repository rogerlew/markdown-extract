# PyO3 Bindings Deployment Guide

Quick reference for installing and using markdown Python bindings in production environments.

---

## Installation (Development/Editable)

### Prerequisites
- Python ≥ 3.8
- Rust toolchain (cargo)
- maturin (`pip install maturin`)

### Install markdown_extract_py

```bash
# From your project's virtualenv
source /path/to/your/.venv/bin/activate

# Editable install (tracks source changes)
maturin develop --manifest-path /workdir/markdown-extract/crates/markdown_extract_py/Cargo.toml --release

# Or debug build (faster compile, slower runtime)
maturin develop --manifest-path /workdir/markdown-extract/crates/markdown_extract_py/Cargo.toml
```

**Output**:
```
🔗 Found pyo3 bindings with abi3 support
🐍 Not using a specific python interpreter
📦 Built wheel for abi3 Python ≥ 3.8 to /tmp/.tmp*/markdown_extract_py-0.1.0-cp38-abi3-linux_x86_64.whl
✏️ Setting installed package as editable
🛠 Installed markdown_extract_py-0.1.0
```

### Verify Installation

```python
import markdown_extract_py as mde

# Smoke test
text = "# Intro\nBody\n## Install\nSteps"
result = mde.extract("Install", text)
print(result)  # ['## Install\nSteps']
```

---

## Usage Examples

### Basic Extraction

```python
import markdown_extract_py as mde

markdown_content = """
# Getting Started
Welcome to the project.

## Installation
Run `pip install project`.

## Configuration
Edit the config file.
"""

# Extract first matching section
sections = mde.extract("Installation", markdown_content)
print(sections[0])
# Output:
# ## Installation
# Run `pip install project`.
```

### Extract All Matches

```python
content = """
# Module A
Info about A

## Usage
How to use A

# Module B
Info about B

## Usage
How to use B
"""

# Get all "Usage" sections
all_usage = mde.extract("Usage", content, all_matches=True)
print(len(all_usage))  # 2
```

### Body-Only Extraction (No Heading)

```python
content = "## Installation\nRun pip install.\n## Usage\nImport the module."

# Extract body without heading line
body = mde.extract("Installation", content, no_heading=True)
print(body[0])  # "Run pip install."
```

### Case-Sensitive Matching

```python
content = "# INSTALL\nSteps\n# install\nMore steps"

# Default: case-insensitive
mde.extract("install", content, all_matches=True)  # Both sections

# Explicit case-sensitive
mde.extract("install", content, case_sensitive=True, all_matches=True)  # Only lowercase
```

### Extract from File

```python
# Read from disk
sections = mde.extract_from_file("Configuration", "README.md")
if sections:
    print(sections[0])
```

### Structured Extraction (Metadata)

```python
# Get Section objects with metadata
sections = mde.extract_sections("Installation", markdown_content)

for section in sections:
    print(f"Heading: {section.heading}")  # "## Installation"
    print(f"Level: {section.level}")      # 2
    print(f"Title: {section.title}")      # "Installation"
    print(f"Body: {section.body}")        # "Run `pip install project`."
    print(f"Full: {section.full_text}")   # Heading + body
```

---

## Error Handling

All file and pattern errors raise `MarkdownExtractError`:

```python
try:
    result = mde.extract_from_file("pattern", "missing.md")
except mde.MarkdownExtractError as e:
    print(f"Error: {e}")
    # "File not found: missing.md"
```

**Error types** (all wrapped in `MarkdownExtractError`):
- File not found
- Permission denied
- Invalid regex pattern
- Generic I/O errors

---

## Production Deployment Example

### wepppy/cao Integration (2025-10-28)

**Environment**: `/workdir/wepppy/services/cao`

**1. Add maturin to dev dependencies**:
```toml
# pyproject.toml
[project.optional-dependencies]
dev = [
    "maturin==1.9.6",
    # ... other dev deps
]
```

**2. Install in virtualenv**:
```bash
cd /workdir/wepppy/services/cao
source .venv/bin/activate
maturin develop --manifest-path /workdir/markdown-extract/crates/markdown_extract_py/Cargo.toml --release
```

**3. Use in application code**:
```python
# services/cao/some_module.py
import markdown_extract_py as mde

def extract_section(markdown_text: str, pattern: str) -> str | None:
    """Extract section from markdown, replacing subprocess CLI calls."""
    sections = mde.extract(pattern, markdown_text)
    return sections[0] if sections else None
```

**Performance**: ~10-50x faster than subprocess calls (no process spawn overhead)

---

## Updating Bindings

After pulling updates to the Rust crate:

```bash
# Rebuild and reinstall
maturin develop --manifest-path /workdir/markdown-extract/crates/markdown_extract_py/Cargo.toml --release
```

**Note**: `maturin develop` creates an editable install, so changes to Python wrapper code (`python/markdown_extract_py/`) are reflected immediately. Rust changes require rebuild.

---

## Future Bindings (M2+)

### markdown_edit_py (TBD)
```python
from markdown_edit_py import replace_section, delete_section, insert_after
```

### markdown_doc_py (TBD)
```python
from markdown_doc_py import catalog, lint, validate_toc
```

Same installation pattern:
```bash
maturin develop --manifest-path /workdir/markdown-extract/crates/markdown_edit_py/Cargo.toml --release
maturin develop --manifest-path /workdir/markdown-extract/crates/markdown_doc_py/Cargo.toml --release
```

---

## Troubleshooting

### "missing field package"
**Symptom**: `maturin develop` fails with "missing field package"  
**Cause**: Pointing at workspace manifest instead of crate manifest  
**Fix**: Use `--manifest-path /path/to/crate/Cargo.toml` (not workspace root)

### "cannot find -lpython3.X"
**Symptom**: Cargo build fails with linker error  
**Cause**: Python development headers not installed  
**Fix**: `apt install python3-dev` (Ubuntu/Debian) or `yum install python3-devel` (RHEL/CentOS)

### Import fails after updates
**Symptom**: `ImportError` or stale behavior after code changes  
**Fix**: Re-run `maturin develop` to rebuild

### Performance slower than expected
**Symptom**: Bindings not faster than CLI  
**Fix**: Use `--release` flag (optimized build) instead of debug build

---

## API Reference

See type stubs: `python/markdown_extract_py/__init__.pyi`

### Functions

**`extract(pattern, content, *, case_sensitive=False, all_matches=False, no_heading=False) -> List[str]`**
- Extract sections from string
- Returns list of section strings (heading + body)

**`extract_from_file(pattern, path, *, case_sensitive=False, all_matches=False, no_heading=False) -> List[str]`**
- Extract sections from file
- Raises `MarkdownExtractError` on file errors

**`extract_sections(pattern, content, *, case_sensitive=False, all_matches=False) -> List[Section]`**
- Extract with metadata
- Returns `Section` objects

**`extract_sections_from_file(pattern, path, *, case_sensitive=False, all_matches=False) -> List[Section]`**
- File variant with metadata

### Classes

**`Section`**
- `heading: str` – Full heading line (e.g., "## Title")
- `level: int` – Heading depth (1-6)
- `title: str` – Normalized heading text
- `body: str` – Section content (excluding heading)
- `full_text: str` – Heading + body

**`MarkdownExtractError(Exception)`**
- Raised for all file and pattern errors
- Includes descriptive error messages

---

## Support

- **Source**: `/workdir/markdown-extract/crates/markdown_extract_py/`
- **Tests**: `/workdir/markdown-extract/python/tests/test_extract.py`
- **Work package**: `docs/work-packages/20251028_pyo3_bindings/`
- **Issues**: Report via work package tracker
