"""Cases of the BigQuery root: one per distinct rendered SQL text of every catalogued form."""

from __future__ import annotations

from dataclasses import replace
from pathlib import Path

from scripts.grammar_catalogue._helpers.toml_out import write_tables
from scripts.grammar_obligations._helpers.cases import (
    canonical,
    digest,
    jsonl_text,
    observed_stage,
    receipt_phases,
    receipt_settlements,
)
from scripts.grammar_obligations._helpers.obligations import family_obligations, obligation_rows
from scripts.grammar_obligations._helpers.render import style_of, submissions
from scripts.grammar_obligations.constants import KIND_EXTREME
from scripts.grammar_obligations.exceptions import ObligationError
from scripts.grammar_obligations.models import Draft, Node, ReceiptView, Rendered
from scripts.grammar_obligations.types import Row
from scripts.syntax_obligations_bigquery._helpers.adapt import is_suppressed, suppress_rows
from scripts.syntax_obligations_bigquery._helpers.fixtures import case_setup
from scripts.syntax_obligations_bigquery.classes.withdrawals import Withdrawals
from scripts.syntax_obligations_bigquery.constants import (
    CAPTURE,
    CASES_DIR,
    DIGEST_WIDTH,
    DISPOSITION_OBSERVE,
    EXPECT_OBSERVE,
    EXPECT_REJECT,
    LIMITS_FILE,
    OBLIGATIONS_DIR,
    OBLIGATIONS_HEADER,
    QUALIFIED_TABLE,
    QUOTE_ACTION,
    QUOTE_SPAN,
    QUOTED_IDENTIFIER,
    RESERVED_ACTION,
    RESERVED_BACKQUOTED,
    RESERVED_QUOTED,
    SESSION,
    STAGE_KEY,
)
from scripts.syntax_obligations_bigquery.models import FamilyPlan


def family_cases(
    *,
    family: str,
    numbered: list[tuple[str, Draft]],
    trees: dict[str, list[tuple[Node, ...]]],
    receipts: ReceiptView,
    observed: list[Row],
) -> tuple[list[Row], set[str]]:
    """The family's cases, settled by receipts, and the texts the gate settles."""
    by_sql: dict[str, tuple[str, Rendered, list[str]]] = {}
    settle: set[str] = set()
    extremes: set[str] = set()
    conflicts: list[str] = []
    for identifier, draft in numbered:
        if draft.form not in trees or draft.spec is None:
            continue
        found: list[Rendered] = submissions(
            trees=trees[draft.form],
            style=style_of(family=family, form=draft.form),
            spec=draft.spec,
            depths=draft.depths,
        )
        if not found:
            raise ObligationError(f"{identifier}: no case could be rendered")
        if draft.spec.action in (QUOTE_ACTION, RESERVED_ACTION):
            found = [backquoted(rendered) for rendered in found]
        found = [unqualified(rendered) for rendered in found]
        if is_suppressed(draft=draft, rows=observed):
            found = [observed_reject(rendered) for rendered in found]
            settle |= {r.sql for r in found if r.expect == EXPECT_OBSERVE}
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
    cases: list[Row] = []
    for sql, (form, rendered, obligations) in by_sql.items():
        sha: str = digest(sql)
        setup: list[str] = case_setup(sql)
        case: Row = {
            "case_id": f"{form}.k{sha[:DIGEST_WIDTH]}",
            "obligations": sorted(set(obligations)),
            "sql": sql,
            "sql_sha256": sha,
            "session": SESSION,
            "session_sha256": session_sha,
            "expect": rendered.expect,
            "capture": CAPTURE,
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
    return sorted(cases, key=lambda case: str(case["case_id"])), settle


def observed_reject(rendered: Rendered) -> Rendered:
    """A reject text of an `observe` suppression, observed until the parser gate settles it."""
    if rendered.expect != EXPECT_REJECT:
        return rendered
    return replace(rendered, expect=EXPECT_OBSERVE)


def withdrawn_texts(
    *, family: str, drafts: list[Draft], trees: dict[str, list[tuple[Node, ...]]]
) -> list[tuple[str, str]]:
    """(form, SQL) of each reject text a `documented` suppression withdrew from the family."""
    texts: list[tuple[str, str]] = []
    for draft in drafts:
        if draft.form not in trees or draft.spec is None:
            continue
        for rendered in submissions(
            trees=trees[draft.form],
            style=style_of(family=family, form=draft.form),
            spec=draft.spec,
            depths=draft.depths,
        ):
            if rendered.expect == EXPECT_REJECT:
                texts.append((draft.form, unqualified(rendered).sql))
    return texts


def backquoted(rendered: Rendered) -> Rendered:
    """A quoted-identifier case with BigQuery's backquotes: double quotes delimit strings."""
    sql: str = QUOTE_SPAN.sub(QUOTED_IDENTIFIER, rendered.sql)
    return replace(rendered, sql=RESERVED_QUOTED.sub(RESERVED_BACKQUOTED, sql))


def unqualified(rendered: Rendered) -> Rendered:
    """A case whose grammar writes the qualifier: a table fill after `x .` keeps its own name."""
    return replace(rendered, sql=QUALIFIED_TABLE.sub("", rendered.sql))


def receipt_view(directory: Path) -> ReceiptView:
    """The phases and settlements of the receipts in a directory."""
    return ReceiptView(phases=receipt_phases(directory), settlements=receipt_settlements(directory))


def _check_keys(numbered: list[tuple[str, Draft]]) -> None:
    """Refuse two obligations with one key: a rebuild would give both the same kept id."""
    owner: dict[tuple[str, ...], str] = {}
    for identifier, draft in numbered:
        key: tuple[str, ...] = (draft.form, draft.kind, draft.target, draft.note, *draft.pair)
        if key in owner:
            raise ObligationError(f"{owner[key]} and {identifier} share one key; read the form")
        owner[key] = identifier


def family_rows(
    *, root: Path, family: str, receipts: ReceiptView, withdrawn: Withdrawals
) -> FamilyPlan:
    """One family's obligation rows and cases, as the build would write them."""
    start: int = len(withdrawn.drafts)
    numbered, trees, _, _, _ = family_obligations(root=root, family=family)
    _check_keys(numbered)
    observed: list[Row] = suppress_rows(path=root / LIMITS_FILE, disposition=DISPOSITION_OBSERVE)
    cases, settle = family_cases(
        family=family,
        numbered=numbered,
        trees=trees,
        receipts=receipts,
        observed=observed,
    )
    return FamilyPlan(
        rows=obligation_rows(numbered),
        cases=cases,
        settle=frozenset(settle),
        withdrawn=tuple(
            withdrawn_texts(family=family, drafts=withdrawn.drafts[start:], trees=trees)
        ),
    )


def write_family(*, root: Path, family: str, rows: list[Row], cases: list[Row]) -> None:
    """Write one family's obligations and cases."""
    path: Path = root / OBLIGATIONS_DIR / f"{family}.toml"
    path.parent.mkdir(parents=True, exist_ok=True)
    write_tables(
        path=path, header=OBLIGATIONS_HEADER.format(family=family), tables=[("obligation", rows)]
    )
    cases_path: Path = root / CASES_DIR / f"{family}.jsonl"
    cases_path.parent.mkdir(parents=True, exist_ok=True)
    cases_path.write_text(jsonl_text(cases))
