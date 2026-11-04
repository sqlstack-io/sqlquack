"""Nanosecond columns the connector returns in microseconds: kept only when proven whole."""

from __future__ import annotations

import pytest

from tests.unit.recorder.src.sqlquack_record.snowflake._test_types import SubMicrosecondTestCase
from tests.unit.recorder.src.sqlquack_record.snowflake.helpers import checked_or_refused

_YEAR_ONE: int = -62_135_596_800 * 10**9
_YEAR_9999: int = 253_402_300_799 * 10**9


@pytest.mark.parametrize(
    "test_case",
    (
        SubMicrosecondTestCase(
            description="TIME(9) whose values have no sub-microsecond digits is kept",
            type_name="TIME",
            names=("T",),
            nanoseconds=((1_000_001_000, None, 0),),
            check_answer="counts",
            expected_checked=(True,),
            expected_message=None,
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="TIMESTAMP_NTZ(9) whose values have no sub-microsecond digits is kept",
            type_name="TIMESTAMP_NTZ",
            names=("T",),
            nanoseconds=((1_000_001_000, None, 0),),
            check_answer="counts",
            expected_checked=(True,),
            expected_message=None,
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="TIMESTAMP_LTZ(9) whose values have no sub-microsecond digits is kept",
            type_name="TIMESTAMP_LTZ",
            names=("T",),
            nanoseconds=((1_000_001_000, None, 0),),
            check_answer="counts",
            expected_checked=(True,),
            expected_message=None,
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="TIMESTAMP_TZ(9) whose values have no sub-microsecond digits is kept",
            type_name="TIMESTAMP_TZ",
            names=("T",),
            nanoseconds=((1_000_001_000, None, 0),),
            check_answer="counts",
            expected_checked=(True,),
            expected_message=None,
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="TIME(9) with a sub-microsecond digit is refused",
            type_name="TIME",
            names=("T",),
            nanoseconds=((1_000_001_000, 1_000_001_001),),
            check_answer="counts",
            expected_checked=None,
            expected_message="(1 of its values)",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="TIMESTAMP_NTZ(9) with a sub-microsecond digit is refused",
            type_name="TIMESTAMP_NTZ",
            names=("T",),
            nanoseconds=((1_000_001_000, 1_000_001_001),),
            check_answer="counts",
            expected_checked=None,
            expected_message="(1 of its values)",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="TIMESTAMP_LTZ(9) with a sub-microsecond digit is refused",
            type_name="TIMESTAMP_LTZ",
            names=("T",),
            nanoseconds=((1_000_001_000, 1_000_001_001),),
            check_answer="counts",
            expected_checked=None,
            expected_message="(1 of its values)",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="TIMESTAMP_TZ(9) with a sub-microsecond digit is refused",
            type_name="TIMESTAMP_TZ",
            names=("T",),
            nanoseconds=((1_000_001_000, 1_000_001_001),),
            check_answer="counts",
            expected_checked=None,
            expected_message="(1 of its values)",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="year 1 and year 9999 rows that force microseconds are kept when whole",
            type_name="TIMESTAMP_NTZ",
            names=("T",),
            nanoseconds=((_YEAR_ONE, _YEAR_9999, 5_000),),
            check_answer="counts",
            expected_checked=(True,),
            expected_message=None,
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="a year 1 row beside a row with nanosecond digits is refused",
            type_name="TIMESTAMP_NTZ",
            names=("T",),
            nanoseconds=((_YEAR_ONE, 1_700_000_000_123_456_789),),
            check_answer="counts",
            expected_checked=None,
            expected_message="(1 of its values)",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="an all-NULL column has nothing to drop",
            type_name="TIMESTAMP_TZ",
            names=("T",),
            nanoseconds=((None, None),),
            check_answer="counts",
            expected_checked=(True,),
            expected_message=None,
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="duplicate column names are checked by position, and the second refuses",
            type_name="TIMESTAMP_NTZ",
            names=("T", "T"),
            nanoseconds=((1_000, 2_000), (3_000, 3_001)),
            check_answer="counts",
            expected_checked=None,
            expected_message="sub-microsecond digits that unit would drop: T TIMESTAMP_NTZ(9) (1 of its values)",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="every column with sub-microsecond digits is named in one refusal",
            type_name="TIMESTAMP_NTZ",
            names=("A", "B"),
            nanoseconds=((1, 2_000), (3_001, 3_002)),
            check_answer="counts",
            expected_checked=None,
            expected_message="A TIMESTAMP_NTZ(9) (1 of its values), B TIMESTAMP_NTZ(9) (2 of its values)",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="duplicate column names that are both whole are both marked",
            type_name="TIME",
            names=("T", "T"),
            nanoseconds=((1_000, 2_000), (3_000, None)),
            check_answer="counts",
            expected_checked=(True, True),
            expected_message=None,
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="a check that errors refuses without a retry",
            type_name="TIMESTAMP_NTZ",
            names=("T",),
            nanoseconds=((1_000,),),
            check_answer="error",
            expected_checked=None,
            expected_message="the sub-microsecond check failed",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="a check answering two rows refuses",
            type_name="TIMESTAMP_NTZ",
            names=("T",),
            nanoseconds=((1_000,),),
            check_answer="two rows",
            expected_checked=None,
            expected_message="returned 2 rows rather than one row of 1 counts",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="a NULL count refuses",
            type_name="TIMESTAMP_NTZ",
            names=("T",),
            nanoseconds=((1_000,),),
            check_answer="null",
            expected_checked=None,
            expected_message="rather than integer counts",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="a boolean in place of a count refuses",
            type_name="TIMESTAMP_NTZ",
            names=("T",),
            nanoseconds=((1_000,),),
            check_answer="boolean",
            expected_checked=None,
            expected_message="rather than integer counts",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="whole Decimal counts, as the connector returns NUMBER, are kept",
            type_name="TIMESTAMP_NTZ",
            names=("T", "T"),
            nanoseconds=((1_000,), (2_000,)),
            check_answer="decimal",
            expected_checked=(True, True),
            expected_message=None,
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="whole Decimal counts above 0 still refuse",
            type_name="TIMESTAMP_NTZ",
            names=("T",),
            nanoseconds=((1_001,),),
            check_answer="decimal",
            expected_checked=None,
            expected_message="(1 of its values)",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="a fractional Decimal count refuses",
            type_name="TIMESTAMP_NTZ",
            names=("T",),
            nanoseconds=((1_000,),),
            check_answer="fraction",
            expected_checked=None,
            expected_message="rather than integer counts",
            expected_checks=1,
        ),
        SubMicrosecondTestCase(
            description="without a check, as in a batch, a microsecond column is refused",
            type_name="TIMESTAMP_NTZ",
            names=("T",),
            nanoseconds=((1_000,),),
            check_answer="absent",
            expected_checked=None,
            expected_message="outside the nanosecond range",
            expected_checks=0,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_nanosecond_column_in_microseconds_when_fetched_then_kept_only_if_whole(
    test_case: SubMicrosecondTestCase,
) -> None:
    checked, message, checks = checked_or_refused(
        type_name=test_case.type_name,
        names=test_case.names,
        nanoseconds=test_case.nanoseconds,
        answer=test_case.check_answer,
    )

    assert checked == test_case.expected_checked
    assert (message is None) == (test_case.expected_message is None)
    assert test_case.expected_message is None or test_case.expected_message in str(message)
    assert checks == test_case.expected_checks


if __name__ == "__main__":
    pytest.main([__file__, "-vv"])
