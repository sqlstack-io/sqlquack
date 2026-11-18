"""A nested field is written as a path below its column, quoted in backticks after the column."""

from __future__ import annotations

import pytest

from tests.unit.scripts.syntax_obligations_databricks._test_types import BareFormTestCase
from tests.unit.scripts.syntax_obligations_databricks.helpers import accepted_sql


@pytest.mark.parametrize(
    "test_case",
    (
        BareFormTestCase(
            description="UPDATE sets a nested field",
            form="db.dml.update",
            expected_sql="UPDATE t1 SET c1.c1 = DEFAULT",
        ),
        BareFormTestCase(
            description="a quoted nested field keeps its column unquoted",
            form="db.ddl.alter-table-manage-column.drop",
            expected_sql="ALTER TABLE t1 DROP COLUMN ( c1.`c1 Q` )",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_field_name_operand_when_built_then_it_is_a_path_below_a_column(
    test_case: BareFormTestCase,
) -> None:
    assert test_case.expected_sql in accepted_sql(test_case.form)
