"""`sqlquack.convert` reports what `sqlquack convert --json` reports."""

from __future__ import annotations

from pathlib import Path

import pytest

import sqlquack
from sqlquack import Conversion, ConvertOptions, Diagnostic
from tests.unit.python.src.sqlquack.api.main._test_types import (
    CommandLineTestCase,
    FailureTestCase,
    TypedDiagnosticTestCase,
)
from tests.unit.python.src.sqlquack.api.main.helpers import command_report, schema_file


@pytest.mark.parametrize(
    "test_case",
    (
        CommandLineTestCase(
            description="a converted query",
            sql="SELECT order_id, price * 2 AS doubled FROM orders",
            arguments=("--builtin-policy", "--schema", "schema.json"),
            options=ConvertOptions(builtin_policy=True, schema="schema.json"),
            expected_converted=True,
        ),
        CommandLineTestCase(
            description="a query refused for an undeclared relation",
            sql="SELECT a FROM customers",
            arguments=("--builtin-policy", "--schema", "schema.json"),
            options=ConvertOptions(builtin_policy=True, schema="schema.json"),
            expected_converted=False,
        ),
        CommandLineTestCase(
            description="a query refused under a session no profile covers",
            sql="SELECT order_id FROM orders",
            arguments=("--builtin-policy", "--schema", "schema.json", "--session", "WEEK_START=3"),
            options=ConvertOptions(
                builtin_policy=True, schema="schema.json", session={"WEEK_START": "3"}
            ),
            expected_converted=False,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_the_same_options_when_converting_then_the_report_is_the_command_lines(
    test_case: CommandLineTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    schema_file(tmp_path)
    monkeypatch.chdir(tmp_path)
    conversion: Conversion = sqlquack.convert(sql=test_case.sql, options=test_case.options)
    assert conversion.report == command_report(test_case.sql, test_case.arguments, tmp_path)
    assert conversion.converted is test_case.expected_converted
    assert len(conversion.diagnostics) == len(conversion.report["refusals"])


@pytest.mark.parametrize(
    "test_case",
    (
        TypedDiagnosticTestCase(
            description="an undeclared relation is SQ0004 at the relation's name",
            sql="SELECT a FROM orders",
            options=ConvertOptions(builtin_policy=True),
            expected_identity=("SQ0004", "unresolved_name", "error"),
            expected_span=sqlquack.Span(
                start=14, end=20, line=1, column=15, end_line=1, end_column=21
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_refusal_when_converting_then_the_diagnostic_is_typed(
    test_case: TypedDiagnosticTestCase,
) -> None:
    conversion: Conversion = sqlquack.convert(sql=test_case.sql, options=test_case.options)
    found: Diagnostic = conversion.diagnostics[0]
    assert (found.code, found.name, found.severity) == test_case.expected_identity
    assert found.span == test_case.expected_span
    assert found.help is not None
    assert conversion.sql == ""


@pytest.mark.parametrize(
    "test_case",
    (
        FailureTestCase(
            description="no policy file and no built-in policy, as on the command line",
            sql="SELECT 1",
            options=None,
            expected_message="no policy file at sqlquack.toml",
        ),
        FailureTestCase(
            description="input holding no statement",
            sql="-- nothing here",
            options=ConvertOptions(builtin_policy=True),
            expected_message="the input holds no SQL statement",
        ),
        FailureTestCase(
            description="an unreadable schema",
            sql="SELECT 1",
            options=ConvertOptions(builtin_policy=True, schema="missing.json"),
            expected_message="missing.json",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_input_the_command_line_exits_2_on_when_converting_then_sqlquack_error_is_raised(
    test_case: FailureTestCase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.chdir(tmp_path)
    with pytest.raises(sqlquack.SqlQuackError, match=test_case.expected_message):
        sqlquack.convert(sql=test_case.sql, options=test_case.options)
