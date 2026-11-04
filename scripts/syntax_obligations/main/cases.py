"""Write coverage/syntax/cases/<family>.jsonl: one case per distinct rendered SQL text."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from scripts.syntax_obligations._helpers.render import FormStyle, Rendered, Renderer
from scripts.syntax_obligations.constants import (
    CASE_CAPTURE,
    CASE_SESSION,
    EXPECT_ACCEPT,
    EXPECT_REJECT,
)
from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.main.build import family_obligations
from scripts.syntax_obligations.models import Draft, Node
from scripts.syntax_obligations.styles import FAMILY_STYLES
from scripts.syntax_obligations.types import Row

DIGEST_WIDTH: int = 12


def _digest(data: str) -> str:
    return hashlib.sha256(data.encode()).hexdigest()


def _canonical(value: object) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def style_of(*, family: str, form: str) -> FormStyle:
    """The longest matching form-id prefix style of the family."""
    styles: dict[str, FormStyle] = FAMILY_STYLES[family]
    best: str = max((prefix for prefix in styles if form.startswith(prefix)), key=len)
    return styles[best]


def family_cases(*, root: Path, family: str) -> tuple[list[Row], dict[str, set[str]]]:
    """Cases with SQL, hashes, expected outcome and the obligation ids each one satisfies, and
    every obligation's expected outcomes. One SQL text never carries two expectations."""
    numbered, trees = family_obligations(root=root, family=family)
    by_sql: dict[str, tuple[str, Rendered, list[str]]] = {}
    expected: dict[str, set[str]] = {}
    conflicts: list[str] = []
    for identifier, draft in numbered:
        found: list[Rendered] = _rendered(draft=draft, trees=trees, family=family)
        if not found:
            raise ObligationError(f"{identifier}: no case could be rendered")
        for rendered in found:
            expected.setdefault(identifier, set()).add(rendered.expect)
            form, earlier, cited = by_sql.setdefault(rendered.sql, (draft.form, rendered, []))
            if (earlier.expect, earlier.stage) != (rendered.expect, rendered.stage):
                conflicts.append(
                    f"{identifier} expects {rendered.expect} {rendered.stage} where "
                    f"{cited[0]} expects {earlier.expect} {earlier.stage}: {rendered.sql}"
                )
            cited.append(identifier)
    if conflicts:
        raise ObligationError("one SQL text with two expectations:\n" + "\n".join(conflicts))
    session_sha: str = _digest(_canonical(CASE_SESSION))
    setup_sha: str = _digest(_canonical([]))
    cases: list[Row] = []
    for sql, (form, rendered, obligations) in by_sql.items():
        sha: str = _digest(sql)
        case: Row = {
            "case_id": f"{form}.k{sha[:DIGEST_WIDTH]}",
            "obligations": sorted(set(obligations)),
            "sql": sql,
            "sql_sha256": sha,
            "setup": [],
            "setup_sha256": setup_sha,
            "session": CASE_SESSION,
            "session_sha256": session_sha,
            "capture": CASE_CAPTURE,
            "expect": rendered.expect,
        }
        if rendered.stage:
            case["stage"] = rendered.stage
        cases.append(case)
    return sorted(cases, key=lambda case: case["case_id"]), expected


def _rendered(
    *, draft: Draft, trees: dict[str, list[tuple[Node, ...]]], family: str
) -> list[Rendered]:
    if draft.spec is None:
        return []
    style: FormStyle = style_of(family=family, form=draft.form)
    renderer: Renderer = Renderer(trees=trees[draft.form], style=style)
    return renderer.submissions(draft.spec, draft.depths)


def merge_mapped(
    *, cases: list[Row], mapped: Path, expected: dict[str, set[str]]
) -> tuple[list[Row], list[str]]:
    """Existing mapped cases first, with the expected outcome their obligations share and any
    citation that contradicts it reported; a generated case is dropped when every one of its
    obligations is already covered by a mapped case with the same expectation."""
    if not mapped.exists():
        return cases, []
    kept: list[Row] = []
    problems: list[str] = []
    for line in mapped.read_text().splitlines():
        case: Row = json.loads(line)
        withdrawn: list[str] = [str(o) for o in case["obligations"] if str(o) not in expected]
        if withdrawn:
            problems.append(f"{case['case_id']}: citations of withdrawn {withdrawn} dropped")
            case["obligations"] = [o for o in case["obligations"] if str(o) in expected]
        if not case["obligations"]:
            continue
        wants: set[str] = set()
        for obligation in case["obligations"]:
            single: set[str] = expected.get(str(obligation), set())
            wants |= single if len(single) == 1 else set()
        if not wants:
            # Only reserved-word identifier obligations, which have a quoted case expected to
            # be accepted and an unquoted one expected to be rejected.
            wants = {EXPECT_ACCEPT if '"' in str(case["sql"]) else EXPECT_REJECT}
        if len(wants) != 1:
            problems.append(f"{case['case_id']}: cites obligations expecting {sorted(wants)}")
            continue
        case["expect"] = wants.pop()
        kept.append(case)
    covered: set[str] = {str(o) for case in kept for o in case["obligations"]}
    generated: list[Row] = [c for c in cases if not set(c["obligations"]) <= covered]
    return kept + generated, problems


def run_cases(argv: list[str] | None = None) -> int:
    """Regenerate the cases of the named families and list those that need a capture."""
    parser: argparse.ArgumentParser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("coverage/syntax"))
    parser.add_argument("--receipts", type=Path, help="receipt directory to match SQL hashes")
    parser.add_argument("--packet", type=Path, help="write the cases still to capture here")
    parser.add_argument("families", nargs="+")
    arguments: argparse.Namespace = parser.parse_args(argv)
    root: Path = arguments.root
    recorded: set[tuple[str, str]] = set()
    if arguments.receipts:
        for path in sorted(arguments.receipts.glob("*.jsonl")):
            for line in path.read_text().splitlines():
                receipt: Row = json.loads(line)
                recorded.add((str(receipt["case_id"]), str(receipt["sql_sha256"])))
    to_capture: list[Row] = []
    for family in arguments.families:
        generated, expected = family_cases(root=root, family=family)
        cases, problems = merge_mapped(
            cases=generated, mapped=root / "mapped-cases" / f"{family}.jsonl", expected=expected
        )
        for problem in problems:
            print(f"mapped case left out: {problem}")
        path: Path = root / "cases" / f"{family}.jsonl"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("".join(_canonical(case) + "\n" for case in cases))
        to_capture += [c for c in cases if (c["case_id"], c["sql_sha256"]) not in recorded]
        print(json.dumps({"family": family, "cases": len(cases), "to_capture": len(to_capture)}))
    if arguments.packet:
        arguments.packet.write_text(
            "".join(
                _canonical(
                    {
                        key: case[key]
                        for key in ("case_id", "sql", "sql_sha256", "capture", "expect")
                    }
                )
                + "\n"
                for case in to_capture
            )
        )
    return 0
