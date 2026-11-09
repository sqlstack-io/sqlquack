"""Count restated syntax once: fold repeated grammar into one canonical form with extra sources."""

from __future__ import annotations

import tomllib
from pathlib import Path

from scripts.grammar_catalogue._helpers.toml_out import row_lines
from scripts.grammar_catalogue.constants import (
    DECISION_RESTATED,
    KIND_PROSE,
)
from scripts.grammar_catalogue.exceptions import CatalogueError
from scripts.grammar_catalogue.models import Catalogue
from scripts.grammar_catalogue.types import Row
from scripts.syntax_catalogue_snowflake.constants import (
    FOLD_IDENTICAL_REASON,
    PAGE_FORMS,
    PAGE_RESTATED,
    SOURCE_FIELD,
)


def load_folds(path: Path) -> dict[str, tuple[tuple[str, ...], str]]:
    """Hand folds from hand.toml: a restating form, the canonical forms it folds into, why."""
    if not path.exists():
        return {}
    return {
        str(row["form"]): (tuple(map(str, row["into"])), str(row["reason"]))
        for row in tomllib.loads(path.read_text()).get("fold", [])
    }


def identical_folds(forms: dict[str, Row]) -> dict[str, tuple[tuple[str, ...], str]]:
    """Byte-identical grammar on other pages, folded into the shortest-id page's form."""
    groups: dict[str, list[Row]] = {}
    for form in forms.values():
        if form["kind"] != KIND_PROSE:
            groups.setdefault(str(form["grammar_sha256"]), []).append(form)
    folds: dict[str, tuple[tuple[str, ...], str]] = {}
    for group in groups.values():
        if len({form["source"] for form in group}) < len(group) or len(group) == 1:
            continue
        group.sort(
            key=lambda form: (len(str(form["source"])), str(form["source"]), str(form["id"]))
        )
        for form in group[1:]:
            folds[str(form["id"])] = ((str(group[0]["id"]),), FOLD_IDENTICAL_REASON)
    return folds


def fold_restatements(
    *, catalogue: Catalogue, hand_folds: dict[str, tuple[tuple[str, ...], str]]
) -> Catalogue:
    """Drop folded forms into their canonical forms' also_sources; mark wholly folded pages."""
    forms: dict[str, Row] = {}
    for rows in catalogue.forms.values():
        forms |= {str(form["id"]): form for form in rows}
    folds: dict[str, tuple[tuple[str, ...], str]] = identical_folds(forms)
    for form_id, fold in hand_folds.items():
        if form_id not in forms or not all(target in forms for target in fold[0]):
            raise CatalogueError(f"fold of {form_id} into {fold[0]}: unknown form")
        folds[form_id] = fold
    extra: dict[str, set[str]] = {}
    for form_id, (targets, _) in folds.items():
        for target in targets:
            if target in folds:
                raise CatalogueError(f"{form_id} folds into {target}, which is itself folded")
            if forms[target]["source"] != forms[form_id]["source"]:
                extra.setdefault(target, set()).add(str(forms[form_id]["source"]))
    folded_pages: set[str] = {str(forms[form_id]["source"]) for form_id in folds}
    for family, rows in catalogue.forms.items():
        catalogue.forms[family] = [
            _with_sources(row=row, extra=extra.get(str(row["id"])))
            for row in rows
            if row["id"] not in folds
        ]
    kept_pages: set[str] = set()
    for rows in catalogue.forms.values():
        kept_pages |= {str(row["source"]) for row in rows}
    for source in catalogue.sources:
        if (
            source["id"] in folded_pages
            and source["id"] not in kept_pages
            and source["status"] == PAGE_FORMS
        ):
            source["status"] = PAGE_RESTATED
            earlier: object = source["decisions"]
            decisions: list[str] = [str(d) for d in earlier] if isinstance(earlier, list) else []
            source["decisions"] = sorted({*decisions, DECISION_RESTATED})
    catalogue.folds.update(folds)
    return catalogue


def _with_sources(*, row: Row, extra: set[str] | None) -> Row:
    if not extra:
        return row
    ordered: Row = {}
    for key, value in row.items():
        ordered[key] = value
        if key == SOURCE_FIELD:
            ordered["also_sources"] = sorted(extra - {value})
    return ordered


def add_fold_aliases(*, path: Path, folds: dict[str, tuple[tuple[str, ...], str]]) -> None:
    """Append an alias for every folded catalogue form that aliases.toml does not list yet."""
    text: str = path.read_text() if path.exists() else ""
    listed: set[str] = {str(row["from"]) for row in tomllib.loads(text).get("alias", [])}
    lines: list[str] = []
    for form_id, (targets, reason) in sorted(folds.items()):
        if form_id not in listed:
            row: dict[str, object] = {"from": form_id, "to": list(targets), "reason": reason}
            lines += row_lines(table="alias", row=row)
    if lines:
        path.write_text(text.rstrip("\n") + "\n\n" + "\n".join(lines))
