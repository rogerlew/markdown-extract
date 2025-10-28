import pathlib

import pytest

import markdown_edit_py as med

def _write_file(tmp_path: pathlib.Path, name: str, content: str) -> pathlib.Path:
    path = tmp_path / name
    path.write_text(content)
    return path


def test_replace_dry_run(tmp_path):
    path = _write_file(tmp_path, "doc.md", "# Title\nBody\n")
    result = med.replace(
        path.as_posix(),
        "Title",
        "# Title\nUpdated\n",
        dry_run=True,
    )
    assert result.applied is False
    assert result.diff and "Updated" in result.diff
    assert result.exit_code == 0


def test_delete_section(tmp_path):
    path = _write_file(tmp_path, "doc.md", "# A\nBody\n# B\nMore\n")
    outcome = med.delete(path.as_posix(), "B")
    assert outcome.applied is True
    assert outcome.written_path == path.as_posix()


def test_append_duplicate_guard(tmp_path):
    path = _write_file(tmp_path, "doc.md", "# A\nBody\n")
    # First append applies
    first = med.append_to(path.as_posix(), "A", "Extra")
    assert first.applied is True
    # Second append skipped because allow_duplicate is False (default)
    second = med.append_to(path.as_posix(), "A", "Extra")
    assert second.applied is False


def test_insert_after_all_matches(tmp_path):
    path = _write_file(tmp_path, "doc.md", "# A\n--\n# A\n--\n")
    with pytest.raises(med.MarkdownEditError):
        med.insert_after(path.as_posix(), "A", "x")

    result = med.insert_after(
        path.as_posix(),
        "A",
        "# A.1\nx",
        all_matches=True,
        allow_duplicate=True,
    )
    assert result.applied is True


def test_replace_with_file_payload(tmp_path):
    source = _write_file(tmp_path, "payload.txt", "Replacement")
    target = _write_file(tmp_path, "doc.md", "# H\nBody\n")
    outcome = med.replace(
        target.as_posix(),
        "H",
        "",
        keep_heading=True,
        with_path=source.as_posix(),
    )
    assert outcome.applied is True


def test_replace_no_match_raises(tmp_path):
    path = _write_file(tmp_path, "doc.md", "# H\nBody\n")
    with pytest.raises(med.MarkdownEditError):
        med.replace(path.as_posix(), "Missing", "text")
