import pathlib

import markdown_doc_py as mdoc

def _write_file(tmp_path: pathlib.Path, name: str, content: str) -> pathlib.Path:
    path = tmp_path / name
    path.write_text(content)
    return path


def test_toc_check_clean(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    _write_file(
        tmp_path,
        "README.md",
        "<!-- toc -->\n<!-- tocstop -->\n\n# Intro\n",
    )
    result = mdoc.toc("README.md")
    assert result.status == "clean"


def test_toc_check_requires_update(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    _write_file(
        tmp_path,
        "doc.md",
        "<!-- toc -->\n- [Old](#old)\n<!-- tocstop -->\n\n# Intro\n",
    )
    result = mdoc.toc("doc.md")
    assert result.status == "changed"


def test_toc_update_writes_file(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    path = _write_file(
        tmp_path,
        "doc.md",
        "<!-- toc -->\n- [Old](#old)\n<!-- tocstop -->\n\n# Intro\n",
    )
    result = mdoc.toc("doc.md", mode="update")
    assert result.status == "changed"
    text = path.read_text()
    assert "# Intro" in text
    assert "Old" not in text


def test_toc_diff_returns_diff(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    _write_file(
        tmp_path,
        "doc.md",
        "<!-- toc -->\n- [Old](#old)\n<!-- tocstop -->\n\n# Intro\n",
    )
    result = mdoc.toc("doc.md", mode="diff")
    assert result.status == "changed"
    assert result.diff and "- [Old]" in result.diff


def test_toc_no_ignore(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    (tmp_path / ".markdown-doc-ignore").write_text("ignored.md\n")
    ignored = _write_file(
        tmp_path,
        "ignored.md",
        "<!-- toc -->\n- [Old](#old)\n<!-- tocstop -->\n\n# Intro\n",
    )
    result = mdoc.toc("ignored.md")
    assert result.status == "clean"

    result = mdoc.toc("ignored.md", mode="diff", no_ignore=True)
    assert result.status == "changed"
    assert result.diff and "- [Old]" in result.diff


def test_invalid_mode_raises(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    _write_file(
        tmp_path,
        "doc.md",
        "<!-- toc -->\n- [Old](#old)\n<!-- tocstop -->\n\n# Intro\n",
    )
    try:
        mdoc.toc("doc.md", mode="invalid")
    except mdoc.MarkdownDocError:
        pass
    else:
        raise AssertionError("expected MarkdownDocError")
