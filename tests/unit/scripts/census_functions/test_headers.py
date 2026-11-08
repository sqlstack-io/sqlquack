"""Compiled CREATE FUNCTION headers read as signatures and written as project function files."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.census_functions._helpers.headers import signature
from scripts.census_functions.main.add_functions import run_add_functions
from scripts.census_functions.models import Signature
from tests.unit.scripts.census_functions._test_types import HeaderTestCase, ProjectFilesTestCase

_SOURCES: str = (
    "sources:\n  - name: orders\n    columns:\n      - name: order_id\n        type: NUMBER\n"
)


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
        ProjectFilesTestCase(
            description="a SQL scalar function is a SQL function file under its placeholder",
            compiled=(
                (
                    "is_gift",
                    "CREATE FUNCTION S.P.IS_GIFT(note STRING) RETURNS BOOLEAN AS $$ note = 'gift' $$",
                ),
            ),
            expected_files={
                "functions/sql/__SQLQUACK_UDF__IS_GIFT.sql": (
                    "FUNCTION (\n  arguments (NOTE STRING),\n  returns BOOLEAN,\n);\nNULL\n"
                ),
            },
        ),
        ProjectFilesTestCase(
            description="a SQL table function returns its columns as a table",
            compiled=(
                (
                    "open_orders",
                    "CREATE FUNCTION S.P.OPEN_ORDERS() RETURNS TABLE (order_id NUMBER(38,0), "
                    "placed TIMESTAMP_NTZ) AS $$ SELECT 1, NULL $$",
                ),
            ),
            expected_files={
                "functions/sql/__SQLQUACK_TABLE_FN__OPEN_ORDERS.sql": (
                    "FUNCTION (\n  arguments (),\n"
                    "  returns table (ORDER_ID NUMBER(38,0), PLACED TIMESTAMP_NTZ),\n);\nNULL\n"
                ),
            },
        ),
        ProjectFilesTestCase(
            description="a Python scalar function is a Python function file with its decorator",
            compiled=(
                (
                    "py_score",
                    "CREATE FUNCTION S.P.PY_SCORE(x NUMBER, label STRING) RETURNS NUMBER "
                    "LANGUAGE PYTHON HANDLER = 'f' AS $$\ndef f(x, label):\n    return x\n$$",
                ),
            ),
            expected_files={
                "functions/python/__SQLQUACK_UDF__PY_SCORE.py": (
                    "from sqlquack import udf\n\n\n"
                    '@udf(arguments={"X": "NUMBER", "LABEL": "STRING"}, returns="NUMBER")\n'
                    "def main(*arguments):\n    return None\n"
                ),
            },
        ),
        ProjectFilesTestCase(
            description="a Python table function or a name that is not plain is not declarable",
            compiled=(
                (
                    "py_rows",
                    "CREATE FUNCTION S.P.PY_ROWS(x NUMBER) RETURNS TABLE (a NUMBER) "
                    "LANGUAGE PYTHON HANDLER = 'f' AS $$\nclass f: ...\n$$",
                ),
                (
                    "hyphened",
                    'CREATE FUNCTION S.P.HYPHENED("my-arg" NUMBER) RETURNS NUMBER AS $$ 1 $$',
                ),
            ),
            expected_files={},
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_compiled_functions_when_adding_them_then_the_project_keeps_its_sources(
    test_case: ProjectFilesTestCase, tmp_path: Path
) -> None:
    functions: Path = tmp_path / "functions"
    functions.mkdir()
    for stem, text in test_case.compiled:
        (functions / f"{stem}.sql").write_text(text, encoding="utf-8")
    project: Path = tmp_path / "project"
    (project / "sources").mkdir(parents=True)
    (project / "sources" / "orders.yml").write_text(_SOURCES, encoding="utf-8")
    (project / "functions" / "sql").mkdir(parents=True)
    (project / "functions" / "sql" / "__SQLQUACK_UDF__STALE.sql").write_text(
        "FUNCTION (arguments (), returns NUMBER);\nNULL\n", encoding="utf-8"
    )

    status: int = run_add_functions(["--functions", str(functions), "--project", str(project)])
    written: dict[str, str] = {
        str(path.relative_to(project)): path.read_text(encoding="utf-8")
        for path in filter(Path.is_file, sorted(project.rglob("*")))
    }
    assert (status, written) == (
        0,
        {**test_case.expected_files, "sources/orders.yml": _SOURCES},
    )
