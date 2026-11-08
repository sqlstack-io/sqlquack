"""Cases of the Databricks root: one per distinct rendered SQL text of every catalogued form."""

from __future__ import annotations

import dataclasses
import tempfile
import tomllib
from pathlib import Path

from scripts.syntax_catalogue._helpers.toml_out import write_tables
from scripts.syntax_catalogue_databricks.constants import DECISION_RUNTIME_ONLY, SOURCES_FILE
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
from scripts.syntax_obligations.constants import EXPECT_ACCEPT, EXPECT_OBSERVE, KIND_EXTREME
from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.models import Draft, Node, ReceiptView, Rendered
from scripts.syntax_obligations.types import Row
from scripts.syntax_obligations_databricks.constants import (
    CAPTURE,
    CASES_DIR,
    CATALOGUE_DIR,
    DIGEST_WIDTH,
    FAMILIES,
    OBLIGATIONS_DIR,
    OBLIGATIONS_HEADER,
    RUNTIME_ONLY_CLAUSES,
    RUNTIME_SCOPE_FAMILIES,
    SESSION,
    SETUP,
    STAGE_KEY,
)


def runtime_only_forms(*, root: Path, family: str) -> set[str]:
    """The family's forms documented for Databricks Runtime only: every form of a page whose
    applies-to banner leaves out Databricks SQL (its sources.toml decision), and each form that is
    one Runtime-only clause (RUNTIME_ONLY_CLAUSES)."""
    if family not in RUNTIME_SCOPE_FAMILIES:
        return set()
    sources: set[str] = {
        str(row["id"])
        for row in tomllib.loads((root / SOURCES_FILE).read_text()).get("source", [])
        if DECISION_RUNTIME_ONLY in row.get("decisions", [])
    }
    catalogue: list[Row] = tomllib.loads((root / CATALOGUE_DIR / f"{family}.toml").read_text()).get(
        "form", []
    )
    found: set[str] = {str(form["id"]) for form in catalogue if form["source"] in sources}
    return found | {form for form in RUNTIME_ONLY_CLAUSES if form.startswith(f"db.{family}.")}


def _on_warehouse(rendered: Rendered) -> Rendered:
    """A Runtime-only positive: observed on the warehouse, not expected (D113 keeps it observed)."""
    if rendered.expect != EXPECT_ACCEPT:
        return rendered
    return dataclasses.replace(rendered, expect=EXPECT_OBSERVE, stage="", documented=True)


def family_cases(  # noqa: PLR0913
    *,
    family: str,
    numbered: list[tuple[str, Draft]],
    trees: dict[str, list[tuple[Node, ...]]],
    forms: set[str],
    receipts: ReceiptView,
    runtime_only: frozenset[str] = frozenset(),
) -> list[Row]:
    """The cases of the family's forms, settled by their receipts (D92, D133); a Runtime-only
    form's positives are observed, since the pinned engine is a Databricks SQL warehouse."""
    by_sql: dict[str, tuple[str, Rendered, list[str]]] = {}
    extremes: set[str] = set()
    conflicts: list[str] = []
    drafted: list[tuple[str, Draft, list[Rendered]]] = []
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
        drafted.append((identifier, draft, found))
    # A text a form that applies to Databricks SQL also accepts stays expected: a Hive-format line
    # with neither STORED AS nor ROW FORMAT is CREATE TABLE [USING] (the hiveformat page's note).
    general: set[str] = {
        rendered.sql
        for _, draft, found in drafted
        if draft.form not in runtime_only
        for rendered in found
        if rendered.expect == EXPECT_ACCEPT
    }
    for identifier, draft, found in drafted:
        if draft.form in runtime_only:
            found = [r if r.sql in general else _on_warehouse(r) for r in found]  # noqa: PLW2901
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
    setup: list[str] = list(SETUP)
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


def family_rows(*, root: Path, family: str, receipts: ReceiptView) -> tuple[list[Row], list[Row]]:
    """One family's obligation rows and cases; a family with no form has neither."""
    if not tomllib.loads((root / CATALOGUE_DIR / f"{family}.toml").read_text()).get("form"):
        return [], []
    numbered, trees, _, _, _ = family_obligations(root=root, family=family)
    _check_keys(numbered)
    cases: list[Row] = family_cases(
        family=family,
        numbered=numbered,
        trees=trees,
        forms=set(trees),
        receipts=receipts,
        runtime_only=frozenset(runtime_only_forms(root=root, family=family)),
    )
    return obligation_rows(numbered), cases


def write_family(*, root: Path, family: str, rows: list[Row], cases: list[Row]) -> None:
    """Write one family's obligations/<family>.toml and cases/<family>.jsonl."""
    path: Path = root / OBLIGATIONS_DIR / f"{family}.toml"
    path.parent.mkdir(parents=True, exist_ok=True)
    write_tables(
        path=path, header=OBLIGATIONS_HEADER.format(family=family), tables=[("obligation", rows)]
    )
    cases_path: Path = root / CASES_DIR / f"{family}.jsonl"
    cases_path.parent.mkdir(parents=True, exist_ok=True)
    cases_path.write_text(jsonl_text(cases))


def stale_files(*, root: Path, planned: dict[str, tuple[list[Row], list[Row]]]) -> list[Path]:
    """Committed family files that are missing, differ from the build, or the build never writes."""
    expected: list[Path] = _family_files()
    with tempfile.TemporaryDirectory() as directory:
        written: Path = Path(directory)
        for family, (rows, cases) in planned.items():
            write_family(root=written, family=family, rows=rows, cases=cases)
        differing: list[Path] = [
            root / relative
            for relative in expected
            if not (root / relative).exists()
            or (root / relative).read_bytes() != (written / relative).read_bytes()
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
