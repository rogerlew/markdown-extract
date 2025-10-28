"""Python bindings for the markdown-extract Rust library."""

from .markdown_extract_py import (
    MarkdownExtractError,
    Section,
    extract,
    extract_from_file,
    extract_sections,
    extract_sections_from_file,
)

__all__ = [
    "extract",
    "extract_from_file",
    "extract_sections",
    "extract_sections_from_file",
    "MarkdownExtractError",
    "Section",
]
