"""Guard raises counted over a Parquet sample, with reasons and redacted shapes only."""

from __future__ import annotations

import re
from pathlib import Path

import pytest

from scripts.guard_measurement._helpers.reasons import number_detail
from tests.unit.scripts.guard_measurement._test_types import (
    CountTestCase,
    MeasureTestCase,
    NumberDetailTestCase,
    RedactionTestCase,
)
from tests.unit.scripts.guard_measurement.helpers import measure

_MANY_RAISING: tuple[str, ...] = (
    tuple(f"bad {index}" for index in range(200))
    + tuple(f"err {index}" for index in range(10))
    + ("ok",) * 290
)
_PROGRESS_LINE: re.Pattern[str] = re.compile(r"guard measurement: \[1/1\][^\n]*")
_NUMBER_REASON: str = (
    "json_fidelity: JSON with a number, word or comma DuckDB may read differently from Snowflake"
)


@pytest.mark.parametrize(
    "test_case",
    (
        MeasureTestCase(
            description="repeated raising values count once per sample row, by reason",
            values=("ok", "bad 1", "bad 1", "err 7", None, "ok", "bad 22") + ("ok",) * 40,
            expression="TAG(x1)",
            declared_type="VARCHAR",
            expected_status="measured",
            expected_raising_rows=4,
            expected_reasons=(
                ("json_fidelity", "json_fidelity: a value DuckDB may read differently", 3),
                ("snowflake_error", "Snowflake error 100035: Timestamp is not recognized", 1),
            ),
        ),
        MeasureTestCase(
            description="number raises are narrowed by the offending token",
            values=('{"a": 1e5}', '{"a": 0.12345678901234567}', '{"a": 1}'),
            expression="NUMBERS(x1)",
            declared_type="VARCHAR",
            expected_status="measured",
            expected_raising_rows=2,
            expected_reasons=(
                ("json_fidelity", f"{_NUMBER_REASON} (17 significant digits)", 1),
                ("json_fidelity", f"{_NUMBER_REASON} (exponent)", 1),
            ),
        ),
        MeasureTestCase(
            description="a probe SQLQuack refuses is reported, not measured",
            values=("ok",),
            expression="UNKNOWN_CALL(x1)",
            declared_type="VARCHAR",
            expected_status="refused",
            expected_raising_rows=0,
            expected_reasons=(),
        ),
        MeasureTestCase(
            description="an argument of unknown type is taken from the sample's recorded types",
            values=("bad 1", "ok"),
            expression="TAG(x1)",
            declared_type="UNKNOWN",
            expected_status="measured",
            expected_raising_rows=1,
            expected_reasons=(
                ("json_fidelity", "json_fidelity: a value DuckDB may read differently", 1),
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_sample_when_measuring_a_probe_then_raising_rows_and_reasons_are_counted(
    test_case: MeasureTestCase, tmp_path: Path
) -> None:
    document: dict[str, object] = measure(
        tmp_path, test_case.values, test_case.expression, test_case.declared_type
    )
    probes: object = document["probes"]
    assert isinstance(probes, list)
    probe: dict[str, object] = probes[0]
    reasons: object = probe["reasons"]
    assert isinstance(reasons, list)
    assert (
        probe["status"],
        probe["raising_rows"],
        tuple((entry["divergence"], entry["reason"], entry["rows"]) for entry in reasons),
    ) == (
        test_case.expected_status,
        test_case.expected_raising_rows,
        test_case.expected_reasons,
    )


@pytest.mark.parametrize(
    "test_case",
    (
        RedactionTestCase(
            description="a raising value appears only as its shape",
            values=("bad secret-sku-4417", "ok"),
            expected_absent=("secret", "4417"),
            expected_shape="aaa aaaaaa-aaa-9999",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_raising_values_when_reporting_then_no_sampled_value_is_written(
    test_case: RedactionTestCase, tmp_path: Path
) -> None:
    measure(tmp_path, test_case.values, "TAG(x1)", "VARCHAR")

    written: str = (tmp_path / "report.json").read_text(encoding="utf-8")
    assert (
        tuple(fragment in written for fragment in test_case.expected_absent),
        test_case.expected_shape in written,
    ) == ((False,) * len(test_case.expected_absent), True)


@pytest.mark.parametrize(
    "test_case",
    (
        NumberDetailTestCase(
            description="an exponent",
            text='{"a": -2.5E+10}',
            expected_reason=f"{_NUMBER_REASON} (exponent)",
            expected_shape="-9.9a+99",
        ),
        NumberDetailTestCase(
            description="a fraction of zeros",
            text='{"a": 3.000, "b": "1e5"}',
            expected_reason=f"{_NUMBER_REASON} (fraction of zeros)",
            expected_shape="9.999",
        ),
        NumberDetailTestCase(
            description="a bare word outside strings",
            text='{"a": NaN}',
            expected_reason=f"{_NUMBER_REASON} (bare word)",
            expected_shape="aaa",
        ),
        NumberDetailTestCase(
            description="a 39-digit integer",
            text="[" + "1" * 39 + "]",
            expected_reason=f"{_NUMBER_REASON} (39-digit integer)",
            expected_shape="9" * 39,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_json_text_when_narrowing_the_number_reason_then_the_offending_token_is_named(
    test_case: NumberDetailTestCase,
) -> None:
    assert number_detail(reason=_NUMBER_REASON, values=(test_case.text,)) == (
        test_case.expected_reason,
        test_case.expected_shape,
    )


@pytest.mark.parametrize(
    "test_case",
    (
        CountTestCase(
            description="one hooked pass finds every raising row and its reason",
            expression="TAG(x1)",
            options=("--examples", "20"),
            expected_count=(210, True, False),
            expected_progress=(
                "1/1 probes measured, worst json_fidelity 210/500 (42.00%); top reason "
                "json_fidelity: a value DuckDB may read differently ("
            ),
        ),
        CountTestCase(
            description="DuckDB's own error falls back to an exact TRY count and capped examples",
            expression="NATIVE(x1)",
            options=("--examples", "20"),
            expected_count=(210, True, True),
            expected_progress=(
                "1/1 probes measured, worst json_fidelity 210/500 (42.00%); top reason "
                "json_fidelity: a value DuckDB may read differently (estimated share)"
            ),
        ),
        CountTestCase(
            description="a body of another shape is bisected, a lower bound when the budget ends",
            expression="UNSHAPED(x1)",
            options=("--query-limit", "3"),
            expected_count=(0, False, True),
            expected_progress="1/1 probes measured, none raising",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_many_raising_tuples_when_measuring_then_the_count_is_exact_and_reasons_scaled(
    test_case: CountTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    document: dict[str, object] = measure(
        tmp_path, _MANY_RAISING, test_case.expression, "VARCHAR", test_case.options
    )
    probes: object = document["probes"]
    assert isinstance(probes, list)
    probe: dict[str, object] = probes[0]
    progress_line: str = str(_PROGRESS_LINE.findall(capsys.readouterr().err))
    assert (
        (probe["raising_rows"], probe["exact"], probe["reasons_estimated"]),
        test_case.expected_progress in progress_line,
    ) == (test_case.expected_count, True)
