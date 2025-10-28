# Work Package: PyO3 Bindings for MCP Integration

**Package ID**: `20251028_pyo3_bindings`  
**Created**: 2025-10-28  
**Owner**: gpt-5-codex, roger
**Status**: Planning

---

## Overview

Expose the markdown-extract, markdown-edit, and markdown-doc Rust toolchain via Python bindings using PyO3, enabling integration with Model Context Protocol (MCP) servers and Python-based agent frameworks.

### Goals

1. **Python API Coverage**: Expose core functionality of all three tools through idiomatic Python interfaces
2. **MCP Server Templates**: Provide reference MCP server implementations for each tool
3. **Performance**: Maintain near-native performance by avoiding unnecessary serialization
4. **Distribution**: Package as installable Python wheels with embedded Rust binaries
5. **Documentation**: Complete API docs, usage examples, and migration guides from CLI

### Non-Goals

- Rewriting CLI functionality in Python (bindings wrap existing Rust code)
- Supporting Python < 3.8 (PyO3 minimum requirement)
- Full feature parity in initial release (focus on high-value operations)

---

## Deliverables

### Phase 1: Core Bindings (Weeks 1-2)

1. **`markdown_extract_py` crate**
   - `extract(pattern: str, content: str) -> List[str]`
   - `extract_from_file(pattern: str, path: str) -> List[str]`
   - Support for `case_sensitive`, `all_matches` flags
   - Return section strings or structured `Section` objects

2. **`markdown_edit_py` crate**
   - `replace(file: str, pattern: str, content: str, **options) -> EditResult`
   - `delete(file: str, pattern: str, **options) -> EditResult`
   - `append_to(file: str, pattern: str, content: str, **options) -> EditResult`
   - Dry-run support returning diffs

3. **`markdown_doc_py` crate**
   - `catalog(path: str = ".", **options) -> Catalog`
   - `lint(path: str = ".", **options) -> LintResults`
   - `validate(path: str, schema: Optional[str]) -> ValidationResults`
   - Structured return types (dataclasses/Pydantic models)

### Phase 2: MCP Server Shims (Week 3)

4. **MCP Tool Definitions**
   - JSON schema definitions for each tool/operation
   - Request/response validators
   - Error mapping (Rust errors → MCP error codes)

5. **Reference Servers**
   - `mcp-markdown-extract` - Read-only extraction server
   - `mcp-markdown-edit` - Safe editing server with confirmation prompts
   - `mcp-markdown-doc` - Repository analysis server
   - FastMCP or direct SSE/stdio implementation

### Phase 3: Distribution & Docs (Week 4)

6. **Build Infrastructure**
   - `maturin` build configuration for cross-platform wheels
   - CI workflows for Linux/macOS/Windows builds
   - PyPI release automation

7. **Documentation**
   - Python API reference (Sphinx/mkdocs)
   - MCP server deployment guides
   - Migration examples (CLI → Python API)
   - Performance benchmarks vs. CLI subprocess calls

8. **Testing**
   - Python unit tests covering all bindings
   - MCP server integration tests
   - Type stub generation (`.pyi` files)

---

## Technical Architecture

### PyO3 Binding Strategy

```
┌─────────────────────────────────────────┐
│   Python Application / MCP Server       │
│   (markdown_extract, markdown_doc, etc) │
└──────────────────┬──────────────────────┘
                   │ PyO3 FFI
┌──────────────────▼──────────────────────┐
│      Rust Bindings Crates                │
│  - markdown_extract_py                   │
│  - markdown_edit_py                      │
│  - markdown_doc_py                       │
└──────────────────┬──────────────────────┘
                   │ Direct Linking
┌──────────────────▼──────────────────────┐
│   Existing Rust Library Crates          │
│  - markdown-extract                      │
│  - markdown-edit-core                    │
│  - markdown-doc-ops                      │
└─────────────────────────────────────────┘
```


### Implementation Notes / Clarifications

