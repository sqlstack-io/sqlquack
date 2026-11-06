"""Cases of the PostgreSQL root: one per distinct rendered SQL text of the focus forms."""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

from scripts.syntax_obligations._helpers.cases import canonical, digest, observed_stage
from scripts.syntax_obligations._helpers.render import style_of, submissions
from scripts.syntax_obligations.constants import KIND_EXTREME
from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.models import Draft, Node, ReceiptView, Rendered
from scripts.syntax_obligations.types import Row
from scripts.syntax_obligations_pg.constants import (
    AUTOCOMMIT_LEADS,
    CAPTURE_AUTOCOMMIT,
    CAPTURE_ROLLED_BACK,
    DIGEST_WIDTH,
    SESSION,
    STAGE_KEY,
)

WORD: re.Pattern[str] = re.compile(r"[A-Za-z_][A-Za-z0-9_]*|\S")


def focus_forms(path: Path) -> list[str]:
    """The form ids of a focus file, in its order."""
    return [str(row["form"]) for row in tomllib.loads(path.read_text()).get("focus", [])]


def capture_for(sql: str) -> str:
    """The capture the statement's leading words call for, as the referee decides it."""
    words: tuple[str, ...] = tuple(word.upper() for word in WORD.findall(sql))
    autocommit: bool = any(words[: len(lead)] == lead for lead in AUTOCOMMIT_LEADS)
    return CAPTURE_AUTOCOMMIT if autocommit else CAPTURE_ROLLED_BACK


def family_cases(
    *,
    family: str,
    numbered: list[tuple[str, Draft]],
    trees: dict[str, list[tuple[Node, ...]]],
    forms: set[str],
    receipts: ReceiptView,
) -> list[Row]:
    """The cases of one family's forms in `forms`, each citing every obligation it serves; an
    accepted case takes its receipt's later stage and an observed one its receipt's outcome,
    as the shared builder settles them (decisions D92, D133)."""
    by_sql: dict[str, tuple[str, Rendered, list[str]]] = {}
    extremes: set[str] = set()
    conflicts: list[str] = []
    for identifier, draft in numbered:
        if draft.form not in forms or draft.spec is None:
            continue
        found: list[Rendered] = submissions(
            trees=trees[draft.form],
            style=style_of(family=family, form=draft.form),
            spec=draft.spec,
            depths=draft.depths,
        )
        if not found:
            raise ObligationError(f"{identifier}: no case could be rendered")
        for rendered in found:
            form, earlier, cited = by_sql.setdefault(rendered.sql, (draft.form, rendered, []))
            if (earlier.expect, earlier.stage) != (rendered.expect, rendered.stage):
                conflicts.append(
                    f"{identifier} expects {rendered.expect} {rendered.stage} where "
                    f"{cited[0]} expects {earlier.expect} {earlier.stage}: {rendered.sql}"
                )
            cited.append(identifier)
            if draft.kind == KIND_EXTREME:
                extremes.add(rendered.sql)
    if conflicts:
        raise ObligationError("one SQL text with two expectations:\n" + "\n".join(conflicts))
    session_sha: str = digest(canonical(SESSION))
    setup: list[str] = []
    cases: list[Row] = []
    for sql, (form, rendered, obligations) in by_sql.items():
        sha: str = digest(sql)
        case: Row = {
            "case_id": f"{form}.k{sha[:DIGEST_WIDTH]}",
            "obligations": sorted(set(obligations)),
            "sql": sql,
            "sql_sha256": sha,
            "session": SESSION,
            "session_sha256": session_sha,
            "expect": rendered.expect,
            "capture": capture_for(sql),
            "setup": setup,
            "setup_sha256": digest(canonical(setup)),
        }
        if rendered.stage:
            case[STAGE_KEY] = rendered.stage
        key: tuple[str, str] = (str(case["case_id"]), sha)
        cases.append(
            observed_stage(
                case=case,
                phase=receipts.phases.get(key),
                extreme=sql in extremes,
                documented=rendered.documented,
                settlement=receipts.settlements.get((*key, str(case["setup_sha256"]), session_sha)),
            )
        )
    return sorted(cases, key=lambda case: str(case["case_id"]))
