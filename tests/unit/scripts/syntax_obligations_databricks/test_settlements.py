"""An observed case takes its receipts' expect only where the kinds of derived obligations allow.

No committed form gives an observed case that a derived positive-kind obligation cites and the
receipts settle (those observed today are documented Runtime-only positives, which never settle), so
the build is tested with a derived negative, beside a hand-written one, and the guard with every kind.
"""

from __future__ import annotations

import json
import tomllib
from pathlib import Path

import pytest

from scripts.grammar_obligations.types import Row
from scripts.syntax_obligations_databricks._helpers.cases import kind_settlement
from tests.unit.scripts.syntax_obligations_databricks._test_types import PreviewSettlementTestCase
from tests.unit.scripts.syntax_obligations_databricks.helpers import build_error, small_root

FAMILY: str = "ddl"
NEGATIVE_KINDS: frozenset[str] = frozenset({"negative_neighbour", "required_operand_missing"})


def _cases(root: Path) -> dict[str, Row]:
    path: Path = root / "cases" / f"{FAMILY}.jsonl"
    return {str(row["case_id"]): row for row in map(json.loads, path.read_text().splitlines())}


def _observed_negatives(root: Path, cases: dict[str, Row]) -> tuple[str, str]:
    """An observed case only derived negatives cite, and one a hand [[limit]] negative renders."""
    rows: list[Row] = tomllib.loads((root / "obligations" / f"{FAMILY}.toml").read_text())[
        "obligation"
    ]
    kinds: dict[str, str] = {str(row["id"]): str(row["kind"]) for row in rows}
    limits: list[Row] = tomllib.loads((root / "limits.toml").read_text())["limit"]
    hand: set[str] = {" ".join(str(row["sql"]).split()) for row in limits if "sql" in row}
    derived: list[str] = []
    written: list[str] = []
    for case_id, case in sorted(cases.items()):
        cited: set[str] = {kinds[str(identifier)] for identifier in case["obligations"]}
        if case["expect"] != "observe" or not cited <= NEGATIVE_KINDS:
            continue
        (written if case["sql"] in hand else derived).append(case_id)
    return derived[0], written[0]


def test_given_parsed_receipts_when_built_then_only_the_derived_negative_stays_observed(
    tmp_path: Path,
) -> None:
    root: Path = small_root(directory=tmp_path, limits="", retired="", family=FAMILY)
    assert build_error(root) == ""
    before: dict[str, Row] = _cases(root)
    derived, written = _observed_negatives(root, before)
    (root / "receipts").mkdir()
    (root / "receipts" / f"{FAMILY}.jsonl").write_text(
        "".join(
            json.dumps(
                {
                    "case_id": case,
                    "sql_sha256": before[case]["sql_sha256"],
                    "setup_sha256": before[case]["setup_sha256"],
                    "session_sha256": before[case]["session_sha256"],
                    "phase": "semantic_error",
                }
            )
            + "\n"
            for case in (derived, written)
        )
    )

    assert build_error(root) == ""

    after: dict[str, Row] = _cases(root)
    assert after[derived]["expect"] == "observe"
    assert (after[written]["expect"], after[written].get("stage")) == ("accept", "semantic")


@pytest.mark.parametrize(
    ("kinds", "expect", "settlement", "expected"),
    [
        ({"positive"}, "observe", ("reject", ""), None),
        ({"identifier_class"}, "observe", ("reject", ""), None),
        ({"alternative", "option_present"}, "observe", ("reject", ""), None),
        ({"nesting_depth"}, "observe", ("reject", ""), None),
        ({"negative_neighbour"}, "observe", ("accept", "semantic"), None),
        ({"negative_neighbour", "positive"}, "observe", ("reject", ""), None),
        ({"negative_neighbour"}, "observe", ("reject", ""), ("reject", "")),
        ({"identifier_class"}, "observe", ("accept", "semantic"), ("accept", "semantic")),
        (set(), "observe", ("reject", ""), ("reject", "")),
        ({"positive"}, "accept", ("reject", ""), ("reject", "")),
    ],
)
def test_given_a_settlement_when_derived_kinds_cite_the_case_then_only_an_agreeing_one_stands(
    kinds: set[str],
    expect: str,
    settlement: tuple[str, str],
    expected: tuple[str, str] | None,
) -> None:
    case: Row = {"case_id": "db.f.k1", "expect": expect}

    assert kind_settlement(case=case, settlement=settlement, kinds=kinds) == expected


@pytest.mark.parametrize(
    "test_case",
    (
        PreviewSettlementTestCase(
            description="a form behind a workspace preview stays observed",
            case_id="db.expressions.decfloat-type.k1",
            expected_settlement=None,
        ),
        PreviewSettlementTestCase(
            description="a form without a preview takes its receipts' settlement",
            case_id="db.expressions.decimal-type.k1",
            expected_settlement=("reject", ""),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_receipts_that_settle_when_the_form_is_behind_a_preview_then_none_stands(
    test_case: PreviewSettlementTestCase,
) -> None:
    case: Row = {"case_id": test_case.case_id, "expect": "observe"}

    found: tuple[str, str] | None = kind_settlement(
        case=case, settlement=("reject", ""), kinds=set()
    )

    assert found == test_case.expected_settlement
