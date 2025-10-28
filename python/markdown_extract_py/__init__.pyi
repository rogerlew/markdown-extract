from typing import List

class MarkdownExtractError(Exception): ...

class Section:
    heading: str
    level: int
    title: str
    body: str
    full_text: str


def extract(
    pattern: str,
    content: str,
    *,
    case_sensitive: bool = ...,
    all_matches: bool = ...,
    no_heading: bool = ...,
) -> List[str]: ...


def extract_from_file(
    pattern: str,
    path: str,
    *,
    case_sensitive: bool = ...,
    all_matches: bool = ...,
    no_heading: bool = ...,
) -> List[str]: ...


def extract_sections(
    pattern: str,
    content: str,
    *,
    case_sensitive: bool = ...,
    all_matches: bool = ...,
) -> List[Section]: ...


def extract_sections_from_file(
    pattern: str,
    path: str,
    *,
    case_sensitive: bool = ...,
    all_matches: bool = ...,
) -> List[Section]: ...
