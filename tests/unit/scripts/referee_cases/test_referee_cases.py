"""The referee generators: deterministic output, the --check mode and the mutants."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.referee_cases._helpers.mutants import mutant_statements
from scripts.referee_cases._helpers.registry import generated_texts
from scripts.referee_cases.main.generate import run_generate
from scripts.referee_cases.models import Statement
from tests.unit.scripts.referee_cases._test_types import (
    CheckTestCase,
    DeterminismTestCase,
    MutationTestCase,
    SplitTestCase,
)
from tests.unit.scripts.referee_cases.helpers import (
    duplicate_names,
    edited,
    split_layout,
    stray,
    untouched,
)


@pytest.mark.parametrize(
    "test_case",
    (
        DeterminismTestCase(
            description="two runs write the same files", runs=2, expected_duplicate_names=0
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_the_generators_when_run_again_then_every_file_is_identical_and_named_uniquely(
    test_case: DeterminismTestCase,
) -> None:
    runs: list[dict[str, str]] = [generated_texts() for _ in range(test_case.runs)]

    assert all(run == runs[0] for run in runs), test_case.description
    assert duplicate_names(runs[0]) == test_case.expected_duplicate_names, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        CheckTestCase("freshly generated files pass", untouched, 0, "passed"),
        CheckTestCase("a hand-edited file fails", edited, 1, "names.json is out of date"),
        CheckTestCase("a stray file fails", stray, 1, "extra.json is out of date"),
    ),
    ids=lambda case: case.description,
)
def test_given_generated_referee_files_when_checking_then_only_exact_output_passes(
    test_case: CheckTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    assert run_generate(["--repository", str(tmp_path)]) == 0
    test_case.tamper(tmp_path)

    return_code: int = run_generate(["--repository", str(tmp_path), "--check"])

    assert return_code == test_case.expected_return_code, test_case.description
    assert test_case.expected_fragment in capsys.readouterr().err, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        SplitTestCase(
            description="cases go to the evidence directory and environments stay",
            expected_layout={
                "cases written under the cases directory": True,
                "environments written in the repository": True,
                "no cases in the repository": True,
                "no environments in the cases directory": True,
                "check passes on the split layout": True,
            },
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_cases_directory_when_generating_then_only_the_cases_are_written_there(
    test_case: SplitTestCase, tmp_path: Path
) -> None:
    repository: Path = tmp_path / "repository"
    cases: Path = tmp_path / "evidence" / "referee" / "snowflake" / "cases"
    arguments: list[str] = ["--repository", str(repository), "--cases", str(cases)]

    assert run_generate(arguments) == 0

    layout: dict[str, bool] = split_layout(repository=repository, cases=cases)
    layout["check passes on the split layout"] = run_generate([*arguments, "--check"]) == 0
    assert layout == test_case.expected_layout, test_case.description


@pytest.mark.parametrize(
    "test_case",
    (
        MutationTestCase(
            description="a column is misspelt by dropping its middle character",
            base="SELECT amount FROM orders",
            expected_mutation="misspelt_column",
            expected_sql="SELECT amont FROM orders",
        ),
        MutationTestCase(
            description="a later alias reference is misspelt, its definition kept",
            base="SELECT amount * 2 AS doubled FROM orders ORDER BY doubled",
            expected_mutation="misspelt_alias",
            expected_sql="SELECT amount * 2 AS doubled FROM orders ORDER BY douled",
        ),
        MutationTestCase(
            description="the last of two GROUP BY keys is dropped",
            base="SELECT region, status, COUNT(*) AS n FROM orders GROUP BY region, status",
            expected_mutation="missing_group_by",
            expected_sql="SELECT region, status, COUNT(*) AS n FROM orders GROUP BY region",
        ),
        MutationTestCase(
            description="a qualified column both joined tables have loses its qualifier",
            base="SELECT o.region FROM orders AS o JOIN customers AS c ON TRUE",
            expected_mutation="ambiguous_name",
            expected_sql="SELECT region FROM orders AS o JOIN customers AS c ON TRUE",
        ),
        MutationTestCase(
            description="a call gains one argument",
            base="SELECT ABS(c_number) AS r FROM all_types",
            expected_mutation="extra_argument",
            expected_sql="SELECT ABS(c_number, 1) AS r FROM all_types",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_valid_statement_when_mutating_then_each_mutant_makes_one_mistake(
    test_case: MutationTestCase,
) -> None:
    mutants: tuple[Statement, ...] = mutant_statements(
        (Statement(name="base", sql=test_case.base, kind="valid"),)
    )

    produced: dict[str | None, str] = {mutant.mutation: mutant.sql for mutant in mutants}
    assert produced[test_case.expected_mutation] == test_case.expected_sql, test_case.description
    assert all(mutant.base == "base" for mutant in mutants), test_case.description
