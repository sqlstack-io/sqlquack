"""Build a family's cases: one per distinct rendered SQL text, merged with mapped cases."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

from scripts.syntax_obligations._helpers.obligations import family_obligations
from scripts.syntax_obligations._helpers.render import style_of, submissions
from scripts.syntax_obligations.constants import (
    CASE_CAPTURE,
    CASE_CAPTURE_DCM,
    CASE_CAPTURE_RUN_DATABASE,
    COUNTED_RUN_STATUSES,
    D113_MARK,
    D133_ACCEPT_STAGES,
    D133_MARKS,
    D133_NEVER_ACCEPT_ERRNO,
    D133_REJECT_ERRNOS,
    DCM_DEFINITION_FORM,
    DIGEST_WIDTH,
    DOUBLE_QUOTE,
    EXPECT_ACCEPT,
    EXPECT_OBSERVE,
    EXPECT_REJECT,
    EXPECT_TERMINATES,
    HELD_RUN_DATABASE_SETUP,
    KIND_EXTREME,
    KIND_POSITIVE,
    LANE_A_BLOCK_SESSION,
    MARKS_DIR,
    MEASURE_KEY,
    PACKET_KEYS,
    RUN_DATABASE_SETUPS,
    SHARED_SCHEMA_SETUP,
    STAGE_BY_RECEIPT_PHASE,
    STAGE_KEY,
    SYNTAX_ERROR_PHASE,
)
from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.models import Draft, Node, ReceiptView, Rendered
from scripts.syntax_obligations.types import Row


def digest(data: str) -> str:
    """The SHA-256 hex digest of a text."""
    return hashlib.sha256(data.encode()).hexdigest()


def canonical(value: object) -> str:
    """Compact, key-sorted JSON."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def family_cases(
    *, root: Path, family: str, setups: dict[tuple[str, str], list[str]], held: frozenset[str]
) -> tuple[list[Row], dict[str, set[str]], list[Row], set[str], set[str], set[str]]:
    """Cases, outcomes, arity cases, extremes, documented observed SQL, and disputed forms."""
    numbered, trees, _, _, arity = family_obligations(root=root, family=family)
    by_sql: dict[str, tuple[str, Rendered, list[str]]] = {}
    expected: dict[str, set[str]] = {}
    documented: set[str] = set()
    disputed: set[str] = set()
    undisputed: set[str] = set()
    conflicts: list[str] = []
    for identifier, draft in numbered:
        found: list[Rendered] = _rendered(draft=draft, trees=trees, family=family)
        if not found:
            raise ObligationError(f"{identifier}: no case could be rendered")
        for rendered in found:
            expected.setdefault(identifier, set()).add(rendered.expect)
            if draft.kind == KIND_POSITIVE:
                (disputed if rendered.documented else undisputed).add(draft.form)
            if rendered.documented:
                documented.add(rendered.sql)
            form, earlier, cited = by_sql.setdefault(rendered.sql, (draft.form, rendered, []))
            if (earlier.expect, earlier.stage) != (rendered.expect, rendered.stage):
                conflicts.append(
                    f"{identifier} expects {rendered.expect} {rendered.stage} where "
                    f"{cited[0]} expects {earlier.expect} {earlier.stage}: {rendered.sql}"
                )
            cited.append(identifier)
    if conflicts:
        raise ObligationError("one SQL text with two expectations:\n" + "\n".join(conflicts))
    session_sha: str = digest(canonical(LANE_A_BLOCK_SESSION))
    kept: dict[str, str] = _kept_prefixes(path=root / "cases" / f"{family}.jsonl")
    cases: list[Row] = []
    for sql, (first, rendered, obligations) in by_sql.items():
        sha: str = digest(sql)
        citing: set[str] = {identifier.rsplit(".o", 1)[0] for identifier in obligations}
        form: str = kept[sha] if kept.get(sha) in citing else first
        case: Row = {
            "case_id": f"{form}.k{sha[:DIGEST_WIDTH]}",
            "obligations": sorted(set(obligations)),
            "sql": sql,
            "sql_sha256": sha,
            "session": LANE_A_BLOCK_SESSION,
            "session_sha256": session_sha,
            "expect": rendered.expect,
            **_binding(form=form, family=family, sql_sha=sha, setups=setups, held=held),
        }
        if rendered.stage:
            case[STAGE_KEY] = rendered.stage
        cases.append(case)
    scored: set[str] = {str(case["sql"]) for case in cases}
    return (
        sorted(cases, key=lambda case: case["case_id"]),
        expected,
        _arity_cases(rows=arity, scored=scored, family=family, setups=setups, held=held),
        {identifier for identifier, draft in numbered if draft.kind == KIND_EXTREME},
        documented,
        disputed - undisputed,
    )


