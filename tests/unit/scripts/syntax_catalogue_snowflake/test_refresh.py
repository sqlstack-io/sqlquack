"""`--refresh` rebuilds only the cached pages and keeps every other page as committed; a changed
form keeps the id a collision minted from its natural id."""

from __future__ import annotations

import gzip
import json
from pathlib import Path

import pytest

from scripts.grammar_catalogue._helpers.forms import minted_from
from scripts.syntax_catalogue_snowflake.main.build import run_build
from tests.unit.scripts.syntax_catalogue_snowflake._test_types import (
    MintedIdTestCase,
    RefreshTestCase,
)
from tests.unit.scripts.syntax_catalogue_snowflake.helpers import toml_rows

PAGE_PATH: str = "sql-reference/functions/task_history.md"
PAGE_SHA256: str = "a" * 64
NEW_GRAMMAR: str = "TASK_HISTORY( [ RESULT_LIMIT => <integer> ] [, V1_ID => '<string>' ] )"
SOURCES: str = """
[[source]]
id = "doc.functions.abs"
url = "https://docs.snowflake.com/en/sql-reference/functions/abs"
title = "ABS"
family = "functions"
retrieved_utc = "2026-11-03T23:20:45Z"
page_sha256 = "223e6a5f8a552e2beec547662a5261fedd2ef15ee49638b3cf3257928b0af267"
syntax_sha256 = "6e9dc1b66e64d261a5edbfb7367edae79bf4f65959afcc308635bd47aa860605"
syntax_blocks = 1
status = "forms"
decisions = []

[[source]]
id = "doc.functions.task_history"
url = "https://docs.snowflake.com/en/sql-reference/functions/task_history"
title = "TASK_HISTORY"
family = "functions"
retrieved_utc = "2026-11-03T23:21:17Z"
page_sha256 = "9d0938aa8339716ed287aa131b9b1cbecc940ae74f881494fb3156ec63657b05"
syntax_sha256 = "c0e196e3f5a3a21b6b027a073d6efd171991df84ccb0722654f53a80f418626e"
syntax_blocks = 1
status = "forms"
decisions = []
"""
CATALOGUE: str = """
[[form]]
id = "sf.functions.abs"
family = "functions"
source = "doc.functions.abs"
anchor = "Syntax"
kind = "call"
grammar = ["ABS( <num_expr> )"]
grammar_sha256 = "6e9dc1b66e64d261a5edbfb7367edae79bf4f65959afcc308635bd47aa860605"
uses = []
status = "catalogued"

[[form]]
id = "sf.functions.task-history"
family = "functions"
source = "doc.functions.task_history"
anchor = "Syntax"
kind = "special"
grammar = ["TASK_HISTORY( [ RESULT_LIMIT => <integer> ] )"]
grammar_sha256 = "c0e196e3f5a3a21b6b027a073d6efd171991df84ccb0722654f53a80f418626e"
uses = []
status = "catalogued"
"""


@pytest.mark.parametrize(
    "test_case",
    (
        RefreshTestCase(
            description="a page whose syntax gained an argument",
            grammar=NEW_GRAMMAR,
            expected_ids=("sf.functions.abs", "sf.functions.task-history"),
            expected_page_sha256=PAGE_SHA256,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_one_changed_page_when_refreshing_then_only_its_rows_change_and_ids_stay(
    tmp_path: Path, test_case: RefreshTestCase
) -> None:
    out: Path = tmp_path / "out"
    (out / "catalogue").mkdir(parents=True)
    (out / "sources.toml").write_text(SOURCES)
    (out / "catalogue" / "functions.toml").write_text(CATALOGUE)
    (out / "hand.toml").write_text("")
    (out / "aliases.toml").write_text("")
    cache: Path = tmp_path / "cache"
    (cache / "raw" / PAGE_PATH).parent.mkdir(parents=True)
    page: str = f"# TASK_HISTORY\n\n## Syntax\n\n```sqlsyntax\n{test_case.grammar}\n```\n"
    (cache / "raw" / f"{PAGE_PATH}.gz").write_bytes(gzip.compress(page.encode()))
    record: dict[str, object] = {
        "url": f"https://docs.snowflake.com/en/{PAGE_PATH}",
        "path": PAGE_PATH,
        "status": 200,
        "retrieved_utc": "2026-11-14T00:00:00Z",
        "sha256": PAGE_SHA256,
        "format": "markdown",
    }
    (cache / "fetch-log.jsonl").write_text(json.dumps(record) + "\n")
    before_sources: dict[str, dict[str, object]] = toml_rows(out / "sources.toml", "source")
    before_forms: dict[str, dict[str, object]] = toml_rows(
        out / "catalogue" / "functions.toml", "form"
    )

    assert run_build(["--refresh", "--cache", str(cache), "--out", str(out)]) == 0

    sources: dict[str, dict[str, object]] = toml_rows(out / "sources.toml", "source")
    forms: dict[str, dict[str, object]] = toml_rows(out / "catalogue" / "functions.toml", "form")
    assert (sources["doc.functions.abs"], forms["sf.functions.abs"]) == (
        before_sources["doc.functions.abs"],
        before_forms["sf.functions.abs"],
    )
    assert (
        sources["doc.functions.task_history"]["page_sha256"],
        tuple(sorted(forms)),
        forms["sf.functions.task-history"]["grammar"],
    ) == (test_case.expected_page_sha256, test_case.expected_ids, [test_case.grammar])


@pytest.mark.parametrize(
    "test_case",
    (
        MintedIdTestCase(
            description="the heading a collision appended",
            old="sf.security_admin.create-secret.syntax",
            natural="sf.security_admin.create-secret",
            expected_minted=True,
        ),
        MintedIdTestCase(
            description="the number a collision appended",
            old="sf.security_admin.create-secret-2",
            natural="sf.security_admin.create-secret",
            expected_minted=True,
        ),
        MintedIdTestCase(
            description="the natural id itself",
            old="sf.security_admin.create-secret",
            natural="sf.security_admin.create-secret",
            expected_minted=False,
        ),
        MintedIdTestCase(
            description="a longer variant that shares the prefix",
            old="sf.security_admin.create-secret-type-password",
            natural="sf.security_admin.create-secret",
            expected_minted=False,
        ),
        MintedIdTestCase(
            description="a nested id under the natural id",
            old="sf.security_admin.create-secret.syntax.more",
            natural="sf.security_admin.create-secret",
            expected_minted=False,
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_an_earlier_id_when_matching_then_only_a_collision_suffix_keeps_it(
    test_case: MintedIdTestCase,
) -> None:
    assert minted_from(old=test_case.old, natural=test_case.natural) is test_case.expected_minted
