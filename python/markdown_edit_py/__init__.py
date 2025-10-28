"""Python bindings for the markdown-edit Rust engine."""

from .markdown_edit_py import (
    EditResult,
    MarkdownEditError,
    append_to,
    delete,
    insert_after,
    insert_before,
    prepend_to,
    replace,
)

__all__ = [
    "replace",
    "delete",
    "append_to",
    "prepend_to",
    "insert_after",
    "insert_before",
    "EditResult",
    "MarkdownEditError",
]
