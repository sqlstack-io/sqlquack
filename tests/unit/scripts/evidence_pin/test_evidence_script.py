"""scripts/evidence.sh against a throwaway evidence remote, and the content hash it shares."""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.evidence_pin._helpers.content_hash import content_sha256
from tests.unit.scripts.evidence_pin._test_types import (
    ContentHashTestCase,
    EvidenceRepositories,
    ScriptOutcome,
    ScriptTestCase,
)
from tests.unit.scripts.evidence_pin.helpers import (
    clone_then_clone_offline,
    evidence_repositories,
    file_tree,
    path_after_clone_changes,
    path_of_working_tree,
    path_offline_after_first_fetch,
    path_offline_without_commit,
    path_while_unpinned,
    path_with_wrong_hash,
    pin_main_then_older,
    pipeline_content_hash,
    pipeline_listing,
    refresh_by_command,
    refresh_by_variable,
)


@pytest.mark.parametrize(
    "test_case",
    (
        ScriptTestCase(
            description="path refuses while nothing is pinned",
            scenario=path_while_unpinned,
            expected_return_code=2,
            expected_fragment="no evidence is pinned yet",
        ),
        ScriptTestCase(
            description="pin writes the commit and the content hash of its files",
            scenario=pin_main_then_older,
            expected_return_code=0,
            expected_fragment="evidence: pinned commit ",
        ),
        ScriptTestCase(
            description="after the first fetch, path needs no network",
            scenario=path_offline_after_first_fetch,
            expected_return_code=0,
            expected_fragment="",
        ),
        ScriptTestCase(
            description="REFRESH=1 fetches and extracts again",
            scenario=refresh_by_variable,
            expected_return_code=0,
            expected_fragment="failed; extracting it again",
        ),
        ScriptTestCase(
            description="refresh fetches and extracts again",
            scenario=refresh_by_command,
            expected_return_code=0,
            expected_fragment="failed; extracting it again",
        ),
        ScriptTestCase(
            description="a local edit and a newer main in the clone leave the snapshot alone",
            scenario=path_after_clone_changes,
            expected_return_code=0,
            expected_fragment="",
        ),
        ScriptTestCase(
            description="a missing commit while offline fails with one clear message",
            scenario=path_offline_without_commit,
            expected_return_code=2,
            expected_fragment="and fetching it failed",
        ),
        ScriptTestCase(
            description="a wrong content hash fails naming both hashes",
            scenario=path_with_wrong_hash,
            expected_return_code=2,
            expected_fragment="content hash mismatch for commit",
        ),
        ScriptTestCase(
            description="EVIDENCE=working prints the clone with a warning",
            scenario=path_of_working_tree,
            expected_return_code=0,
            expected_fragment="unpinned working tree",
        ),
        ScriptTestCase(
            description="clone clones without checkout once, then needs no network",
            scenario=clone_then_clone_offline,
            expected_return_code=0,
            expected_fragment="",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_scenario_when_running_evidence_script_then_it_ends_as_expected(
    test_case: ScriptTestCase, tmp_path: Path
) -> None:
    repositories: EvidenceRepositories = evidence_repositories(tmp_path)

    outcome: ScriptOutcome = test_case.scenario(repositories)

    assert outcome.return_code == test_case.expected_return_code, outcome.stderr
    assert test_case.expected_fragment in outcome.stderr
    assert outcome.facts == dict.fromkeys(outcome.facts, True)


@pytest.mark.parametrize(
    "test_case",
    (
        ContentHashTestCase(
            description="plain names",
            files={"plain.txt": b"plain\n", "nested/a-b": b"1", "nested.txt": b"2"},
            links={},
            expected_files_hashed=3,
        ),
        ContentHashTestCase(
            description="names sha256sum escapes, non-ASCII, case and dot files; links left out",
            files={
                "with space.jsonl": b"{}\n",
                "back\\slash": b"x",
                "new\nline": b"y",
                "carriage\rreturn": b"z",
                "Upper": b"",
                "lower": b"\x00\xff",
                "nested/deeper/caf\u00e9.json": b"{}",
                ".hidden": b"3",
            },
            links={"link": "lower"},
            expected_files_hashed=8,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_files_when_hashing_then_bash_and_python_agree(
    test_case: ContentHashTestCase, tmp_path: Path
) -> None:
    root: Path = file_tree(tmp_path / "snapshot", files=test_case.files, links=test_case.links)

    python_hash: str = content_sha256(root)

    assert python_hash == pipeline_content_hash(root)
    assert len(pipeline_listing(root).splitlines()) == test_case.expected_files_hashed