def _kept_prefixes(*, path: Path) -> dict[str, str]:
    """The form prefix each SQL's case id had in the last build, so a receipt keeps joining."""
    if not path.exists():
        return {}
    kept: dict[str, str] = {}
    for line in path.read_text().splitlines():
        row: Row = json.loads(line)
        prefix, _, _ = str(row["case_id"]).rpartition(".k")
        if prefix:
            kept[str(row["sql_sha256"])] = prefix
    return kept


def _binding(
    *,
    form: str,
    family: str,
    sql_sha: str,
    setups: dict[tuple[str, str], list[str]],
    held: frozenset[str],
) -> Row:
    """Capture and setup: DCM file for DEFINE, run database as receipted for ddl and held."""
    capture: str = CASE_CAPTURE
    setup: list[str] = []
    case_id: str = f"{form}.k{sql_sha[:DIGEST_WIDTH]}"
    if DCM_DEFINITION_FORM.search(form):
        capture = CASE_CAPTURE_DCM
    elif family in RUN_DATABASE_SETUPS:
        capture = CASE_CAPTURE_RUN_DATABASE
        setup = setups.get((case_id, sql_sha), list(RUN_DATABASE_SETUPS[family]))
    elif case_id in held:
        capture = CASE_CAPTURE_RUN_DATABASE
        setup = setups.get((case_id, sql_sha), list(HELD_RUN_DATABASE_SETUP))
    return {"capture": capture, "setup": setup, "setup_sha256": digest(canonical(setup))}


def run_database_setups(directory: Path) -> tuple[dict[tuple[str, str], list[str]], list[str]]:
    """Each ddl case's setup from its latest counted receipt, and cases for review."""
    texts_by_setup_sha: dict[str, list[str]] = {}
    counted_runs: set[str] = set()
    for path in directory.glob("*.report.json"):
        report: Row = json.loads(path.read_text())
        for row in report.get("inputs", []):
            setup: list[str] = list(row["setup"])
            texts_by_setup_sha[digest(canonical(setup))] = setup
        counted_runs |= {
            str(run["run_id"])
            for run in report.get("runs", [])
            if isinstance(run, dict) and run.get("status") in COUNTED_RUN_STATUSES
        }
    latest: dict[tuple[str, str], tuple[str, list[str]]] = {}
    classes: dict[tuple[str, str], set[bool]] = {}
    for path in sorted(directory.glob("*.jsonl")):
        for line in path.read_text().splitlines():
            receipt: Row = json.loads(line)
            key: tuple[str, str] = (str(receipt["case_id"]), str(receipt["sql_sha256"]))
            setup = texts_by_setup_sha.get(str(receipt.get("setup_sha256", "")), [])
            if (
                receipt.get("capture_method") != CASE_CAPTURE_RUN_DATABASE
                or MEASURE_KEY in receipt
                or str(receipt.get("run_id")) not in counted_runs
                or not setup
                or any(statement.startswith(SHARED_SCHEMA_SETUP) for statement in setup)
            ):
                continue
            classes.setdefault(key, set()).add(receipt["phase"] == SYNTAX_ERROR_PHASE)
            recorded: str = str(receipt["recorded_at"])
            if key not in latest or recorded > latest[key][0]:
                latest[key] = (recorded, setup)
    flagged: list[str] = sorted(key[0] for key, seen in classes.items() if len(seen) > 1)
    setups: dict[tuple[str, str], list[str]] = {
        key: setup for key, (_, setup) in latest.items() if len(classes[key]) == 1
    }
    return setups, flagged


