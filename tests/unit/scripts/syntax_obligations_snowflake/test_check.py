"""The build's --check fails when a committed obligations, withdrawn or elisions file is stale."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.syntax_obligations_snowflake.main.build import run_build
from tests.unit.scripts.syntax_obligations_snowflake._test_types import (
    CheckTestCase,
    MissingFileTestCase,
    StrayElisionTestCase,
)
from tests.unit.scripts.syntax_obligations_snowflake.helpers import (
    ELISION_FAMILY,
    FAMILY,
    WITHDRAWN_ROW,
    built_root,
)

STALE_ROW: str = (
    '[[withdrawn]]\nform = "sf.transaction_session.use-database"\nkind = "negative_neighbour"\n'
    'target = ""\nnote = "a rejection the build no longer withdraws"\nsql = "USE ;"\n'
)


@pytest.mark.parametrize(
    "test_case",
    (
        CheckTestCase(
            description="committed files as the build writes them",
            family=FAMILY,
            relative=f"withdrawn/{FAMILY}.toml",
            old=WITHDRAWN_ROW,
            new=WITHDRAWN_ROW,
            expected_code=0,
            expected_stale=(),
        ),
        CheckTestCase(
            description="a withdrawn file holding a withdrawal the build no longer makes",
            family=FAMILY,
            relative=f"withdrawn/{FAMILY}.toml",
            old="[[withdrawn]]\n",
            new=STALE_ROW + "\n[[withdrawn]]\n",
            expected_code=1,
            expected_stale=(f"withdrawn/{FAMILY}.toml",),
        ),
        CheckTestCase(
            description="a withdrawn file missing a withdrawal the build makes",
            family=FAMILY,
            relative=f"withdrawn/{FAMILY}.toml",
            old=WITHDRAWN_ROW,
            new='form = "sf.transaction_session.use-other"',
            expected_code=1,
            expected_stale=(f"withdrawn/{FAMILY}.toml",),
        ),
        CheckTestCase(
            description="an obligation made optional",
            family=FAMILY,
            relative=f"obligations/{FAMILY}.toml",
            old="required = true",
            new="required = false",
            expected_code=1,
            expected_stale=(f"obligations/{FAMILY}.toml",),
        ),
        CheckTestCase(
            description="an elision family's files as the build writes them",
            family=ELISION_FAMILY,
            relative=f"elisions/{ELISION_FAMILY}.toml",
            old='fill = "1"',
            new='fill = "1"',
            expected_code=0,
            expected_stale=(),
        ),
        CheckTestCase(
            description="an elisions file with its fill edited",
            family=ELISION_FAMILY,
            relative=f"elisions/{ELISION_FAMILY}.toml",
            old='fill = "1"',
            new='fill = "2"',
            expected_code=1,
            expected_stale=(f"elisions/{ELISION_FAMILY}.toml",),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_committed_files_when_checking_then_only_a_stale_file_fails_and_none_is_written(
    test_case: CheckTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    root: Path = built_root(directory=tmp_path, family=test_case.family)
    path: Path = root / test_case.relative
    edited: str = path.read_text().replace(test_case.old, test_case.new, 1)
    path.write_text(edited)
    capsys.readouterr()

    code: int = run_build(["--root", str(root), "--check", test_case.family])

    assert code == test_case.expected_code
    assert capsys.readouterr().err.splitlines() == [
        f"stale: {root / relative} differs from what the build writes"
        for relative in test_case.expected_stale
    ]
    assert path.read_text() == edited


@pytest.mark.parametrize(
    "test_case",
    (
        MissingFileTestCase(
            description="the withdrawn file deleted",
            family=FAMILY,
            relative=f"withdrawn/{FAMILY}.toml",
            expected_code=1,
        ),
        MissingFileTestCase(
            description="the obligations file deleted",
            family=FAMILY,
            relative=f"obligations/{FAMILY}.toml",
            expected_code=1,
        ),
        MissingFileTestCase(
            description="an elision family's elisions file deleted",
            family=ELISION_FAMILY,
            relative=f"elisions/{ELISION_FAMILY}.toml",
            expected_code=1,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_file_the_build_writes_is_missing_when_checking_then_it_fails_and_stays_missing(
    test_case: MissingFileTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    root: Path = built_root(directory=tmp_path, family=test_case.family)
    path: Path = root / test_case.relative
    path.unlink()
    capsys.readouterr()

    code: int = run_build(["--root", str(root), "--check", test_case.family])

    assert code == test_case.expected_code
    assert capsys.readouterr().err.splitlines() == [
        f"stale: {path} differs from what the build writes"
    ]
    assert not path.exists()


@pytest.mark.parametrize(
    "test_case",
    (
        StrayElisionTestCase(
            description="regenerating",
            arguments=(FAMILY,),
            expected_code=0,
            expected_text="# kept\n",
        ),
        StrayElisionTestCase(
            description="checking",
            arguments=("--check", FAMILY),
            expected_code=0,
            expected_text="# kept\n",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_elisions_file_of_a_family_without_rules_when_building_then_it_is_left_alone(
    test_case: StrayElisionTestCase, tmp_path: Path
) -> None:
    root: Path = built_root(directory=tmp_path, family=FAMILY)
    stray: Path = root / "elisions" / f"{FAMILY}.toml"
    stray.parent.mkdir()
    stray.write_text("# kept\n")

    code: int = run_build(["--root", str(root), *test_case.arguments])

    assert code == test_case.expected_code
    assert stray.read_text() == test_case.expected_text
