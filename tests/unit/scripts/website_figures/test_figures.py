"""Writing the website's syntax figures from a coverage summary, then checking them against it."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from scripts.website_figures.main.figures import run_figures
from tests.unit.scripts.website_figures._test_types import CheckTestCase, WriteTestCase
from tests.unit.scripts.website_figures.helpers import written_files


@pytest.mark.parametrize(
    "test_case",
    (
        WriteTestCase(
            description="the summary replaces the syntax figures and other figures stay",
            day="2026-11-16",
            expected_figures={
                "SYNTAX_VERIFIED": "3,199",
                "SYNTAX_FORMS": "3,351",
                "SYNTAX_PCT": "95.46%",
                "SYNTAX_BASIS": (
                    "3,199 forms verified against cases recorded on Snowflake and approved "
                    "reviews, as of 16 November 2026."
                ),
                "DUCKSHOP_MODELS": "1,000",
            },
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_stale_figures_when_writing_then_the_summary_figures_replace_them(
    test_case: WriteTestCase, tmp_path: Path
) -> None:
    _, figures = written_files(tmp_path=tmp_path, day=test_case.day)
    assert json.loads(figures.read_text()) == test_case.expected_figures


@pytest.mark.parametrize(
    "test_case",
    (
        CheckTestCase(
            description="freshly written figures pass",
            edits={},
            expected_return_code=0,
            expected_fragment="",
        ),
        CheckTestCase(
            description="a stale share fails",
            edits={"SYNTAX_PCT": "71.26%"},
            expected_return_code=1,
            expected_fragment="SYNTAX_PCT is '71.26%', but the summary gives '95.46%'",
        ),
        CheckTestCase(
            description="a basis that states an older count fails",
            edits={"SYNTAX_BASIS": "2,388 forms verified, as of 6 November 2026."},
            expected_return_code=1,
            expected_fragment="SYNTAX_BASIS does not start with the verified count 3,199",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_written_figures_when_checking_then_only_a_mismatch_fails(
    test_case: CheckTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    summary, figures = written_files(tmp_path=tmp_path, day="2026-11-16")
    edited: dict[str, Any] = {**json.loads(figures.read_text()), **test_case.edits}
    figures.write_text(json.dumps(edited))
    capsys.readouterr()
    return_code: int = run_figures(["check", str(summary), "--figures", str(figures)])
    assert (return_code, test_case.expected_fragment in capsys.readouterr().err) == (
        test_case.expected_return_code,
        True,
    )
