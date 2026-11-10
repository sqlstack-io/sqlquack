"""The clutter guard against real Git trees."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from tests.unit.scripts.clutter_guard._test_types import GuardTestCase, JsonGuardTestCase
from tests.unit.scripts.clutter_guard.helpers import (
    add_agent_file,
    add_agent_folder,
    add_binary_file,
    add_case_with_digest_alias,
    add_case_with_digest_alias_and_decision,
    add_decision,
    add_github_token,
    add_hex_and_recorded_setup,
    add_home_path,
    add_identifiers,
    add_ignored_clutter,
    add_lower_case_decision_names,
    add_placeholder_paths,
    add_product_named_source,
    add_release_changelog,
    add_review_record,
    add_reviewer_word,
    add_run_record,
    add_secret_variable,
    add_sql_identifier_with_stream_word,
    add_sql_system_function,
    add_stream_word,
    add_top_level_folder,
    add_unknown_dialect,
    add_unknown_suite,
    add_worklist,
    force_add_ignored_agent_file,
    guard_after,
    unchanged,
)

_STREAM_CLASS: str = "la" + "ne-reference"
_REVIEWER_CLASS: str = "audit" + "or-reference"


@pytest.mark.parametrize(
    "test_case",
    (
        GuardTestCase(
            description="a clean tree passes",
            change=unchanged,
            expected_return_code=0,
            expected_fragment="passed, 0 findings in 4 tracked files",
        ),
        GuardTestCase(
            description="a source file named after a product concept passes",
            change=add_product_named_source,
            expected_return_code=0,
            expected_fragment="passed, 0 findings in 5 tracked files",
        ),
        GuardTestCase(
            description="the release changelog at the top level passes",
            change=add_release_changelog,
            expected_return_code=0,
            expected_fragment="passed, 0 findings in 5 tracked files",
        ),
        GuardTestCase(
            description="an XPath row path and a placeholder user's file URL pass",
            change=add_placeholder_paths,
            expected_return_code=0,
            expected_fragment="passed, 0 findings in 5 tracked files",
        ),
        GuardTestCase(
            description="agent and work-note files the repository's .gitignore ignores stay untracked",
            change=add_ignored_clutter,
            expected_return_code=0,
            expected_fragment="passed, 0 findings in 5 tracked files",
        ),
        GuardTestCase(
            description="an ignored agent file force-added past .gitignore is rejected",
            change=force_add_ignored_agent_file,
            expected_return_code=1,
            expected_fragment="AGENTS.md:0: agent-file: AGENTS.md",
        ),
        GuardTestCase(
            description="a top-level folder outside the layout is rejected",
            change=add_top_level_folder,
            expected_return_code=1,
            expected_fragment="notes/plan.md:0: path-not-allowed: notes",
        ),
        GuardTestCase(
            description="a suite folder outside the layout is rejected",
            change=add_unknown_suite,
            expected_return_code=1,
            expected_fragment="suites/misc/cases.toml:0: path-not-allowed: suites/misc",
        ),
        GuardTestCase(
            description="an unknown dialect folder under a dialect suite is rejected",
            change=add_unknown_dialect,
            expected_return_code=1,
            expected_fragment="path-not-allowed: suites/convert/oracle",
        ),
        GuardTestCase(
            description="an agent instruction file in any case is rejected",
            change=add_agent_file,
            expected_return_code=1,
            expected_fragment="scripts/sub/Agents.md:0: agent-file: Agents.md",
        ),
        GuardTestCase(
            description="a file in an agent tool folder is rejected",
            change=add_agent_folder,
            expected_return_code=1,
            expected_fragment="agent-file: .cursor",
        ),
        GuardTestCase(
            description="a worklist is rejected",
            change=add_worklist,
            expected_return_code=1,
            expected_fragment="process-note-file: settled-worklist.md",
        ),
        GuardTestCase(
            description="a run record is rejected",
            change=add_run_record,
            expected_return_code=1,
            expected_fragment="process-note-file: runs.toml",
        ),
        GuardTestCase(
            description="a work stream's review records are rejected",
            change=add_review_record,
            expected_return_code=1,
            expected_fragment="process-note-file",
        ),
        GuardTestCase(
            description="a decision number in prose is rejected",
            change=add_decision,
            expected_return_code=1,
            expected_fragment="docs/notes.md:1: decision-reference:",
        ),
        GuardTestCase(
            description="a digest size as a SQL alias and a recorded column name passes",
            change=add_case_with_digest_alias,
            expected_return_code=0,
            expected_fragment="passed, 0 findings",
        ),
        GuardTestCase(
            description="a decision number in a case description still fails beside a SQL alias",
            change=add_case_with_digest_alias_and_decision,
            expected_return_code=1,
            expected_fragment="sha2.toml:1: decision-reference:",
        ),
        GuardTestCase(
            description="stream, decision and reviewer references inside identifiers are rejected",
            change=add_identifiers,
            expected_return_code=1,
            expected_fragment="FAILED, 3 findings in 6 tracked files",
        ),
        GuardTestCase(
            description="lower-case decision numbers in run, packet and measure names are rejected",
            change=add_lower_case_decision_names,
            expected_return_code=1,
            expected_fragment="FAILED, 5 findings in 5 tracked files",
        ),
        GuardTestCase(
            description="hex identifiers, a case id segment and recorded setup SQL pass",
            change=add_hex_and_recorded_setup,
            expected_return_code=0,
            expected_fragment="passed, 0 findings in 6 tracked files",
        ),
        GuardTestCase(
            description="the stream word inside SQL and recorded column names passes",
            change=add_sql_identifier_with_stream_word,
            expected_return_code=0,
            expected_fragment="passed, 0 findings in 6 tracked files",
        ),
        GuardTestCase(
            description="a work stream named in prose is rejected",
            change=add_stream_word,
            expected_return_code=1,
            expected_fragment=f"docs/notes.md:1: {_STREAM_CLASS}:",
        ),
        GuardTestCase(
            description="a reviewer role named in prose is rejected",
            change=add_reviewer_word,
            expected_return_code=1,
            expected_fragment=f"docs/notes.md:1: {_REVIEWER_CLASS}:",
        ),
        GuardTestCase(
            description="an absolute home path is rejected",
            change=add_home_path,
            expected_return_code=1,
            expected_fragment="docs/notes.md:1: home-path:",
        ),
        GuardTestCase(
            description="a read of a secret-ish process setting is rejected",
            change=add_secret_variable,
            expected_return_code=1,
            expected_fragment="scripts/client.py:1: secret-variable-name:",
        ),
        GuardTestCase(
            description="the standard GitHub token passes",
            change=add_github_token,
            expected_return_code=0,
            expected_fragment="passed, 0 findings",
        ),
        GuardTestCase(
            description="a SQL system function with a secret-ish part passes",
            change=add_sql_system_function,
            expected_return_code=0,
            expected_fragment="passed, 0 findings",
        ),
        GuardTestCase(
            description="a binary file is not read",
            change=add_binary_file,
            expected_return_code=0,
            expected_fragment="passed, 0 findings",
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_tracked_tree_when_guarding_then_only_clutter_fails(
    test_case: GuardTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    return_code: int = guard_after(repository=tmp_path, change=test_case.change)

    assert return_code == test_case.expected_return_code
    assert test_case.expected_fragment in capsys.readouterr().out


@pytest.mark.parametrize(
    "test_case",
    (
        JsonGuardTestCase(
            description="a home path is counted under its class alone",
            change=add_home_path,
            expected_counts={
                "path-not-allowed": 0,
                "agent-file": 0,
                "process-note-file": 0,
                "decision-reference": 0,
                _STREAM_CLASS: 0,
                _REVIEWER_CLASS: 0,
                "home-path": 1,
                "secret-variable-name": 0,
            },
            expected_findings=[
                {
                    "path": "docs/notes.md",
                    "line": 1,
                    "class": "home-path",
                    "snippet": "Data lives in " + "/" + "home/alice/data.csv.",
                }
            ],
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_clutter_when_guarding_as_json_then_each_class_is_counted(
    test_case: JsonGuardTestCase, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    return_code: int = guard_after(
        repository=tmp_path, change=test_case.change, arguments=("--json",)
    )

    document: dict[str, object] = json.loads(capsys.readouterr().out)
    assert return_code == 1
    assert document["counts"] == test_case.expected_counts
    assert document["findings"] == test_case.expected_findings
