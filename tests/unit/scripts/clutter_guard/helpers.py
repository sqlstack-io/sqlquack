"""A throwaway Git repository with a clean tree, and the clutter added to it.

The guard scans this file too, so every forbidden word or path is assembled at run time.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

from scripts.clutter_guard.main.check import run_clutter_guard
from tests.unit.scripts.clutter_guard._test_types import Change

_DECISION: str = "D" + "133"
_DIGEST_ALIAS: str = "D" + "256"
_STREAM: str = "la" + "ne"
_REVIEWER: str = "audit" + "or"
_HOME: str = "/" + "home/alice/"
_SECRET_NAME: str = "SERVICE_" + "TOKEN"
_GITHUB_SECRET_NAME: str = "GITHUB_" + "TOKEN"
_DELIVERY_SECRET_NAME: str = "CHIO_DELIVERY_" + "PRIVATE_KEY"
_REPOSITORY_GITIGNORE: Path = Path(__file__).resolve().parents[4] / ".gitignore"
_IGNORED_CLUTTER: tuple[str, ...] = (
    "AGENTS.md",
    "docs/CLAUDE.md",
    ".agents/setup",
    ".amp/notes.md",
    "coverage/worklists/settled.md",
    f"coverage/{_STREAM}-reviews/one.toml",
    "suites/syntax/duckdb/runs.toml",
    "docs/run-notes.md",
)
_CLEAN_FILES: dict[str, str] = {
    "README.md": "# Example\n\nA clean tree.\n",
    "scripts/tool.py": '"""A tool."""\n\nTOKEN_PATTERN: str = "x"\n',
    "suites/syntax/duckdb/cases.toml": 'query = "SELECT 1"\n',
    "docs/guide.md": "Use `make check` to run the gate.\n",
}


def _git(repository: Path, *arguments: str) -> None:
    subprocess.run(
        ("git", "-c", "user.name=Test", "-c", "user.email=test@example.com", *arguments),
        cwd=repository,
        check=True,
        capture_output=True,
    )


def _write(repository: Path, path: str, text: str) -> None:
    file: Path = repository / path
    file.parent.mkdir(parents=True, exist_ok=True)
    file.write_text(text, encoding="utf-8")


def unchanged(_: Path) -> None:
    """Adds nothing to the clean tree."""


def add_top_level_folder(repository: Path) -> None:
    """Adds a top-level folder outside the layout."""
    _write(repository, "notes/plan.md", "Plans.\n")


def add_release_changelog(repository: Path) -> None:
    """Adds the changelog the release tooling writes at the top level."""
    _write(
        repository,
        "CHANGELOG.md",
        "# Changelog\n\n## 0.3.0 (2026-11-10)\n\n### Features\n\n* convert more\n",
    )


def add_unknown_suite(repository: Path) -> None:
    """Adds a suite folder outside the layout."""
    _write(repository, "suites/misc/cases.toml", "\n")


def add_unknown_dialect(repository: Path) -> None:
    """Adds a dialect folder no dialect suite knows."""
    _write(repository, "suites/convert/oracle/cases.toml", "\n")


def add_agent_file(repository: Path) -> None:
    """Adds an agent instruction file, in mixed case, deep in the tree."""
    _write(repository, "scripts/sub/Agents.md", "Instructions.\n")


def add_agent_folder(repository: Path) -> None:
    """Adds a file inside an agent tool folder."""
    _write(repository, "docs/.cursor/rules.md", "Rules.\n")


def add_worklist(repository: Path) -> None:
    """Adds a worklist."""
    _write(repository, "docs/settled-worklist.md", "- one\n")


def add_run_record(repository: Path) -> None:
    """Adds a run record."""
    _write(repository, "suites/syntax/duckdb/runs.toml", "\n")


def add_review_record(repository: Path) -> None:
    """Adds a folder of a parallel work stream's review records."""
    _write(repository, f"docs/{_STREAM}-reviews/one.toml", "\n")