- **Crate naming**: use `markdown_extract` / `markdown_edit_core` / `markdown_doc_ops` (underscores). Update Cargo dependencies accordingly.
- **Result shape**: core extract returns `Vec<Vec<String>>` (heading/body lines). Python bindings should join lines into a single string and manually handle cases where no heading is present (matching CLI behaviour).
- **Regex options**: `case_sensitive=False` should compile via `RegexBuilder` with `.case_insensitive(true)` and `.unicode(true)` to mirror CLI defaults.
- **Structured return (stretch)**: If exposing Section-like structs, mark them with `#[pyclass]` + `#[pyo3(get)]`, or return dicts. Dataclass behaviour isn’t automatic.
- **Type stubs**: `pyo3-stub-gen` is optional; store `.pyi` files under the package (e.g., `python/markdown_extract_py/__init__.pyi`) or use `maturin develop` to emit dist-info.
- **Testing**: Rust tests invoking PyO3 functions must acquire the GIL (`Python::with_gil`). Alternatively, add Python-side pytest cases calling the bindings.
- **Build settings**: Enable the `pyo3/extension-module` feature only for extension targets; set `PYO3_BUILT_CONFIG` or platform-specific env in CI (Windows/macOS) as needed.
- **Layout**: Place `pyproject.toml`, README, and package code under `python/` (e.g., `python/markdown_extract_py/`). Tests can live under that directory or a top-level `tests/` consumed by pytest.
- **Error mapping**: Wrap all I/O failures in `MarkdownExtractError` for a consistent contract and map regex compilation failures to `ValueError` with descriptive messages.

### MCP Integration Pattern

```python
# Example MCP tool using bindings
from mcp.server import Server
import markdown_extract_py as mde

@mcp_server.tool()
async def extract_section(file: str, pattern: str) -> str:
    """Extract markdown section matching pattern."""
    try:
        sections = mde.extract_from_file(pattern, file)
        return sections[0] if sections else ""
    except Exception as e:
        raise MCPError(f"Extraction failed: {e}")
```

### Type Mappings

| Rust Type | Python Type | Notes |
|-----------|-------------|-------|
| `String` | `str` | UTF-8 validated |
| `Vec<String>` | `List[str]` | Section results |
| `Result<T, E>` | Return T or raise exception | Map Rust errors to Python exceptions |
| `Option<T>` | `Optional[T]` | None for missing values |
| Structs | `@dataclass` or Pydantic models | Structured return types |

---

## Milestones & Timeline

| Milestone | Target Date | Deliverables |
|-----------|-------------|--------------|
| **M1 – Extract Bindings** | 2025-11-04 | `markdown_extract_py` crate, basic tests, type stubs |
| **M2 – Edit/Doc Bindings** | 2025-11-11 | `markdown_edit_py`, `markdown_doc_py` crates, integration tests |
| **M3 – MCP Servers** | 2025-11-18 | Three reference MCP servers, deployment docs |
| **M4 – Release** | 2025-11-25 | PyPI packages, CI/CD, comprehensive docs |

---

## Dependencies

- **PyO3** `>= 0.20` - Rust ↔ Python bindings framework
- **maturin** - Build tool for Python packages with Rust extensions
- **MCP SDK** - Model Context Protocol server framework (FastMCP or official SDK)
- **Pydantic** `>= 2.0` (optional) - Runtime type validation for structured returns

---

## Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| PyO3 version compatibility | Medium | Pin to stable PyO3 version, test across Python 3.8-3.12 |
| Cross-platform build complexity | Medium | Use maturin + cibuildwheel in CI, test on all platforms early |
| Performance overhead from Python ↔ Rust crossing | Low | Benchmark against CLI subprocess calls; optimize hot paths |
| MCP protocol changes | Low | Use versioned MCP SDK, document supported protocol versions |
| Type stub accuracy | Low | Generate from PyO3 annotations, validate with mypy/pyright |

---

## Open Questions

1. **Error handling philosophy**: Raise exceptions vs. return Result-like types in Python?
   - **Proposal**: Use exceptions for errors, match Python conventions
   
2. **Async support**: Should bindings expose async interfaces for I/O operations?
   - **Proposal**: Start with sync APIs, add async wrappers in Phase 2 if needed

3. **Configuration loading**: Expose Rust config system or use Python-native TOML libs?
   - **Proposal**: Wrap Rust config loader for consistency with CLI behavior

4. **Wheel distribution strategy**: Pure wheels + platform wheels, or platform-only?
   - **Proposal**: Platform wheels only (include compiled Rust code)

---

## Success Criteria

- [ ] Python bindings cover ≥80% of CLI functionality
- [ ] MCP servers can be deployed via `pip install mcp-markdown-*`
- [ ] Performance within 10% of direct CLI invocation
- [ ] Type stubs pass `mypy --strict` validation
- [ ] Documentation includes end-to-end MCP integration example
- [ ] CI builds wheels for Linux (x86_64, aarch64), macOS (universal2), Windows (x64)

---

## References

- [PyO3 User Guide](https://pyo3.rs/)
- [maturin Documentation](https://www.maturin.rs/)
- [Model Context Protocol Specification](https://modelcontextprotocol.io/)
- [markdown-extract README](../../README.md)
- Related work package: [20251025_markdown_doc_toolkit](../20251025_markdown_doc_toolkit/)
