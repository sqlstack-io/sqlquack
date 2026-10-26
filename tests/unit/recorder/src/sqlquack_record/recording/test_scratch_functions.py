"""Renaming a case's functions for its run, and dropping them afterwards."""

from __future__ import annotations

import pytest

from sqlquack_record.recording._helpers.functions import scratch_functions
from sqlquack_record.recording.models import ScratchFunctions
from tests.unit.recorder.src.sqlquack_record.recording._test_types import ScratchTestCase

_HASH: str = "0123456789abcdef" + "0" * 48


@pytest.mark.parametrize(
    "test_case",
    (
        ScratchTestCase(
            description="calls are renamed, a column of the same name is not, and drops are typed",
            setup_sql=(
                "CREATE FUNCTION order_total(qty NUMBER(10,0), price NUMBER(10,2))\n"
                "RETURNS NUMBER(12,2)\nAS $$\nqty * price\n$$",
            ),
            source_sql="SELECT order_total(qty, price) AS order_total FROM input",
            expected_functions=ScratchFunctions(
                setup_sql=(
                    "CREATE FUNCTION order_total__sqlquack_0123456789abcdef_a1b2c3d4"
                    "(qty NUMBER(10,0), price NUMBER(10,2))\n"
                    "RETURNS NUMBER(12,2)\nAS $$\nqty * price\n$$",
                ),
                source_sql=(
                    "SELECT order_total__sqlquack_0123456789abcdef_a1b2c3d4(qty, price) AS order_total "
                    "FROM input"
                ),
                drop_sql=(
                    "DROP FUNCTION IF EXISTS order_total__sqlquack_0123456789abcdef_a1b2c3d4"
                    "(NUMBER(10,0), NUMBER(10,2))",
                ),
            ),
        ),
        ScratchTestCase(
            description="a function calling another is renamed in its body too",
            setup_sql=(
                "CREATE FUNCTION base(x VARCHAR)\nRETURNS VARCHAR\nAS $$\nx\n$$",
                "CREATE FUNCTION outer_label()\nRETURNS VARCHAR\nAS $$\nbase('a')\n$$",
            ),
            source_sql="SELECT outer_label() AS v",
            expected_functions=ScratchFunctions(
                setup_sql=(
                    "CREATE FUNCTION base__sqlquack_0123456789abcdef_a1b2c3d4(x VARCHAR)\n"
                    "RETURNS VARCHAR\nAS $$\nx\n$$",
                    "CREATE FUNCTION outer_label__sqlquack_0123456789abcdef_a1b2c3d4()\n"
                    "RETURNS VARCHAR\nAS $$\nbase__sqlquack_0123456789abcdef_a1b2c3d4('a')\n$$",
                ),
                source_sql="SELECT outer_label__sqlquack_0123456789abcdef_a1b2c3d4() AS v",
                drop_sql=(
                    "DROP FUNCTION IF EXISTS base__sqlquack_0123456789abcdef_a1b2c3d4(VARCHAR)",
                    "DROP FUNCTION IF EXISTS outer_label__sqlquack_0123456789abcdef_a1b2c3d4()",
                ),
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_case_functions_when_renamed_for_a_run_then_calls_and_drops_use_unique_names(
    test_case: ScratchTestCase,
) -> None:
    """Given a case's functions, when renamed for its run, then calls and drops use unique names."""
    assert (
        scratch_functions(
            setup_sql=test_case.setup_sql,
            source_sql=test_case.source_sql,
            case_hash=_HASH,
            run_nonce="a1b2c3d4",
        )
        == test_case.expected_functions
    )
