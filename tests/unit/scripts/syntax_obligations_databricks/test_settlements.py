"""An observed case takes its receipts' expect only where the kinds of derived obligations allow."""

from __future__ import annotations

import json
from pathlib import Path

from scripts.grammar_obligations.types import Row
from tests.unit.scripts.syntax_obligations_databricks.helpers import build_error, small_root

SCRIPTING: str = "scripting"
# A derived negative neighbour of SIGNAL (o015) and a hand [[limit]] row of the same form, both
# observed until a receipt answers.
DERIVED_NEGATIVE: str = "db.scripting.signal-stmt.k85ff2428eb59"
HAND_LIMIT: str = "db.scripting.signal-stmt.k23844fe95906"


def _cases(root: Path) -> dict[str, Row]:
    path: Path = root / "cases" / f"{SCRIPTING}.jsonl"
    return {str(row["case_id"]): row for row in map(json.loads, path.read_text().splitlines())}


def test_given_parsed_receipts_when_built_then_only_the_derived_negative_stays_observed(
    tmp_path: Path,
) -> None:
    root: Path = small_root(directory=tmp_path, limits="", retired="", family=SCRIPTING)
    assert build_error(root) == ""
    before: dict[str, Row] = _cases(root)
    assert [before[case]["expect"] for case in (DERIVED_NEGATIVE, HAND_LIMIT)] == [
        "observe",
        "observe",
    ]
    (root / "receipts").mkdir()
    (root / "receipts" / f"{SCRIPTING}.jsonl").write_text(
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
            for case in (DERIVED_NEGATIVE, HAND_LIMIT)
        )
    )

    assert build_error(root) == ""

    after: dict[str, Row] = _cases(root)
    assert after[DERIVED_NEGATIVE]["expect"] == "observe"
    assert (after[HAND_LIMIT]["expect"], after[HAND_LIMIT].get("stage")) == ("accept", "semantic")
