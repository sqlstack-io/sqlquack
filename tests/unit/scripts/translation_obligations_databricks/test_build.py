"""The build cites each live form's measured cases, and --check fails when a file differs."""

from __future__ import annotations

import tomllib
from pathlib import Path

import pytest

from scripts.translation_obligations_databricks.main.build import run_build
from tests.unit.scripts.translation_obligations_databricks._test_types import BuildTestCase
from tests.unit.scripts.translation_obligations_databricks.helpers import small_root


@pytest.mark.parametrize(
    "test_case",
    (
        BuildTestCase(
            description="committed files as the build writes them",
            edit="required = true",
            expected_code=0,
            expected_cited=(
                ("cases/translation/functions/abs.toml#t001",),
                ("cases/translation/functions/abs.toml#t003",),
            ),
        ),
        BuildTestCase(
            description="a committed obligation edited",
            edit="required = false",
            expected_code=1,
            expected_cited=(
                ("cases/translation/functions/abs.toml#t001",),
                ("cases/translation/functions/abs.toml#t003",),
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_cases_when_building_obligations_then_only_live_measured_cases_are_cited(
    tmp_path: Path, test_case: BuildTestCase
) -> None:
    catalogue, cases, out = small_root(root=tmp_path)
    paths: list[str] = ["--cases", str(cases), "--catalogue", str(catalogue), "--out", str(out)]
    run_build(paths)
    written: Path = out / "functions.toml"
    written.write_text(written.read_text().replace("required = true", test_case.edit, 1))

    code: int = run_build([*paths, "--check"])

    obligations: list[dict[str, list[str]]] = tomllib.loads(written.read_text())["obligation"]
    cited: tuple[tuple[str, ...], ...] = tuple(tuple(row["cases"]) for row in obligations)
    assert (code, cited) == (test_case.expected_code, test_case.expected_cited), (
        test_case.description
    )