def _omission(row: Row) -> str:
    """`kind | target | note` of one withdrawn arity draft."""
    return f"{row['kind']} | {row['target']} | {row['note']}"


def _arity_cases(
    *,
    rows: list[Row],
    scored: set[str],
    family: str,
    setups: dict[tuple[str, str], list[str]],
    held: frozenset[str],
) -> list[Row]:
    """Unscored observe cases, one per distinct SQL text, naming the omissions they render."""
    by_sql: dict[str, list[Row]] = {}
    for row in rows:
        if row["sql"] not in scored:
            by_sql.setdefault(str(row["sql"]), []).append(row)
    session_sha: str = digest(canonical(LANE_A_BLOCK_SESSION))
    cases: list[Row] = []
    for sql, found in by_sql.items():
        sha: str = digest(sql)
        cases.append(
            {
                "case_id": f"{found[0]['form']}.k{sha[:DIGEST_WIDTH]}",
                "obligations": [],
                "omits": sorted(_omission(row) for row in found),
                "sql": sql,
                "sql_sha256": sha,
                "session": LANE_A_BLOCK_SESSION,
                "session_sha256": session_sha,
                "expect": EXPECT_OBSERVE,
                **_binding(
                    form=str(found[0]["form"]), family=family, sql_sha=sha, setups=setups, held=held
                ),
            }
        )
    return sorted(cases, key=lambda case: case["case_id"])


def _rendered(
    *, draft: Draft, trees: dict[str, list[tuple[Node, ...]]], family: str
) -> list[Rendered]:
    if draft.spec is None:
        return []
    return submissions(
        trees=trees[draft.form],
        style=style_of(family=family, form=draft.form),
        spec=draft.spec,
        depths=draft.depths,
    )


def _cited_pairs(case: Row) -> set[tuple[str, str]]:
    return {(str(o), str(case["expect"])) for o in case["obligations"]}


def _reserved_word_expect(sql: str) -> str:
    """Reserved-word identifier cases: accept the quoted case, reject the unquoted one."""
    return EXPECT_ACCEPT if DOUBLE_QUOTE in sql else EXPECT_REJECT


def merge_mapped(
    *, cases: list[Row], mapped: Path, expected: dict[str, set[str]]
) -> tuple[list[Row], list[str]]:
    """Mapped cases (contradictions reported), then generated cases they do not already cover."""
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
            wants = {_reserved_word_expect(str(case["sql"]))}
        if len(wants) != 1:
            problems.append(f"{case['case_id']}: cites obligations expecting {sorted(wants)}")
            continue
        case["expect"] = wants.pop()
        kept.append(case)
    covered: set[tuple[str, str]] = set()
    for case in kept:
        covered |= _cited_pairs(case)
    generated: list[Row] = [c for c in cases if not _cited_pairs(c) <= covered]
    return kept + generated, problems


def held_cases(paths: list[Path]) -> frozenset[str]:
    """The case ids of held-case lists (JSON lines with case_id), run later in a run database."""
    held: set[str] = set()
    for path in paths:
        held |= {str(json.loads(line)["case_id"]) for line in path.read_text().splitlines()}
    return frozenset(held)


def receipt_phases(directory: Path) -> dict[tuple[str, str], str]:
    """Phase of each (case_id, sql_sha256) receipt not withdrawn, superseded or of a measure."""
    gone: set[str] = set()
    for path in directory.glob("*.report.json"):
        gone |= {
            str(x["receipt_sha256"]) for x in json.loads(path.read_text()).get("withdrawn", [])
        }
    receipts: list[tuple[str, Row]] = []
    for path in sorted(directory.glob("*.jsonl")):
        for line in path.read_text().splitlines():
            receipt: Row = json.loads(line)
            gone.add(str(receipt.get("supersedes_receipt_sha256", "")))
            if MEASURE_KEY not in receipt:
                receipts.append((digest(line), receipt))
    return {
        (str(r["case_id"]), str(r["sql_sha256"])): str(r["phase"])
        for sha, r in receipts
        if sha not in gone
    }


