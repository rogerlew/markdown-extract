"""Python bindings for markdown-doc TOC operations."""

from .markdown_doc_py import MarkdownDocError, TocResult, toc

__all__ = ["toc", "TocResult", "MarkdownDocError"]
