"""Comparing two runs, each a receipt file or a directory of one run's receipt files per batch."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from scripts.databricks_receipts.main.run import run_referee


def _write_run(directory: Path, batches: dict[str, list[tuple[str, str]]]) -> None:
    """One `.jsonl` per batch, each line a receipt with the compared keys, plus a report file."""
    directory.mkdir()
    for name, receipts in batches.items():
        lines: list[str] = [
            json.dumps(
                {
                    "case_id": case_id,
                    "phase": phase,
                    "engine_code": None,
                    "parse_only_sqlstate": None,
                    "capture_method": "execute",
                }
            )
            for case_id, phase in receipts
        ]
        (directory / f"{name}.jsonl").write_text("\n".join(lines) + "\n")
        (directory / f"{name}.report.json").write_text("{}\n")


@pytest.mark.parametrize(
    ("second_phase", "expected_exit", "expected_line"),
    [
        ("semantic_error", 0, "3 and 3 receipts; 0 differ"),
        ("syntax_error", 1, "3 and 3 receipts; 1 differ"),
    ],
)
def test_given_two_run_directories_when_compared_then_every_batch_file_is_read(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
    second_phase: str,
    expected_exit: int,
    expected_line: str,
) -> None:
    _write_run(
        tmp_path / "out",
        {"a": [("c1", "accepted"), ("c2", "syntax_error")], "b": [("c3", "semantic_error")]},
    )
    _write_run(
        tmp_path / "rep",
        {"x": [("c3", second_phase)], "y": [("c2", "syntax_error"), ("c1", "accepted")]},
    )

    exit_code: int = run_referee(["compare", str(tmp_path / "out"), str(tmp_path / "rep")])

    assert exit_code == expected_exit
    assert expected_line in capsys.readouterr().out
