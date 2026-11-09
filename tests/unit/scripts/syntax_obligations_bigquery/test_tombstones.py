"""The obligation build fails closed: it refuses a vanished or reissued id before any write."""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.unit.scripts.syntax_obligations_bigquery._test_types import TombstoneTestCase
from tests.unit.scripts.syntax_obligations_bigquery.helpers import (
    MISSING_PARSER,
    POSITIVE_ID,
    RETIRE_POSITIVE,
    SUPPRESS_POSITIVE,
    build_error,
    obligations_text,
    small_root,
)


@pytest.mark.parametrize(
    "test_case",
    (
        TombstoneTestCase(
            description="an obligation withdrawn without a tombstone",
            limits=SUPPRESS_POSITIVE,
            retired="",
            expected_error=(
                f"obligation ids would leave the root without a tombstone: ['{POSITIVE_ID}']"
            ),
            expected_present=True,
        ),
        TombstoneTestCase(
            description="a retired id the grammar still derives",
            limits="",
            retired=RETIRE_POSITIVE,
            expected_error=f"retired obligation ids issued again: ['{POSITIVE_ID}']",
            expected_present=True,
        ),
        TombstoneTestCase(
            description="an obligation withdrawn with its tombstone",
            limits=SUPPRESS_POSITIVE,
            retired=RETIRE_POSITIVE,
            expected_error="no parser at {parser}",
            expected_present=True,
        ),
        TombstoneTestCase(
            description="nothing withdrawn and nothing retired",
            limits="",
            retired="",
            expected_error="no parser at {parser}",
            expected_present=True,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_withdrawn_or_retired_id_when_building_then_it_refuses_before_writing(
    test_case: TombstoneTestCase, tmp_path: Path
) -> None:
    root: Path = small_root(directory=tmp_path, limits=test_case.limits, retired=test_case.retired)
    parser: Path = tmp_path / MISSING_PARSER
    before: str = obligations_text(root)

    error: str = build_error(root=root, parser=parser)
    second: str = build_error(root=root, parser=parser)

    after: str = obligations_text(root)
    assert error == test_case.expected_error.format(parser=parser)
    assert second == error
    assert (f'"{POSITIVE_ID}"' in after) == test_case.expected_present
    assert after == before
