"""Cases of the DuckDB root: one per distinct rendered SQL text of every catalogued form."""

from __future__ import annotations

from pathlib import Path

from scripts.syntax_catalogue._helpers.toml_out import write_tables
from scripts.syntax_obligations._helpers.cases import (
    canonical,
    digest,
    jsonl_text,
    observed_stage,
    receipt_phases,
    receipt_settlements,
)
from scripts.syntax_obligations._helpers.obligations import family_obligations, obligation_rows
from scripts.syntax_obligations._helpers.render import style_of, submissions
from scripts.syntax_obligations.constants import EXPECT_OBSERVE, KIND_EXTREME
from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.models import Draft, Node, ReceiptView, Rendered
from scripts.syntax_obligations.types import Row
from scripts.syntax_obligations_duckdb.constants import (
    CAPTURE_FRESH,
    CASES_DIR,
    DIGEST_WIDTH,
    FAMILIES,
    OBLIGATIONS_DIR,
    OBLIGATIONS_HEADER,
    SESSION,
    STAGE_KEY,
    UNOBSERVABLE_FORMS,
    UNOBSERVABLE_TEXTS,
)


def family_cases(
    *,
    family: str,
    numbered: list[tuple[str, Draft]],
    trees: dict[str, list[tuple[Node, ...]]],
    forms: set[str],
    receipts: ReceiptView,
) -> list[Row]:
    """The family's cases, settled by their receipts unless unobservable."""
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
            "capture": CAPTURE_FRESH,
            "setup": setup,
            "setup_sha256": digest(canonical(setup)),
        }
        if rendered.stage:
            case[STAGE_KEY] = rendered.stage
        if form in UNOBSERVABLE_FORMS or any(p.search(sql) for p in UNOBSERVABLE_TEXTS):
            case["expect"] = EXPECT_OBSERVE
            case.pop(STAGE_KEY, None)
            cases.append(case)
            continue
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


def write_family(*, root: Path, family: str, receipts: ReceiptView) -> tuple[int, int]:
    """Write one family's obligations and cases; return their counts."""
    numbered, trees, _, _, _ = family_obligations(root=root, family=family)
    _check_keys(numbered)
    rows: list[Row] = obligation_rows(numbered)
    path: Path = root / OBLIGATIONS_DIR / f"{family}.toml"
    path.parent.mkdir(parents=True, exist_ok=True)
    write_tables(
        path=path, header=OBLIGATIONS_HEADER.format(family=family), tables=[("obligation", rows)]
    )
    cases: list[Row] = family_cases(
        family=family, numbered=numbered, trees=trees, forms=set(trees), receipts=receipts
    )
    cases_path: Path = root / CASES_DIR / f"{family}.jsonl"
    cases_path.parent.mkdir(parents=True, exist_ok=True)
    cases_path.write_text(jsonl_text(cases))
    return len(rows), len(cases)


def stale_files(*, root: Path, built: Path) -> list[Path]:
    """Committed family files that are missing, differ from the build in `built`, or are stray."""
    expected: list[Path] = _family_files()
    differing: list[Path] = [
        root / relative
        for relative in expected
        if not (root / relative).exists()
        or (root / relative).read_bytes() != (built / relative).read_bytes()
    ]
    return differing + _unexpected_files(root=root, expected=expected)


def _family_files() -> list[Path]:
    """The obligations and cases file of every family, relative to the root."""
    files: list[Path] = []
    for family in FAMILIES:
        files.append(Path(OBLIGATIONS_DIR) / f"{family}.toml")
        files.append(Path(CASES_DIR) / f"{family}.jsonl")
    return files


def _unexpected_files(*, root: Path, expected: list[Path]) -> list[Path]:
    """Files under the obligations and cases directories that no family's build writes."""
    committed: list[Path] = sorted(
        [*(root / OBLIGATIONS_DIR).rglob("*"), *(root / CASES_DIR).rglob("*")]
    )
    return [path for path in committed if path.is_file() and path.relative_to(root) not in expected]
