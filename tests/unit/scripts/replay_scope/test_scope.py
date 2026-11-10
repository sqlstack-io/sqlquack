"""What a change replays and checks, the cache scope's digest, and the declared paths."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.replay_scope._helpers.crates import crate_dirs
from scripts.replay_scope._helpers.decisions import (
    needs_code_checks,
    needs_site,
    replay_scope,
    scope_digest,
    unmatched,
)
from scripts.replay_scope._helpers.git import changed, tracked
from scripts.replay_scope.constants import FULL_REPLAY_PATHS
from tests.unit.scripts.replay_scope._test_types import (
    ChecksTestCase,
    DigestTestCase,
    RenameTestCase,
    RepositoryTestCase,
    ScopeTestCase,
)
from tests.unit.scripts.replay_scope.helpers import commit_all, committed_tree

_BINARY_DIRS: frozenset[str] = frozenset(
    {"crates/sqlquack/", "crates/sqlquack-convert/", "crates/sqlquack-diagnostics/"}
)
_SITE_DIRS: frozenset[str] = frozenset({"crates/sqlquack-convert/"})
_REPOSITORY: Path = Path(__file__).resolve().parents[4]


@pytest.mark.parametrize(
    "test_case",
    (
        ScopeTestCase(
            description="documentation and the website replay nothing",
            changed=("README.md", "website/src/content/docs/index.mdx"),
            expected_scope="none",
        ),
        ScopeTestCase(
            description="a crate the binary is not built from replays nothing",
            changed=("crates/sqlquack-lint/src/lib.rs",),
            expected_scope="none",
        ),
        ScopeTestCase(
            description="a rule in the converter replays the changed cases",
            changed=("README.md", "crates/sqlquack-convert/src/snowflake/rules/cast.rs"),
            expected_scope="changed",
        ),
        ScopeTestCase(
            description="a directory's name alone is not inside it",
            changed=("crates/sqlquack-convert-extra/src/lib.rs",),
            expected_scope="none",
        ),
        ScopeTestCase(
            description="the size ratchet's baseline reruns the replay's checks",
            changed=("coverage/sql-size/baseline.json",),
            expected_scope="changed",
        ),
        ScopeTestCase(
            description="the replay engine replays every case",
            changed=("crates/sqlquack-evidence/src/replay/main/replay_on_engine.rs",),
            expected_scope="full",
        ),
        ScopeTestCase(
            description="a refusal message replays the changed cases",
            changed=("crates/sqlquack-diagnostics/src/refusals/constants.rs",),
            expected_scope="changed",
        ),
        ScopeTestCase(
            description="an error kind, which error checks read, replays every case",
            changed=("crates/sqlquack-diagnostics/src/failures/constants.rs",),
            expected_scope="full",
        ),
        ScopeTestCase(
            description="a forced file wins over a rule change",
            changed=("crates/sqlquack-convert/src/lib.rs", "Cargo.lock"),
            expected_scope="full",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_changed_paths_when_scoping_then_the_replay_matches_what_they_feed(
    test_case: ScopeTestCase,
) -> None:
    scope: str = replay_scope(paths=list(test_case.changed), binary_dirs=_BINARY_DIRS)
    assert scope == test_case.expected_scope


@pytest.mark.parametrize(
    "test_case",
    (
        ChecksTestCase(
            description="a docs page needs only the site",
            changed=("website/src/content/docs/index.mdx",),
            expected_code=False,
            expected_site=True,
        ),
        ChecksTestCase(
            description="the README needs neither the code checks nor the site",
            changed=("README.md",),
            expected_code=False,
            expected_site=False,
        ),
        ChecksTestCase(
            description="a Markdown file inside a crate is code",
            changed=("crates/sqlquack/README.md",),
            expected_code=True,
            expected_site=False,
        ),
        ChecksTestCase(
            description="a crate the engine is built from needs both",
            changed=("crates/sqlquack-convert/src/lib.rs",),
            expected_code=True,
            expected_site=True,
        ),
        ChecksTestCase(
            description="a workflow is code",
            changed=(".github/workflows/ci.yml",),
            expected_code=True,
            expected_site=False,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_changed_paths_when_planning_checks_then_only_what_they_need_runs(
    test_case: ChecksTestCase,
) -> None:
    paths: list[str] = list(test_case.changed)
    planned: tuple[bool, bool] = (
        needs_code_checks(paths=paths),
        needs_site(paths=paths, site_dirs=_SITE_DIRS),
    )
    assert planned == (test_case.expected_code, test_case.expected_site)


@pytest.mark.parametrize(
    "test_case",
    (
        DigestTestCase(
            description="a forced file's content", edited="Cargo.lock", expected_changed=True
        ),
        DigestTestCase(
            description="a file in a forced directory",
            edited="crates/sqlquack-evidence/src/lib.rs",
            expected_changed=True,
        ),
        DigestTestCase(
            description="a converter file",
            edited="crates/sqlquack-convert/src/lib.rs",
            expected_changed=False,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_edit_when_digesting_the_scope_then_only_forced_paths_change_it(
    tmp_path: Path, test_case: DigestTestCase
) -> None:
    root: Path = committed_tree(
        root=tmp_path,
        files={
            "Cargo.lock": "a",
            "crates/sqlquack-evidence/src/lib.rs": "b",
            "crates/sqlquack-convert/src/lib.rs": "c",
        },
    )
    before: str = scope_digest(root=root)
    (root / test_case.edited).write_text("edited")
    assert (scope_digest(root=root) != before) == test_case.expected_changed


@pytest.mark.parametrize(
    "test_case",
    (
        RepositoryTestCase(
            description="every declared path is tracked, a file under each forces a full replay, "
            "and the binary is built from the converter and the replay but not the linter",
            expected_unmatched=(),
            expected_forced_scopes=frozenset({"full"}),
            expected_binary_crates=(True, True, False),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_the_repository_when_checking_the_declared_paths_then_each_is_honoured(
    test_case: RepositoryTestCase,
) -> None:
    binary_dirs: frozenset[str] = crate_dirs(root=_REPOSITORY, packages=("sqlquack",))
    forced_files: list[str] = [
        tracked(root=_REPOSITORY, prefixes=(prefix,))[0] for prefix in FULL_REPLAY_PATHS
    ]
    forced_scopes: frozenset[str] = frozenset(
        replay_scope(paths=[path], binary_dirs=binary_dirs) for path in forced_files
    )
    binary_crates: tuple[bool, bool, bool] = (
        "crates/sqlquack-convert/" in binary_dirs,
        "crates/sqlquack-evidence/" in binary_dirs,
        "crates/sqlquack-lint/" in binary_dirs,
    )
    assert (tuple(unmatched(root=_REPOSITORY)), forced_scopes, binary_crates) == (
        test_case.expected_unmatched,
        test_case.expected_forced_scopes,
        test_case.expected_binary_crates,
    )


@pytest.mark.parametrize(
    "test_case",
    (
        RenameTestCase(
            description="a file moved out of a crate into the docs lists both of its paths",
            source="crates/sqlquack-evidence/notes.md",
            destination="docs/notes.md",
            expected_changed=("crates/sqlquack-evidence/notes.md", "docs/notes.md"),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_rename_when_listing_changes_then_both_paths_are_listed(
    tmp_path: Path, test_case: RenameTestCase
) -> None:
    root: Path = committed_tree(root=tmp_path, files={test_case.source: "notes"})
    (root / test_case.destination).parent.mkdir(parents=True, exist_ok=True)
    (root / test_case.source).rename(root / test_case.destination)
    commit_all(root=root)
    assert tuple(changed(root=root, base="HEAD~1")) == test_case.expected_changed
