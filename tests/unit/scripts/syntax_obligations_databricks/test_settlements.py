"""An observed case takes its receipts' expect only where the kinds of derived obligations allow.

No committed form gives an observed case that a derived positive-kind obligation cites and the
receipts settle (those observed today are documented Runtime-only positives, which never settle), so
the build is tested with a derived negative, beside a hand-written one, and the guard with every kind.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from scripts.grammar_obligations.types import Row
from scripts.syntax_obligations_databricks._helpers.cases import kind_settlement
from tests.unit.scripts.syntax_obligations_databricks._test_types import (
    KindSettlementTestCase,
    PreviewSettlementTestCase,
    SettlementBuildTestCase,
)
from tests.unit.scripts.syntax_obligations_databricks.helpers import (
    build_error,
    family_cases,
    observed_negatives,
    small_root,
)


@pytest.mark.parametrize(
    "test_case",
    (
        SettlementBuildTestCase(
            description="a semantic error receipt settles only the hand-written negative",
            family="ddl",
            phase="semantic_error",
            expected_derived_expect="observe",
            expected_written=("accept", "semantic"),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_parsed_receipts_when_built_then_only_the_derived_negative_stays_observed(
    test_case: SettlementBuildTestCase, tmp_path: Path
) -> None:
    root: Path = small_root(directory=tmp_path, limits="", retired="", family=test_case.family)
    assert build_error(root) == ""
    before: dict[str, Row] = family_cases(root, test_case.family)
    derived, written = observed_negatives(root, before, test_case.family)
    (root / "receipts").mkdir()
    (root / "receipts" / f"{test_case.family}.jsonl").write_text(
        "".join(
            json.dumps(
                {
                    "case_id": case,
                    "sql_sha256": before[case]["sql_sha256"],
                    "setup_sha256": before[case]["setup_sha256"],
                    "session_sha256": before[case]["session_sha256"],
                    "phase": test_case.phase,
                }
            )
            + "\n"
            for case in (derived, written)
        )
    )

    assert build_error(root) == ""

    after: dict[str, Row] = family_cases(root, test_case.family)
    assert after[derived]["expect"] == test_case.expected_derived_expect
    assert (after[written]["expect"], after[written].get("stage")) == test_case.expected_written


@pytest.mark.parametrize(
    "test_case",
    (
        KindSettlementTestCase(
            description="a positive refutes a reject",
            kinds={"positive"},
            expect="observe",
            settlement=("reject", ""),
            expected_settlement=None,
        ),
        KindSettlementTestCase(
            description="an identifier class refutes a reject",
            kinds={"identifier_class"},
            expect="observe",
            settlement=("reject", ""),
            expected_settlement=None,
        ),
        KindSettlementTestCase(
            description="an alternative with an option refutes a reject",
            kinds={"alternative", "option_present"},
            expect="observe",
            settlement=("reject", ""),
            expected_settlement=None,
        ),
        KindSettlementTestCase(
            description="a nesting depth refutes a reject",
            kinds={"nesting_depth"},
            expect="observe",
            settlement=("reject", ""),
            expected_settlement=None,
        ),
        KindSettlementTestCase(
            description="a negative neighbour refutes an accept",
            kinds={"negative_neighbour"},
            expect="observe",
            settlement=("accept", "semantic"),
            expected_settlement=None,
        ),
        KindSettlementTestCase(
            description="a negative beside a positive refutes a reject",
            kinds={"negative_neighbour", "positive"},
            expect="observe",
            settlement=("reject", ""),
            expected_settlement=None,
        ),
        KindSettlementTestCase(
            description="a negative neighbour agrees with a reject",
            kinds={"negative_neighbour"},
            expect="observe",
            settlement=("reject", ""),
            expected_settlement=("reject", ""),
        ),
        KindSettlementTestCase(
            description="an identifier class agrees with an accept",
            kinds={"identifier_class"},
            expect="observe",
            settlement=("accept", "semantic"),
            expected_settlement=("accept", "semantic"),
        ),
        KindSettlementTestCase(
            description="no citing kind keeps the settlement",
            kinds=set(),
            expect="observe",
            settlement=("reject", ""),
            expected_settlement=("reject", ""),
        ),
        KindSettlementTestCase(
            description="a case not observed keeps the settlement",
            kinds={"positive"},
            expect="accept",
            settlement=("reject", ""),
            expected_settlement=("reject", ""),
        ),
    ),
    ids=lambda case: case.description,
)
def test_given_a_settlement_when_derived_kinds_cite_the_case_then_only_an_agreeing_one_stands(
    test_case: KindSettlementTestCase,
) -> None:
    case: Row = {"case_id": "db.f.k1", "expect": test_case.expect}

    found: tuple[str, str] | None = kind_settlement(
        case=case, settlement=test_case.settlement, kinds=test_case.kinds
    )

    assert found == test_case.expected_settlement


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
