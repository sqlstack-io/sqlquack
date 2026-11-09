"""The polyglot guard against synthetic dependency graphs."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.polyglot_guard.main.check import run_polyglot_guard
from tests.unit.scripts.polyglot_guard._test_types import GuardTestCase
from tests.unit.scripts.polyglot_guard.helpers import write_metadata


@pytest.mark.parametrize(
    "test_case",
    (
        GuardTestCase(
            description="the check crate may depend on polyglot",
            members=("sqlquack-polyglot-check",),
            edges=(("sqlquack-polyglot-check", "polyglot-sql", None),),
            expected_return_code=0,
            expected_fragment="passed, 1 crates reach polyglot-sql",
        ),
        GuardTestCase(
            description="a dev-dependency does not count",
            members=("sqlquack-syntax",),
            edges=(("sqlquack-syntax", "polyglot-sql", "dev"),),
            expected_return_code=0,
            expected_fragment="passed, 0 crates",
        ),
        GuardTestCase(
            description="the syntax crate must never reach polyglot",
            members=("sqlquack-syntax",),
            edges=(("sqlquack-syntax", "polyglot-sql", None),),
            expected_return_code=1,
            expected_fragment="sqlquack-syntax depends on polyglot-sql",
        ),
        GuardTestCase(
            description="the core crate no longer reaches polyglot",
            members=("sqlquack-core",),
            edges=(("sqlquack-core", "polyglot-sql", None),),
            expected_return_code=1,
            expected_fragment="sqlquack-core depends on polyglot-sql",
        ),
        GuardTestCase(
            description="a transitive runtime path counts",
            members=("sqlquack-policy",),
            edges=(("sqlquack-policy", "middle", None), ("middle", "polyglot-sql", None)),
            expected_return_code=1,
            expected_fragment="sqlquack-policy depends on polyglot-sql",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_dependency_graph_when_guarding_then_only_allowed_crates_reach_polyglot(
    test_case: GuardTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    metadata: Path = write_metadata(
        directory=tmp_path, members=test_case.members, edges=test_case.edges
    )
    return_code: int = run_polyglot_guard(["--metadata", str(metadata)])
    assert return_code == test_case.expected_return_code
    assert test_case.expected_fragment in capsys.readouterr().err
