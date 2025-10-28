# Python bindings

This directory hosts the Python packages produced by the PyO3 bindings. Each
crate can be installed with `maturin develop --manifest-path <crate>/Cargo.toml`
from the repository root.

## markdown-extract-py

Bindings for the `markdown-extract` Rust library.

```bash
# Build and install the extension in the active virtualenv
maturin develop --manifest-path ../crates/markdown_extract_py/Cargo.toml

python - <<'PY'
import markdown_extract_py as mde

text = """# Intro\ncontent\n## Details\nMore"""
print(mde.extract("Details", text))
PY
```

See `tests/test_extract.py` for more usage examples.

## markdown-edit-py

Bindings for the `markdown-edit` engine. Provide safe section editing without
invoking the CLI.

```bash
maturin develop --manifest-path ../crates/markdown_edit_py/Cargo.toml

python - <<'PY'
import pathlib
import markdown_edit_py as med

path = pathlib.Path("demo.md")
path.write_text("# Intro\nBody\n")
result = med.append_to(path.as_posix(), "Intro", "More text", dry_run=True)
print(result.diff)
PY
```

See `tests/test_edit.py` for additional scenarios (duplicate guard, payload
sources, max-matches handling).

## markdown-doc-py

Bindings for the `markdown-doc` TOC command.

```bash
maturin develop --manifest-path ../crates/markdown_doc_py/Cargo.toml

python - <<'PY'
import pathlib
import markdown_doc_py as mdoc

path = pathlib.Path("toc.md")
path.write_text("""<!-- toc -->\n- [Old](#old)\n<!-- tocstop -->\n\n# Intro\n""")
print(mdoc.toc(path.as_posix(), mode="diff").diff)
PY
```

See `tests/test_doc_toc.py` for TOC check/update/diff coverage.