def receipt_settlements(directory: Path) -> dict[tuple[str, str, str, str], tuple[str, str]]:
    """The expect and stage each (case, sql, setup, session) settles to, where it settles."""
    gone: set[str] = set()
    for path in directory.glob("*.report.json"):
        gone |= {
            str(x["receipt_sha256"]) for x in json.loads(path.read_text()).get("withdrawn", [])
        }
    answers: dict[tuple[str, str, str, str], set[tuple[str, str]]] = {}
    lines: list[tuple[str, Row]] = []
    for path in sorted(directory.glob("*.jsonl")):
        for line in path.read_text().splitlines():
            receipt: Row = json.loads(line)
            gone.add(str(receipt.get("supersedes_receipt_sha256", "")))
            if MEASURE_KEY not in receipt:
                lines.append((digest(line), receipt))
    for sha, r in lines:
        if sha in gone:
            continue
        key: tuple[str, str, str, str] = (
            str(r["case_id"]),
            str(r["sql_sha256"]),
            str(r.get("setup_sha256", "")),
            str(r.get("session_sha256", "")),
        )
        errno: object = r.get("errno")
        answers.setdefault(key, set()).add((str(r["phase"]), "" if errno is None else str(errno)))
    settled: dict[tuple[str, str, str, str], tuple[str, str]] = {}
    for key, found in answers.items():
        if len(found) != 1:
            continue
        phase, errno = next(iter(found))
        if errno in D133_REJECT_ERRNOS or phase == SYNTAX_ERROR_PHASE:
            settled[key] = (EXPECT_REJECT, "")
        elif errno != D133_NEVER_ACCEPT_ERRNO and phase in D133_ACCEPT_STAGES:
            settled[key] = (EXPECT_ACCEPT, D133_ACCEPT_STAGES[phase])
    return settled


def recorded_receipts(directory: Path) -> set[tuple[str, str]]:
    """Every receipted (case_id, sql_sha256), withdrawn too: recording it again would repeat it."""
    recorded: set[tuple[str, str]] = set()
    for path in directory.glob("*.jsonl"):
        for line in path.read_text().splitlines():
            receipt: Row = json.loads(line)
            recorded.add((str(receipt["case_id"]), str(receipt["sql_sha256"])))
    return recorded


def observed_stage(
    *,
    case: Row,
    phase: str | None,
    extreme: bool = True,
    documented: bool = False,
    settlement: tuple[str, str] | None = None,
) -> Row:
    """Give an accepted case its receipt's later stage, reporting a contradicted declared one."""
    stage: str | None = STAGE_BY_RECEIPT_PHASE.get(phase or "")
    if case["expect"] == EXPECT_OBSERVE and not extreme and not documented:
        return _settled(case=case, settlement=settlement)
    if case["expect"] == EXPECT_OBSERVE:
        return _observed_outcome(
            case=case, phase=phase, stage=stage, extreme=extreme, documented=documented
        )
    if case["expect"] != EXPECT_ACCEPT or stage is None or stage == case.get(STAGE_KEY, ""):
        return case
    if STAGE_KEY in case:
        print(f"stage of {case['case_id']}: declared {case[STAGE_KEY]}, receipt {phase}")
    if stage:
        case[STAGE_KEY] = stage
    else:
        del case[STAGE_KEY]
    return case


def _settled(*, case: Row, settlement: tuple[str, str] | None) -> Row:
    """An observed case takes the expect its agreeing receipts settle, or stays observed."""
    if settlement is not None:
        case["expect"], stage = settlement
        if stage:
            case[STAGE_KEY] = stage
    return case


def _observed_outcome(
    *, case: Row, phase: str | None, stage: str | None, extreme: bool, documented: bool
) -> Row:
    """An extreme's outcome; an observed negative settles only on a syntax error."""
    if not extreme:
        return case
    if phase == SYNTAX_ERROR_PHASE:
        print(f"observed implementation limit, undocumented: {case['case_id']}")
        case["expect"] = EXPECT_TERMINATES
    elif stage is not None:
        case["expect"] = EXPECT_ACCEPT
        if stage:
            case[STAGE_KEY] = stage
    return case


