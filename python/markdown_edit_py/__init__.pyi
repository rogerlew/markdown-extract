from typing import List, Optional

class MarkdownEditError(Exception): ...

class EditResult:
    applied: bool
    exit_code: int
    diff: Optional[str]
    messages: List[str]
    written_path: Optional[str]


def replace(
    file: str,
    pattern: str,
    replacement: str,
    *,
    case_sensitive: bool = ...,
    all_matches: bool = ...,
    body_only: bool = ...,
    keep_heading: bool = ...,
    allow_duplicate: bool = ...,
    max_matches: Optional[int] = ...,
    dry_run: bool = ...,
    backup: bool = ...,
    with_path: Optional[str] = ...,
    with_string: Optional[str] = ...,
) -> EditResult: ...


def delete(
    file: str,
    pattern: str,
    *,
    case_sensitive: bool = ...,
    all_matches: bool = ...,
    allow_duplicate: bool = ...,
    max_matches: Optional[int] = ...,
    dry_run: bool = ...,
    backup: bool = ...,
) -> EditResult: ...


def append_to(
    file: str,
    pattern: str,
    payload: str,
    *,
    case_sensitive: bool = ...,
    all_matches: bool = ...,
    allow_duplicate: bool = ...,
    max_matches: Optional[int] = ...,
    dry_run: bool = ...,
    backup: bool = ...,
    with_path: Optional[str] = ...,
    with_string: Optional[str] = ...,
) -> EditResult: ...


def prepend_to(
    file: str,
    pattern: str,
    payload: str,
    *,
    case_sensitive: bool = ...,
    all_matches: bool = ...,
    allow_duplicate: bool = ...,
    max_matches: Optional[int] = ...,
    dry_run: bool = ...,
    backup: bool = ...,
    with_path: Optional[str] = ...,
    with_string: Optional[str] = ...,
) -> EditResult: ...


def insert_after(
    file: str,
    pattern: str,
    payload: str,
    *,
    case_sensitive: bool = ...,
    all_matches: bool = ...,
    allow_duplicate: bool = ...,
    max_matches: Optional[int] = ...,
    dry_run: bool = ...,
    backup: bool = ...,
    with_path: Optional[str] = ...,
    with_string: Optional[str] = ...,
) -> EditResult: ...


def insert_before(
    file: str,
    pattern: str,
    payload: str,
    *,
    case_sensitive: bool = ...,
    all_matches: bool = ...,
    allow_duplicate: bool = ...,
    max_matches: Optional[int] = ...,
    dry_run: bool = ...,
    backup: bool = ...,
    with_path: Optional[str] = ...,
    with_string: Optional[str] = ...,
) -> EditResult: ...
