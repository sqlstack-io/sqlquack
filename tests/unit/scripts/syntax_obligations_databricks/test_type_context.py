"""A data type form's cases write the type where a type goes, so no case reads it as a column."""

from __future__ import annotations

import pytest

from tests.unit.scripts.syntax_obligations_databricks._test_types import BareFormTestCase
from tests.unit.scripts.syntax_obligations_databricks.helpers import accepted_sql


@pytest.mark.parametrize(
    "test_case",
    (
        BareFormTestCase(
            description="INTEGER, a spelling of INT",
            form="db.expressions.int-type",
            expected_sql="SELECT CAST ( NULL AS INTEGER )",
        ),
        BareFormTestCase(
            description="SHORT, a spelling of SMALLINT",
            form="db.expressions.smallint-type",
            expected_sql="SELECT CAST ( NULL AS SHORT )",
        ),
        BareFormTestCase(
            description="BYTE, a spelling of TINYINT",
            form="db.expressions.tinyint-type",
            expected_sql="SELECT CAST ( NULL AS BYTE )",
        ),
        BareFormTestCase(
            description="STRING",
            form="db.expressions.string-type",
            expected_sql="SELECT CAST ( NULL AS STRING )",
        ),
        BareFormTestCase(
            description="VARIANT",
            form="db.expressions.variant-type",
            expected_sql="SELECT CAST ( NULL AS VARIANT )",
        ),
        BareFormTestCase(
            description="TIME without a precision",
            form="db.expressions.time-type.time",
            expected_sql="SELECT CAST ( NULL AS TIME )",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_data_type_form_when_built_then_its_positive_casts_to_the_type(
    test_case: BareFormTestCase,
) -> None:
    assert test_case.expected_sql in accepted_sql(test_case.form)