def settled_cases(
    *,
    root: Path,
    family: str,
    receipts: ReceiptView,
    setups: dict[tuple[str, str], list[str]],
    held: frozenset[str],
) -> tuple[list[Row], list[Row], list[Row]]:
    """The merged cases with their receipts' stages, unscored arity cases, case marks."""
    generated, expected, arity, extremes, documented, disputed = family_cases(
        root=root, family=family, setups=setups, held=held
    )
    cases, problems = merge_mapped(
        cases=generated, mapped=root / "mapped-cases" / f"{family}.jsonl", expected=expected
    )
    for problem in problems:
        print(f"mapped case left out: {problem}")
    settled: list[Row] = []
    marked: list[Row] = []
    for case in cases:
        extreme: bool = bool(extremes & set(case["obligations"]))
        is_documented: bool = case["sql"] in documented or _cites_disputed_form(
            obligations=case["obligations"], disputed=disputed
        )
        by_receipts: bool = case["expect"] == EXPECT_OBSERVE and not extreme and not is_documented
        settled.append(
            observed_stage(
                case=case,
                phase=receipts.phases.get((case["case_id"], case["sql_sha256"])),
                extreme=extreme,
                documented=is_documented,
                settlement=receipts.settlements.get(
                    (
                        case["case_id"],
                        case["sql_sha256"],
                        case["setup_sha256"],
                        case["session_sha256"],
                    )
                ),
            )
        )
        mark: str = (
            D113_MARK
            if is_documented and case["expect"] == EXPECT_OBSERVE
            else (D133_MARKS.get(case["expect"], "") if by_receipts else "")
        )
        if mark:
            marked.append(
                {
                    "case_id": case["case_id"],
                    "sql_sha256": case["sql_sha256"],
                    "obligations": case["obligations"],
                    "mark": mark,
                }
            )
    return settled, arity, marked


def _cites_disputed_form(*, obligations: list[str], disputed: set[str]) -> bool:
    """Whether a case cites a form all of whose documented positives Snowflake refuses."""
    forms: set[str] = {obligation.rsplit(".o", 1)[0] for obligation in obligations}
    return bool(forms & disputed)


def jsonl_text(rows: list[Row]) -> str:
    """Canonical JSON lines."""
    return "".join(canonical(row) + "\n" for row in rows)


def _packet_row(case: Row) -> Row:
    return {key: case[key] for key in PACKET_KEYS}


def packet_text(cases: list[Row]) -> str:
    """The capture packet lines of the cases: only the fields a capture needs."""
    return jsonl_text([_packet_row(case) for case in cases])


def family_outputs(
    *,
    root: Path,
    family: str,
    receipts: ReceiptView,
    setups: dict[tuple[str, str], list[str]],
    held: frozenset[str],
) -> tuple[list[Row], list[Row], list[tuple[Path, str | None]]]:
    """One family's cases and arity cases, and each of its files' text (None: no file)."""
    cases, arity, marked = settled_cases(
        root=root,
        family=family,
        receipts=receipts,
        setups=setups,
        held=held,
    )
    outputs: list[tuple[Path, str | None]] = [
        (root / MARKS_DIR / f"{family}.jsonl", jsonl_text(marked) if marked else None),
        (root / "cases" / f"{family}.jsonl", jsonl_text(cases)),
        (root / "arity" / f"{family}.jsonl", jsonl_text(arity) if arity else None),
    ]
    return cases, arity, outputs


def sync_outputs(*, outputs: list[tuple[Path, str | None]], check: bool) -> list[Path]:
    """The files whose text differs; each is rewritten or removed unless check."""
    differing: list[Path] = []
    for path, text in outputs:
        if text == (path.read_text() if path.exists() else None):
            continue
        differing.append(path)
        if check:
            continue
        if text is None:
            path.unlink()
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
    return differing
