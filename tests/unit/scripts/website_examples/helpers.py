"""A small website, one example's inputs and a stand-in sqlquack, written into a directory."""

from __future__ import annotations

import io
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

from scripts.website_examples.main.examples import run_examples

STAND_IN: str = '#!/bin/sh\ncat "$(dirname "$0")/printed.txt"\n'


def checked_site(*, tmp_path: Path, pages: tuple[str, str], printed: str) -> tuple[int, str]:
    """The check's return code and messages for a docs page and a homepage over `printed`."""
    page, homepage = pages
    docs: Path = tmp_path / "website/src/content/docs/docs"
    docs.mkdir(parents=True)
    (docs / "page.mdx").write_text(page, encoding="utf-8")
    (tmp_path / "website/public").mkdir(parents=True)
    (tmp_path / "website/public/index.html").write_text(homepage, encoding="utf-8")
    example: Path = tmp_path / "website/examples/orders"
    example.mkdir(parents=True)
    (example / "query.sql").write_text("SELECT a\nFROM t\n", encoding="utf-8")
    tools: Path = tmp_path / "bin"
    tools.mkdir()
    (tools / "printed.txt").write_text(printed, encoding="utf-8")
    sqlquack: Path = tools / "sqlquack"
    sqlquack.write_text(STAND_IN)
    sqlquack.chmod(0o755)
    messages: io.StringIO = io.StringIO()
    with redirect_stderr(messages), redirect_stdout(messages):
        code: int = run_examples([str(sqlquack), "--root", str(tmp_path)])
    return code, messages.getvalue()
