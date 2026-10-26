"""Compiled CREATE FUNCTION headers read as schema signatures."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.census_functions._helpers.headers import signature
from scripts.census_functions.main.add_functions import run_add_functions
from scripts.census_functions.models import Signature
from tests.unit.scripts.census_functions._test_types import HeaderTestCase, SchemaFileTestCase


@pytest.mark.parametrize(
    "test_case",
    (
        HeaderTestCase(
            description="a Python scalar function",
            header=(
                "CREATE OR REPLACE FUNCTION SHOP.PUBLIC.IS_GIFT(note STRING)\nRETURNS BOOLEAN\n"
                "LANGUAGE PYTHON\nRUNTIME_VERSION = '3.11'\nHANDLER = 'run'\nAS $$\ndef run(n): ...\n$$"
            ),
            expected_signature=Signature(
                language="python", arguments=(("NOTE", "STRING"),), returns="BOOLEAN"
            ),
        ),
        HeaderTestCase(
            description="a SQL table function with typed parameters and columns",
            header=(
                "CREATE OR REPLACE FUNCTION SHOP.PUBLIC.OPEN_ORDERS(minimum NUMBER(10,2), "
                "regions ARRAY)\nRETURNS TABLE (order_id NUMBER(38,0), placed TIMESTAMP_NTZ)\n"
                "AS $$\nSELECT 1, NULL\n$$"
            ),
            expected_signature=Signature(
                language="sql",
                arguments=(("MINIMUM", "NUMBER(10,2)"), ("REGIONS", "ARRAY")),
                returns=(("ORDER_ID", "NUMBER(38,0)"), ("PLACED", "TIMESTAMP_NTZ")),
            ),
        ),
        HeaderTestCase(
            description="text without a function header",
            header="SELECT 1",
            expected_signature=None,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_compiled_function_when_reading_its_header_then_the_signature_is_returned(
    test_case: HeaderTestCase,
) -> None:
    assert signature(test_case.header) == test_case.expected_signature


@pytest.mark.parametrize(
    "test_case",
    (
        SchemaFileTestCase(
            description="a relations-only schema gains a functions section",
            schema='{"ORDERS": [["ORDER_ID", "NUMBER(38,0)"]]}',
            expected_schema=(
                '{"relations": {"ORDERS": [["ORDER_ID", "NUMBER(38,0)"]]}, "functions": '
                '{"__SQLQUACK_UDF__IS_GIFT": {"language": "sql", "arguments": [["NOTE", "STRING"]], '
                '"returns": "BOOLEAN"}}}'
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_compiled_functions_when_adding_them_then_the_schema_keeps_its_relations(
    test_case: SchemaFileTestCase, tmp_path: Path
) -> None:
    functions: Path = tmp_path / "functions"
    functions.mkdir()
    (functions / "is_gift.sql").write_text(
        "CREATE FUNCTION S.P.IS_GIFT(note STRING) RETURNS BOOLEAN AS $$ note = 'gift' $$",
        encoding="utf-8",
    )
    schema: Path = tmp_path / "schema.json"
    schema.write_text(test_case.schema, encoding="utf-8")

    assert run_add_functions(["--functions", str(functions), "--schema", str(schema)]) == 0
    assert schema.read_text(encoding="utf-8") == test_case.expected_schema
