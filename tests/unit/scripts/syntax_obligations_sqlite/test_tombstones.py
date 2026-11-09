"""The obligation build fails closed: an issued id never leaves the root without a tombstone."""

from __future__ import annotations

from pathlib import Path

import pytest

from tests.unit.scripts.syntax_obligations_sqlite._test_types import TombstoneTestCase
from tests.unit.scripts.syntax_obligations_sqlite.helpers import (
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
            expected_unchanged=True,
        ),
        TombstoneTestCase(
            description="an obligation withdrawn with its tombstone",
            limits=SUPPRESS_POSITIVE,
            retired=RETIRE_POSITIVE,
            expected_error="",
            expected_present=False,
            expected_unchanged=False,
        ),
        TombstoneTestCase(
            description="a retired id the grammar still derives",
            limits="",
            retired=RETIRE_POSITIVE,
            expected_error=f"retired obligation ids issued again: ['{POSITIVE_ID}']",
            expected_present=True,
            expected_unchanged=True,
        ),
        TombstoneTestCase(
            description="nothing withdrawn and nothing retired",
            limits="",
            retired="",
            expected_error="",
            expected_present=True,
            expected_unchanged=True,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_withdrawn_or_retired_id_when_building_then_no_id_vanishes_or_returns(
    test_case: TombstoneTestCase, tmp_path: Path
) -> None:
    root: Path = small_root(directory=tmp_path, limits=test_case.limits, retired=test_case.retired)
    before: str = obligations_text(root)

    error: str = build_error(root)

    after: str = obligations_text(root)
    assert error == test_case.expected_error
    assert (f'"{POSITIVE_ID}"' in after) == test_case.expected_present
    assert (after == before) == test_case.expected_unchanged
