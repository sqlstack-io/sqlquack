"""Comparing two runs, each a receipt file or a directory of one run's receipt files per batch."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.databricks_receipts.main.run import run_referee
from tests.unit.scripts.databricks_receipts._test_types import CompareTestCase
from tests.unit.scripts.databricks_receipts.helpers import write_run


@pytest.mark.parametrize(
    "test_case",
    [
        CompareTestCase(
            description="same phases",
            second_phase="semantic_error",
            expected_exit=0,
            expected_line="3 and 3 receipts; 0 differ",
        ),
        CompareTestCase(
            description="one phase differs",
            second_phase="syntax_error",
            expected_exit=1,
            expected_line="3 and 3 receipts; 1 differ",
        ),
    ],
    ids=lambda case: case.description,
)
def test_given_two_run_directories_when_compared_then_every_batch_file_is_read(
    tmp_path: Path, capsys: pytest.CaptureFixture[str], test_case: CompareTestCase
) -> None:
    write_run(
        tmp_path / "out",
        {"a": [("c1", "accepted"), ("c2", "syntax_error")], "b": [("c3", "semantic_error")]},
    )
    write_run(
        tmp_path / "rep",
        {"x": [("c3", test_case.second_phase)], "y": [("c2", "syntax_error"), ("c1", "accepted")]},
    )

    exit_code: int = run_referee(["compare", str(tmp_path / "out"), str(tmp_path / "rep")])

    assert exit_code == test_case.expected_exit
    assert test_case.expected_line in capsys.readouterr().out