def add_decision(repository: Path) -> None:
    """Adds prose that cites a decision number."""
    _write(repository, "docs/notes.md", f"Settled this way ({_DECISION}).\n")


def add_case_with_digest_alias(repository: Path) -> None:
    """Adds a case whose SQL aliases a column by a digest size, and its recorded column name."""
    _write(
        repository,
        "suites/convert/snowflake/sha2.toml",
        f'description = "Each digest size"\nquery = "SELECT SHA2(a, 256) AS {_DIGEST_ALIAS}"\n',
    )
    _write(repository, "suites/convert/snowflake/result.json", f'{{"name": "{_DIGEST_ALIAS}"}}\n')


def add_case_with_digest_alias_and_decision(repository: Path) -> None:
    """Adds a case with the same SQL alias and a description citing a decision number."""
    _write(
        repository,
        "suites/convert/snowflake/sha2.toml",
        f'description = "Settled ({_DECISION})"\n'
        f'query = "SELECT SHA2(a, 256) AS {_DIGEST_ALIAS}"\n',
    )


def add_identifiers(repository: Path) -> None:
    """Adds constants named after a work stream and a decision, and a test after a reviewer."""
    _write(
        repository,
        "scripts/settings.py",
        f"{_STREAM.upper()}_A_SESSION: int = 1\n{_DECISION}_MARK: str = 'x'\n",
    )
    _write(repository, "tools/check/tests/test_fix.rs", f"fn given_{_REVIEWER}_findings() {{}}\n")


def add_lower_case_decision_names(repository: Path) -> None:
    """Adds run, packet, check and measure names that carry a lower-case decision number."""
    number: str = "d" + "82"
    _write(
        repository,
        "docs/runs.md",
        f"Recorded by run {number}-scripting-2.\n"
        f"Held by the capture guard in packet {number}:\n"
        f"Answered otherwise in cat-{number} (import 155).\n"
        f"Snowflake parses it ({number}-query-1, semantic_error).\n"
        f'measure = "{number}_missing_object"\n',
    )


def add_hex_and_recorded_setup(repository: Path) -> None:
    """Adds hex identifiers, a case id segment and a recorded setup that only look alike."""
    number: str = "d" + "952"
    _write(
        repository,
        "docs/ids.md",
        f"uuid 5feceb66-ffc8-6f38-{number}-786c6d696c79 and ffff:8883:{number}:7d6b\n"
        f'case "c-ddl-{number}-stage-with-tag"\n'
        f'"id": "call: dateDiff(\'day\', {number}, {number})"\n',
    )
    _write(
        repository,
        "suites/syntax/snowflake/arity.jsonl",
        f'{{"case_id":"x","setup":["CREATE DATABASE db COMMENT = \'{number} run\'","USE db"]}}\n',
    )


def add_sql_identifier_with_stream_word(repository: Path) -> None:
    """Adds SQL and a recorded column name that contain the stream word as data."""
    _write(
        repository,
        "corpora/upstream.sql",
        f"SELECT arrival_from_{_STREAM}_history FROM shipments;\n",
    )
    _write(
        repository,
        "suites/referee/snowflake/result.json",
        f'{{"name": "{_STREAM.upper()}_MEDIAN_TRANSIT_HOURS"}}\n',
    )


def add_stream_word(repository: Path) -> None:
    """Adds prose naming a parallel work stream."""
    _write(repository, "docs/notes.md", f"Reviewed by the other {_STREAM.title()}.\n")


def add_reviewer_word(repository: Path) -> None:
    """Adds prose naming a reviewer role."""
    _write(repository, "docs/notes.md", f"Listed for the {_REVIEWER}s.\n")


def add_home_path(repository: Path) -> None:
    """Adds an absolute home path."""
    _write(repository, "docs/notes.md", f"Data lives in {_HOME}data.csv.\n")


def add_product_named_source(repository: Path) -> None:
    """Adds a source file whose name is a product concept that shares a note word."""
    _write(repository, "tools/check/src/results/worklist.rs", "pub fn gaps() {}\n")


