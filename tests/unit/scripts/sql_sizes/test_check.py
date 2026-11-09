"""The generated-SQL size ratchet over small sizes files."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.sql_sizes.main.check import run_sql_size_check
from tests.unit.scripts.sql_sizes._test_types import BaselineTestCase, GrowthTestCase
from tests.unit.scripts.sql_sizes.helpers import sizes_file

_RULED_A: str = """
[[case]]
case = "a#1"
from = 100
to = 400
reason = "a guard the old SQL lacked"
ruling = "D1"
"""
_RULED_TOTAL: str = """
[[total]]
from = 5100
bytes = 90
reason = "longer guards"
ruling = "D2"
"""


@pytest.mark.parametrize(
    "test_case",
    (
        GrowthTestCase(
            description="shrinking passes",
            base={"a#1": 100, "b#1": 5000},
            measured={"a#1": 90, "b#1": 300},
            allowances="",
            expected_code=0,
            expected_fragment="passed",
        ),
        GrowthTestCase(
            description="a small case may grow 64 bytes, not 65, even when the total shrinks",
            base={"a#1": 100, "b#1": 5000},
            measured={"a#1": 165, "b#1": 300},
            allowances="",
            expected_code=1,
            expected_fragment="a#1 grew from 100 B to 165 B",
        ),
        GrowthTestCase(
            description="a small case growing 64 bytes passes",
            base={"a#1": 100, "b#1": 5000},
            measured={"a#1": 164, "b#1": 300},
            allowances="",
            expected_code=0,
            expected_fragment="passed",
        ),
        GrowthTestCase(
            description="a large case may grow 10%, but the shared total must not grow",
            base={"a#1": 100, "b#1": 5000},
            measured={"a#1": 100, "b#1": 5400},
            allowances="",
            expected_code=1,
            expected_fragment="grew from 5100 B to 5500 B",
        ),
        GrowthTestCase(
            description="a case new on this branch counts towards neither",
            base={"a#1": 100},
            measured={"a#1": 100, "new#1": 9000},
            allowances="",
            expected_code=0,
            expected_fragment="passed",
        ),
        GrowthTestCase(
            description="a ruled case may grow to its stated size, the total with it",
            base={"a#1": 100, "b#1": 5000},
            measured={"a#1": 400, "b#1": 5000},
            allowances=_RULED_A,
            expected_code=0,
            expected_fragment="passed",
        ),
        GrowthTestCase(
            description="a ruled case may not grow past its stated size",
            base={"a#1": 100, "b#1": 5000},
            measured={"a#1": 401, "b#1": 5000},
            allowances=_RULED_A,
            expected_code=1,
            expected_fragment="a#1 grew from 100 B to 401 B",
        ),
        GrowthTestCase(
            description="a ruling for another base size applies to nothing",
            base={"a#1": 120, "b#1": 5000},
            measured={"a#1": 400, "b#1": 5000},
            allowances=_RULED_A,
            expected_code=1,
            expected_fragment="applies to nothing",
        ),
        GrowthTestCase(
            description="a ruled total lets the shared cases grow that much",
            base={"a#1": 100, "b#1": 5000},
            measured={"a#1": 150, "b#1": 5040},
            allowances=_RULED_TOTAL,
            expected_code=0,
            expected_fragment="passed",
        ),
        GrowthTestCase(
            description="the same growth fails without its ruling",
            base={"a#1": 100, "b#1": 5000},
            measured={"a#1": 150, "b#1": 5040},
            allowances="",
            expected_code=1,
            expected_fragment="grew from 5100 B to 5190 B",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_base_sizes_when_checking_growth_then_only_growth_beyond_the_allowance_fails(
    tmp_path: Path, capsys: pytest.CaptureFixture[str], test_case: GrowthTestCase
) -> None:
    sizes: Path = sizes_file(tmp_path / "sizes.json", test_case.measured)
    baseline: Path = sizes_file(tmp_path / "baseline.json", test_case.measured)
    against: Path = sizes_file(tmp_path / "base.json", test_case.base)
    allowances: Path = tmp_path / "allowances.toml"
    allowances.write_text(test_case.allowances)
    code: int = run_sql_size_check(
        [str(sizes), "--baseline", str(baseline), "--against", str(against)]
        + ["--allowances", str(allowances)]
    )
    assert (code, test_case.expected_fragment in capsys.readouterr().err) == (
        test_case.expected_code,
        True,
    )


@pytest.mark.parametrize(
    "test_case",
    (
        BaselineTestCase(
            description="a smaller measure fails until the baseline is rewritten",
            committed={"a#1": 100},
            measured={"a#1": 90},
            expected_codes=(1, 0, 0),
            expected_fragment="make sql-size-baseline",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_stale_baseline_when_checking_then_it_fails_until_updated(
    tmp_path: Path, capsys: pytest.CaptureFixture[str], test_case: BaselineTestCase
) -> None:
    sizes: Path = sizes_file(tmp_path / "sizes.json", test_case.measured)
    baseline: Path = sizes_file(tmp_path / "baseline.json", test_case.committed)
    stale: int = run_sql_size_check([str(sizes), "--baseline", str(baseline)])
    message: str = capsys.readouterr().err
    updated: int = run_sql_size_check([str(sizes), "--baseline", str(baseline), "--update"])
    current: int = run_sql_size_check([str(sizes), "--baseline", str(baseline)])
    assert ((stale, updated, current), test_case.expected_fragment in message) == (
        test_case.expected_codes,
        True,
    )
