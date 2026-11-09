"""The cases of one declared form: its arguments' hazards and its row sets'."""

from __future__ import annotations

import re
from typing import Any

from scripts.case_generators.models import Case, Column
from scripts.hazard_cases._helpers.combinations import combination_cases, distinct_fill
from scripts.hazard_cases.constants import (
    COMBINED_HAZARDS,
    EXPRESSION_SLOT,
    HAZARD_ARGUMENT_PREFIX,
    HAZARD_GROUP_COLUMN,
    HAZARD_QUERY,
    LITERAL_KIND,
)


def form_cases(
    *, form: dict[str, Any], suffix: str, families: dict[str, Any], tags: tuple[str, ...]
) -> list[Case]:
    """The cases of one form: its combinations, each argument's hazards, its row-set hazards."""
    slug: str = construct_slug(form["construct"]) + suffix
    combined: list[Case] = combination_cases(form=form, slug=slug, families=families, tags=tags)
    return combined + _hazard_cases(form=form, slug=slug, families=families, tags=tags)


def recordable(hazards: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The hazards a case can hold: those with cells and without an unrecordable status."""
    return [hazard for hazard in hazards if hazard.get("cells") and not hazard.get("unrecordable")]


def hazard_divergence(
    *,
    form: dict[str, Any],
    hazard: dict[str, Any],
    place: tuple[int, str] | None = None,
) -> str | None:
    """The divergence a case holding `hazard` at `place` expects; an empty value means none."""
    declared: dict[str, str] = form.get("divergences", {})
    keys: list[str] = [hazard["id"]]
    if place is not None:
        position, family = place
        keys = [f"{position}.{family}.{hazard['id']}", f"{family}.{hazard['id']}", *keys]
    found: str | None = next(
        (declared[key] for key in keys if key in declared), hazard.get("divergence")
    )
    return found or None


def _hazard_cases(
    *, form: dict[str, Any], slug: str, families: dict[str, Any], tags: tuple[str, ...]
) -> list[Case]:
    """Each argument family's hazards, then the form's row-set hazards."""
    arguments: list[dict[str, Any]] = form.get("arguments") or []
    chosen: list[str] = [argument["families"][0] for argument in arguments]
    if any(families.get(family, {}).get("sql_type") is None for family in chosen):
        return []
    cases: list[Case] = []
    for position, argument in enumerate(arguments):
        if argument.get("grammar", False):
            continue
        literal_only: bool = argument.get("kinds") == [LITERAL_KIND]
        for family in argument["families"]:
            if family not in families or families[family].get("sql_type") is None:
                continue
            types: list[str] = [*chosen]
            types[position] = family
            hazards: list[dict[str, Any]] = [
                hazard
                for hazard in recordable(families[family].get("hazard", []))
                if f"{position}.{family}.{hazard['id']}" not in form.get("unrecordable", {})
            ]
            alone: list[dict[str, Any]] = [
                hazard
                for hazard in hazards
                if literal_only
                or hazard.get("isolate")
                or hazard.get("divergence")
                or hazard_divergence(form=form, hazard=hazard, place=(position, family)) is not None
                or f"{position}.{family}.{hazard['id']}" in form.get("alone", [])
            ]
            together: list[dict[str, Any]] = [hazard for hazard in hazards if hazard not in alone]
            groups: list[tuple[str, list[dict[str, Any]]]] = [
                (hazard["id"], [hazard]) for hazard in alone
            ]
            if together:
                groups.insert(0, (COMBINED_HAZARDS, together))
            for name, group in groups:
                cells: list[str] = _cells(group)
                described: str = (
                    group[0]["description"].rstrip(".")
                    if len(group) == 1
                    else "the hazards " + ", ".join(hazard["id"] for hazard in group)
                )
                cases.append(
                    _argument_case(
                        declared=(form, families),
                        types=types,
                        hazard_at=(position, cells, literal_only),
                        labels=(
                            f"{slug}_{position}_{family}_{name}",
                            f"{form['construct']} with {described} in argument {position}, "
                            f"{family}.",
                            tags,
                        ),
                        divergence=(
                            hazard_divergence(form=form, hazard=group[0], place=(position, family))
                            if len(group) == 1
                            else None
                        ),
                    )
                )
    rows: str | None = form.get("rows")
    if rows is not None and rows in families and arguments:
        for hazard in families[rows].get("hazard", []):
            if hazard.get("unrecordable"):
                continue
            cases.append(
                _rows_case(
                    declared=(form, families),
                    types=chosen,
                    hazard=hazard,
                    labels=(f"{slug}_{rows}_{hazard['id']}", tags),
                    divergence=hazard_divergence(form=form, hazard=hazard),
                )
            )
    return cases


def _argument_case(
    *,
    declared: tuple[dict[str, Any], dict[str, Any]],
    types: list[str],
    hazard_at: tuple[int, list[str], bool],
    labels: tuple[str, str, tuple[str, ...]],
    divergence: str | None,
) -> Case:
    """One hazard's cells in one argument; the others hold their family's values, each distinct."""
    form, families = declared
    position, cells, literal_only = hazard_at
    case_id, description, tags = labels
    literal_positions: set[int] = {
        index
        for index, argument in enumerate(form.get("arguments") or [])
        if argument.get("kinds") == [LITERAL_KIND]
    }
    names: list[str] = [f"{HAZARD_ARGUMENT_PREFIX}{index}" for index in range(len(types))]
    columns: list[Column] = [
        Column(names[index], _sql_type(form=form, families=families, position=(index, family)))
        for index, family in enumerate(types)
        if index not in literal_positions
    ]
    benign: list[str] = distinct_fill(form=form, families=families, types=types, skip=position)
    grouped: bool = EXPRESSION_SLOT in (form.get("query") or "")
    if grouped:
        columns.insert(0, Column(HAZARD_GROUP_COLUMN, "VARCHAR"))
    if literal_only:
        expressions: list[str] = [
            _call(
                template=form["template"],
                names=names,
                literal_positions=literal_positions,
                benign=benign,
                replaced=(position, cell),
            )
            for cell in cells
        ]
        selected: str = ", ".join(
            f"{expression} AS r{index}" for index, expression in enumerate(expressions)
        )
        row: tuple[str, ...] = tuple(
            benign[index] for index in range(len(types)) if index not in literal_positions
        )
        query: str = f"SELECT {selected} FROM input ORDER BY ordinal"
        if grouped:
            row = ("'a'", *row)
            query = form["query"].replace(f"{EXPRESSION_SLOT} AS r", selected)
        return Case(
            id=case_id,
            description=description,
            columns=tuple(columns),
            rows=(row,),
            tags=tags,
            query=query,
            order="ordered",
            expected_divergence=divergence,
        )
    ordinals: set[int] = {int(index) for index in form.get("ordinal", [])} - {position}
    rows: list[tuple[str, ...]] = []
    for number, cell in enumerate(cells, start=1):
        values: list[str] = [*benign]
        values[position] = cell
        for index in ordinals:
            values[index] = str(number)
        row = tuple(values[index] for index in range(len(types)) if index not in literal_positions)
        group: str = f"'g{number:03d}'" if form.get("row_groups") else "'a'"
        rows.append((group, *row) if grouped else row)
    expression: str = _call(
        template=form["template"],
        names=names,
        literal_positions=literal_positions,
        benign=benign,
        replaced=None,
    )
    if grouped:
        return Case(
            id=case_id,
            description=description,
            columns=tuple(columns),
            rows=tuple(rows),
            tags=tags,
            query=form["query"].replace(EXPRESSION_SLOT, expression),
            order="ordered",
            expected_divergence=divergence,
        )
    return Case(
        id=case_id,
        description=description,
        columns=tuple(columns),
        rows=tuple(rows),
        tags=tags,
        expression=expression,
        expected_divergence=divergence,
    )


def _rows_case(
    *,
    declared: tuple[dict[str, Any], dict[str, Any]],
    types: list[str],
    hazard: dict[str, Any],
    labels: tuple[str, tuple[str, ...]],
    divergence: str | None,
) -> Case:
    """A row-set hazard: groups of the first argument's values, the others distinct per family."""
    form, families = declared
    case_id, tags = labels
    literal_positions: set[int] = {
        index
        for index, argument in enumerate(form.get("arguments") or [])
        if argument.get("kinds") == [LITERAL_KIND]
    }
    names: list[str] = [f"{HAZARD_ARGUMENT_PREFIX}{index}" for index in range(len(types))]
    benign: list[str] = distinct_fill(form=form, families=families, types=types)
    first: dict[str, Any] = families[types[0]]
    columns: list[Column] = [Column(HAZARD_GROUP_COLUMN, "VARCHAR")] + [
        Column(names[index], _sql_type(form=form, families=families, position=(index, family)))
        for index, family in enumerate(types)
        if index not in literal_positions
    ]
    template_rows: list[list[str]] = hazard.get("rows") or [["'a'", "{benign}"]]
    ordinals: set[int] = {int(index) for index in form.get("ordinal", [])} - {0}
    rows: list[tuple[str, ...]] = []
    for number, (group, value) in enumerate(template_rows, start=1):
        cell: str = value.replace("{benign}", first["benign"]).replace("{other}", first["other"])
        values: list[str] = [cell, *benign[1:]]
        for index in ordinals:
            values[index] = _reversed_ordinal(rows=len(template_rows), number=number)
        rows.append(
            (
                group,
                *(values[index] for index in range(len(types)) if index not in literal_positions),
            )
        )
    expression: str = _call(
        template=form["template"],
        names=names,
        literal_positions=literal_positions,
        benign=benign,
        replaced=None,
    )
    query: str = hazard.get("query") or form.get("query") or HAZARD_QUERY
    return Case(
        id=case_id,
        description=f"{form['construct']} over {hazard['description'].rstrip('.').lower()}.",
        columns=tuple(columns),
        rows=tuple(rows),
        tags=tags,
        query=query.replace(EXPRESSION_SLOT, expression),
        order="ordered",
        expected_divergence=divergence,
    )


def _reversed_ordinal(*, rows: int, number: int) -> str:
    """An ordering key against insertion order and apart from every value, so WITHIN GROUP shows."""
    return str(10 * (rows - number + 1))


def _cells(group: list[dict[str, Any]]) -> list[str]:
    """The cells of a group of hazards, each once, in catalogue order."""
    cells: list[str] = []
    for hazard in group:
        for cell in hazard["cells"]:
            if cell not in cells:
                cells.append(cell)
    return cells


def _sql_type(*, form: dict[str, Any], families: dict[str, Any], position: tuple[int, str]) -> str:
    """The Snowflake type of an argument's column: the form's override, else its family's."""
    index, family = position
    return form.get("sql_types", {}).get(str(index), families[family]["sql_type"])


def _call(
    *,
    template: str,
    names: list[str],
    literal_positions: set[int],
    benign: list[str],
    replaced: tuple[int, str] | None,
) -> str:
    """The template with each argument: its column, or a literal for literal-only positions."""
    arguments: list[str] = [
        benign[index] if index in literal_positions else name for index, name in enumerate(names)
    ]
    if replaced is not None:
        arguments[replaced[0]] = replaced[1]
    return re.sub(r"\{(\d+)\}", lambda match: arguments[int(match.group(1))], template)


def construct_slug(construct: str) -> str:
    """A construct key as an identifier fragment, such as `fn_get` or `node_like`."""
    return re.sub(r"[^a-z0-9]+", "_", construct.lower()).strip("_")
