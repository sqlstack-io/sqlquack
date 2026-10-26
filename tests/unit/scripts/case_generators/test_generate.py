"""The case generators: deterministic output, valid case files and the --check mode."""

from __future__ import annotations

import tomllib
from pathlib import Path

import pytest

from scripts.case_generators._helpers.registry import generated_texts
from scripts.case_generators.main.generate import run_generate
from tests.unit.scripts.case_generators._test_types import CheckTestCase, FileTestCase
from tests.unit.scripts.case_generators.helpers import edited, removed, stray, untouched


@pytest.mark.parametrize(
    "test_case",
    [
        FileTestCase(
            description=name,
            name=name,
            expected_origin_prefix=f"generated: {name.split('/', 1)[0]}@",
        )
        for name in sorted(generated_texts())
    ],
    ids=lambda case: case.description,
)
def test_given_a_generated_file_when_generated_twice_then_it_is_identical_and_valid(
    test_case: FileTestCase,
) -> None:
    text: str = generated_texts()[test_case.name]
    assert generated_texts()[test_case.name] == text
    parsed: dict[str, list[dict[str, object]]] = tomllib.loads(text)
    cases: list[dict[str, object]] = parsed["case"]
    ids: list[object] = [case["id"] for case in cases]
    assert len(ids) == len(set(ids))
    assert all(str(case["origin"]).startswith(test_case.expected_origin_prefix) for case in cases)
    assert all(case["rows"] for case in cases)


@pytest.mark.parametrize(
    "test_case",
    (
        CheckTestCase("freshly generated files pass", untouched, 0, "passed"),
        CheckTestCase(
            "a hand-edited file fails", edited, 1, "convert_timezone.toml is out of date"
        ),
        CheckTestCase("a stray file fails", stray, 1, "extra.toml is out of date"),
        CheckTestCase("a missing file fails", removed, 1, "convert_timezone.toml is out of date"),
    ),
    ids=lambda case: case.description,
)
def test_given_generated_files_when_checking_then_only_exact_output_passes(
    test_case: CheckTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    assert run_generate(["--repository", str(tmp_path)]) == 0
    test_case.tamper(tmp_path)
    assert (
        run_generate(["--repository", str(tmp_path), "--check"]) == test_case.expected_return_code
    )
    assert test_case.expected_fragment in capsys.readouterr().err
