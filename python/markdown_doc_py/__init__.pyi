from typing import List, Optional

class MarkdownDocError(Exception): ...

class TocResult:
    mode: str
    status: str
    diff: Optional[str]
    messages: List[str]


def toc(
    path: str,
    *,
    mode: str = ...,
    no_ignore: bool = ...,
    quiet: bool = ...,
) -> TocResult: ...
