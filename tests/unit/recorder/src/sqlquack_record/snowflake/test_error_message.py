"""Warehouse error messages kept without their query ID."""

from __future__ import annotations

import pytest
from snowflake.connector.errors import ProgrammingError

from sqlquack_record.snowflake._helpers.cursors import error_message
from tests.unit.recorder.src.sqlquack_record.snowflake._test_types import ErrorMessageTestCase

_QUERY_ID: str = "01b2c3d4-0000-4a5b-8c9d-0123456789ab"


@pytest.mark.parametrize(
    "test_case",
    (
        ErrorMessageTestCase(
            description="a plain message is kept as it is",
            message="Numeric value 'abc' is not recognized",
            sqlstate="22018",
            query_id=None,
            expected_message="Numeric value 'abc' is not recognized",
        ),
        ErrorMessageTestCase(
            description="the errno and SQLSTATE prefix and the query ID after it are removed",
            message=f"100038 (22018): {_QUERY_ID}: Numeric value 'abc' is not recognized",
            sqlstate="22018",
            query_id=_QUERY_ID,
            expected_message="Numeric value 'abc' is not recognized",
        ),
        ErrorMessageTestCase(
            description="a query ID in parentheses at the end is removed",
            message=f"Division by zero (query id: {_QUERY_ID})",
            sqlstate="22018",
            query_id=None,
            expected_message="Division by zero",
        ),
        ErrorMessageTestCase(
            description="a compilation error's message, which can name account objects, is dropped",
            message="SQL compilation error: Object 'ORDERS' does not exist",
            sqlstate="42S02",
            query_id=None,
            expected_message=None,
        ),
        ErrorMessageTestCase(
            description="a listed built-in function error keeps its message",
            message="100344 (P0000): Type LineString is not supported as argument to ST_AZIMUTH.",
            sqlstate="P0000",
            query_id=None,
            expected_message="Type LineString is not supported as argument to ST_AZIMUTH.",
            errno=100344,
        ),
        ErrorMessageTestCase(
            description="an unlisted P0000 error's message is dropped",
            message="100999 (P0000): Something internal",
            sqlstate="P0000",
            query_id=None,
            expected_message=None,
            errno=100999,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_warehouse_error_when_reading_message_then_data_error_kept_without_query_id(
    test_case: ErrorMessageTestCase,
) -> None:
    error: ProgrammingError = ProgrammingError(
        msg=test_case.message,
        errno=test_case.errno,
        sqlstate=test_case.sqlstate,
        sfqid=test_case.query_id,
    )

    assert error_message(error) == test_case.expected_message


if __name__ == "__main__":
    pytest.main([__file__, "-vv"])
