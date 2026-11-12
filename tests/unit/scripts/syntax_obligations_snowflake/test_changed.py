"""--since selects only the families whose inputs or generated files changed."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.syntax_obligations_snowflake._helpers.changed import changed_families
from scripts.syntax_obligations_snowflake.constants import BUILDER_PATHS
from scripts.syntax_obligations_snowflake.main.build import run_build
from tests.unit.scripts.syntax_obligations_snowflake._test_types import (
    BranchTestCase,
    ClosureTestCase,
    SelectionTestCase,
    SkipTestCase,
)
from tests.unit.scripts.syntax_obligations_snowflake.helpers import (
    BUILDER_MODULE,
    SELECTION_FAMILIES,
    SELECTION_ROOT,
    checkout,
    commit,
    committed_repository,
    edit,
    scripts_outside,
)


@pytest.mark.parametrize(
    "test_case",
    (
        SelectionTestCase(
            description="nothing changed",
            relative=f"{SELECTION_ROOT}/limits.toml",
            old="a ddl limit",
            new="a ddl limit",
            expected_families=(),
        ),
        SelectionTestCase(
            description="a ddl limit edited in the shared limits.toml",
            relative=f"{SELECTION_ROOT}/limits.toml",
            old='note = "a ddl limit"',
            new='note = "an edited ddl limit"',
            expected_families=("ddl",),
        ),
        SelectionTestCase(
            description="a limits.toml comment that changes no row",
            relative=f"{SELECTION_ROOT}/limits.toml",
            old="[[limit]]",
            new="# a comment\n[[limit]]",
            expected_families=(),
        ),
        SelectionTestCase(
            description="one family's withdrawn file edited",
            relative=f"{SELECTION_ROOT}/withdrawn/utility.toml",
            old="# withdrawn utility",
            new="# withdrawn utility, edited",
            expected_families=("utility",),
        ),
        SelectionTestCase(
            description="a catalogue edited, which the elisions' families also read",
            relative=f"{SELECTION_ROOT}/catalogue/ddl.toml",
            old="# catalogue ddl",
            new="# catalogue ddl, edited",
            expected_families=("ddl", "expressions", "query"),
        ),
        SelectionTestCase(
            description="another dialect's file of the same family",
            relative="coverage/syntax-databricks/withdrawn/utility.toml",
            old="# withdrawn utility",
            new="# withdrawn utility, edited",
            expected_families=(),
        ),
        SelectionTestCase(
            description="a module of the builder edited",
            relative=BUILDER_MODULE,
            old="X = 0",
            new="X = 1",
            expected_families=SELECTION_FAMILIES,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_change_since_the_base_when_selecting_then_only_affected_families_are_built(
    test_case: SelectionTestCase, tmp_path: Path
) -> None:
    repository: Path = committed_repository(tmp_path)
    path: Path = repository / test_case.relative
    path.write_text(path.read_text().replace(test_case.old, test_case.new, 1))

    families: list[str] = changed_families(
        root=repository / SELECTION_ROOT, base="main", families=list(SELECTION_FAMILIES)
    )

    assert families == list(test_case.expected_families)


@pytest.mark.parametrize(
    "test_case",
    (
        BranchTestCase(
            description="a committed ddl edit while main moved on with a query edit",
            branch_edit=(f"{SELECTION_ROOT}/limits.toml", "a ddl limit", "an edited ddl limit"),
            main_edit=(f"{SELECTION_ROOT}/limits.toml", "a query suppression", "an edited one"),
            expected_families=("ddl",),
        ),
        BranchTestCase(
            description="a committed withdrawn edit while main moved on with a catalogue edit",
            branch_edit=(f"{SELECTION_ROOT}/withdrawn/utility.toml", "utility", "utility, edited"),
            main_edit=(f"{SELECTION_ROOT}/catalogue/ddl.toml", "ddl", "ddl, edited"),
            expected_families=("utility",),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_committed_branch_behind_its_base_when_selecting_then_only_its_own_changes_count(
    test_case: BranchTestCase, tmp_path: Path
) -> None:
    repository: Path = committed_repository(tmp_path)
    checkout(repository=repository, arguments=("-b", "feature"))
    edit(
        repository=repository,
        relative=test_case.branch_edit[0],
        old=test_case.branch_edit[1],
        new=test_case.branch_edit[2],
    )
    commit(repository=repository, message="branch")
    checkout(repository=repository, arguments=("main",))
    edit(
        repository=repository,
        relative=test_case.main_edit[0],
        old=test_case.main_edit[1],
        new=test_case.main_edit[2],
    )
    commit(repository=repository, message="main")
    checkout(repository=repository, arguments=("feature",))

    families: list[str] = changed_families(
        root=repository / SELECTION_ROOT, base="main", families=list(SELECTION_FAMILIES)
    )

    assert families == list(test_case.expected_families)


@pytest.mark.parametrize(
    "test_case",
    (
        SkipTestCase(
            description="no family changed since the base",
            families=SELECTION_FAMILIES,
            expected_lines=(
                "syntax obligations: 0 families changed since main; skipped 4",
                "syntax obligations: passed, 0 stale files; run without --check to regenerate",
            ),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_no_family_changed_when_checking_since_the_base_then_the_check_skips_and_passes(
    test_case: SkipTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    root: Path = committed_repository(tmp_path) / SELECTION_ROOT

    code: int = run_build(["--root", str(root), "--check", "--since", "main", *test_case.families])

    assert code == 0
    assert capsys.readouterr().out.splitlines() == list(test_case.expected_lines)


@pytest.mark.parametrize(
    "test_case",
    (
        ClosureTestCase(
            description="the Snowflake obligation builder",
            module="scripts.build_syntax_obligations_snowflake",
            selection_only=("scripts/__init__.py", "scripts/replay_scope/"),
            expected_outside=(),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_the_builder_when_importing_it_then_every_module_it_loads_is_a_builder_path(
    test_case: ClosureTestCase,
) -> None:
    outside: tuple[str, ...] = scripts_outside(
        module=test_case.module, allowed=BUILDER_PATHS + test_case.selection_only
    )

    assert outside == test_case.expected_outside
