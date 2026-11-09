"""The build's --check fails when a committed file differs from what the build writes."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from tests.unit.scripts.syntax_obligations_sqlite._test_types import CheckTestCase, StrayTestCase
from tests.unit.scripts.syntax_obligations_sqlite.helpers import (
    FAMILY,
    build_error,
    build_process,
    small_root,
)


@pytest.mark.parametrize(
    "test_case",
    (
        CheckTestCase(
            description="committed files as the build writes them",
            relative=f"cases/{FAMILY}.jsonl",
            old="",
            new="",
            expected_code=0,
            expected_stale=0,
        ),
        CheckTestCase(
            description="one committed case with its expectation edited",
            relative=f"cases/{FAMILY}.jsonl",
            old='"expect":"reject"',
            new='"expect":"accept"',
            expected_code=1,
            expected_stale=1,
        ),
        CheckTestCase(
            description="one committed obligation made optional",
            relative=f"obligations/{FAMILY}.toml",
            old="required = true",
            new="required = false",
            expected_code=1,
            expected_stale=1,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_committed_files_when_checking_then_only_an_edit_fails_and_nothing_is_written(
    test_case: CheckTestCase, tmp_path: Path
) -> None:
    root: Path = small_root(directory=tmp_path, limits="", retired="")
    assert build_error(root) == ""
    path: Path = root / test_case.relative
    original: str = path.read_text()
    edited: str = original.replace(test_case.old, test_case.new, 1)
    path.write_text(edited)

    checked: subprocess.CompletedProcess[str] = build_process(["--root", str(root), "--check"])

    assert checked.returncode == test_case.expected_code
    assert checked.stderr.count("stale: ") == test_case.expected_stale
    assert path.read_text() == edited


@pytest.mark.parametrize(
    "test_case",
    (
        StrayTestCase(
            description="a cases file of no family", relative="cases/stray.jsonl", expected_code=1
        ),
        StrayTestCase(
            description="an obligations file of no family",
            relative="obligations/stray.toml",
            expected_code=1,
        ),
        StrayTestCase(
            description="a file nested under the cases directory",
            relative="cases/old/query.jsonl",
            expected_code=1,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_stray_committed_file_when_checking_then_the_check_names_it_and_fails(
    test_case: StrayTestCase, tmp_path: Path
) -> None:
    root: Path = small_root(directory=tmp_path, limits="", retired="")
    assert build_error(root) == ""
    stray: Path = root / test_case.relative
    stray.parent.mkdir(parents=True, exist_ok=True)
    stray.write_text("")

    checked: subprocess.CompletedProcess[str] = build_process(["--root", str(root), "--check"])

    assert checked.returncode == test_case.expected_code
    assert checked.stderr.splitlines() == [
        f"stale: {stray} is missing, differs, or is not written by the build"
    ]
    assert stray.read_text() == ""
