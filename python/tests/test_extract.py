import pytest

import markdown_extract_py as mde


def test_extract_basic():
    content = "# Title\nIntro\n## Section\nContent"
    result = mde.extract("Section", content)
    assert result == ["## Section\nContent"]


def test_extract_all_matches_no_heading():
    content = "# A\nBody\n# A\nMore"
    result = mde.extract("A", content, all_matches=True, no_heading=True)
    assert result == ["Body", "More"]


def test_extract_case_sensitive_flag():
    content = "# Intro\n## Details\nBody"
    assert mde.extract("details", content) == ["## Details\nBody"]
    assert mde.extract("details", content, case_sensitive=True) == []


def test_extract_from_file(tmp_path):
    md_file = tmp_path / "sample.md"
    md_file.write_text("# Intro\n## Install\nSteps")
    result = mde.extract_from_file("Install", md_file.as_posix())
    assert result == ["## Install\nSteps"]


def test_extract_from_file_missing(tmp_path):
    with pytest.raises(mde.MarkdownExtractError):
        mde.extract_from_file("pattern", (tmp_path / "missing.md").as_posix())


def test_extract_sections_returns_metadata():
    content = "# Intro\n## Details\nBody"
    sections = mde.extract_sections("Details", content)
    assert len(sections) == 1
    section = sections[0]
    assert section.heading == "## Details"
    assert section.level == 2
    assert section.body == "Body"
    assert section.full_text == "## Details\nBody"


def test_extract_sections_from_file(tmp_path):
    md_file = tmp_path / "sample.md"
    md_file.write_text("# Intro\n## Install\nSteps")
    sections = mde.extract_sections_from_file("Install", md_file.as_posix())
    assert len(sections) == 1