def add_placeholder_paths(repository: Path) -> None:
    """Adds SQL with an XPath row path and a placeholder user's file URL."""
    _write(
        repository,
        "corpora/upstream.sql",
        "SELECT * FROM XMLTABLE('/" + "root/data' PASSING x);\n"
        "PUT file:///" + "Users/test/projects/data.csv @stage;\n",
    )


def add_secret_variable(repository: Path) -> None:
    """Adds a read of a secret-ish process setting."""
    _write(repository, "scripts/client.py", f'token = os.environ["{_SECRET_NAME}"]\n')


def add_github_token(repository: Path) -> None:
    """Adds a workflow that passes the standard GitHub token."""
    _write(
        repository,
        ".github/workflows/ci.yml",
        f"env:\n  {_GITHUB_SECRET_NAME}: ${{{{ secrets.{_GITHUB_SECRET_NAME} }}}}\n",
    )


def _delivery_workflow(triggers: str) -> str:
    """A workflow with `triggers` that reads the delivery App key, mentioning pull_request runs."""
    return (
        f"on:\n{triggers}\n"
        "# A pull request it opens starts no pull_request runs.\n"
        "jobs:\n  release:\n    steps:\n      - with:\n"
        f"          private-key: ${{{{ secrets.{_DELIVERY_SECRET_NAME} }}}}\n"
    )


def add_release_workflow_delivery_key(repository: Path) -> None:
    """Adds the delivery App key to the release workflow, on push and dispatch only."""
    _write(
        repository,
        ".github/workflows/release-please.yml",
        _delivery_workflow("  push:\n  workflow_dispatch:"),
    )


def add_delivery_key_in_another_workflow(repository: Path) -> None:
    """Adds the delivery App key to a workflow other than the release workflow."""
    _write(repository, ".github/workflows/ci.yml", _delivery_workflow("  push:"))


def add_delivery_key_with_pull_request_trigger(repository: Path) -> None:
    """Adds the delivery App key to the release workflow, which also runs on pull requests."""
    _write(
        repository,
        ".github/workflows/release-please.yml",
        _delivery_workflow("  push:\n  pull_request:"),
    )


def add_delivery_key_with_pull_request_target_trigger(repository: Path) -> None:
    """Adds the delivery App key to the release workflow, which also runs on pull_request_target."""
    _write(
        repository,
        ".github/workflows/release-please.yml",
        _delivery_workflow("  push:\n  pull_request_target:\n    types: [opened]"),
    )


def add_sql_system_function(repository: Path) -> None:
    """Adds SQL calling a Snowflake system function whose name has a secret-ish part."""
    _write(repository, "docs/sql.md", f"SELECT SYSTEM${_SECRET_NAME}();\n")


def add_binary_file(repository: Path) -> None:
    """Adds a binary file whose bytes happen to spell forbidden text."""
    file: Path = repository / "docs/image.bin"
    file.write_bytes(b"\0" + f"{_STREAM} {_HOME}".encode())


def add_ignored_clutter(repository: Path) -> None:
    """Adds the repository's .gitignore and agent and work-note files it ignores."""
    _write(repository, ".gitignore", _REPOSITORY_GITIGNORE.read_text(encoding="utf-8"))
    for path in _IGNORED_CLUTTER:
        _write(repository, path, "Notes.\n")


def force_add_ignored_agent_file(repository: Path) -> None:
    """Adds the same ignored files, then force-adds the agent instruction file past .gitignore."""
    add_ignored_clutter(repository)
    _git(repository, "add", "--force", "AGENTS.md")


def guard_after(*, repository: Path, change: Change, arguments: tuple[str, ...] = ()) -> int:
    """Commit a clean tree, apply `change` on top, and run the guard on the repository."""
    _git(repository, "init", "--quiet", "--initial-branch=main")
    for path, text in _CLEAN_FILES.items():
        _write(repository, path, text)
    change(repository)
    _git(repository, "add", "-A")
    return run_clutter_guard(["--root", str(repository), *arguments])
