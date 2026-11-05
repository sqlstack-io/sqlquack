"""Benign cases for each declared combination of a form: families, kinds, variants, partitions."""

from __future__ import annotations

import re
from itertools import product
from typing import Any

from scripts.case_generators.models import Case, Column
from scripts.hazard_cases.constants import (
    EXPRESSION_SLOT,
    HAZARD_ARGUMENT_PREFIX,
    HAZARD_GROUP_COLUMN,
    LITERAL_KIND,
    NULL_FAMILY,
    PARTITION_CLAUSE,
    PARTITION_FACET,
    SECOND_GROUP_COLUMN,
    SEVERAL_PARTITIONS,
    UNPARTITIONED,
    VALUE_KIND,
)
from scripts.hazard_cases.types import Position


def combination_cases(
    *, form: dict[str, Any], slug: str, families: dict[str, Any], tags: tuple[str, ...]
) -> list[Case]:
    """One benign case per template and declared argument combination, then per partition."""
    templates: list[str] = [form["template"], *form.get("variants", [])]
    cases: list[Case] = []
    for number, template in enumerate(templates):
        for positions in _position_choices(form=form, families=families):
            label: str = "_".join(
                f"{family}{'_literal' if literal else ''}" for family, literal in positions
            )
            cases.append(
                _benign_case(
                    form=form,
                    families=families,
                    shape=(template, positions),
                    labels=(f"{slug}_combination_{number}_{label}", tags),
                )
            )
    first: list[Position] | None = _first_positions(form=form, families=families)
    if form.get("extra_rows") and first is not None:
        cases.append(
            _extra_case(form=form, families=families, positions=first, labels=(slug, tags))
        )
    partitions: list[str] = form.get("each", {}).get(PARTITION_FACET, [])
    if partitions and PARTITION_CLAUSE in form["template"] and first is not None:
        for partition in partitions:
            case: Case | None = _partition_case(
                form=form,
                families=families,
                shape=(partition, first),
                labels=(f"{slug}_partition_{partition}", tags),
            )
            if case is not None:
                cases.append(case)
    return cases


def _position_choices(*, form: dict[str, Any], families: dict[str, Any]) -> list[list[Position]]:
    """Each argument's families and kinds, crossed, or one position at a time when one-wise."""
    choices: list[list[Position]] = []
    for argument in form.get("arguments") or []:
        kinds: list[str] = argument.get("kinds") or [VALUE_KIND]
        options: list[Position] = []
        stand_in: bool = str(len(choices)) in form.get("benign", {})
        for family in argument["families"]:
            if family == NULL_FAMILY or (stand_in and family not in families):
                options.append((family, True))
                continue
            if families.get(family, {}).get("sql_type") is None:
                continue
            options.extend((family, kind == LITERAL_KIND) for kind in kinds)
        if not options:
            return []
        choices.append(options)
    if not form.get("one_wise") or not choices:
        return [list(combination) for combination in product(*choices)]
    first: list[Position] = [options[0] for options in choices]
    picked: list[list[Position]] = []
    for index, options in enumerate(choices):
        for option in options:
            positions: list[Position] = [*first]
            positions[index] = option
            if positions not in picked:
                picked.append(positions)
    return picked


def _first_positions(*, form: dict[str, Any], families: dict[str, Any]) -> list[Position] | None:
    """The first declared choice of each argument, or `None` when an argument has none."""
    choices: list[list[Position]] = _position_choices(
        form={**form, "one_wise": True}, families=families
    )
    return choices[0] if choices else None


def _benign_case(
    *,
    form: dict[str, Any],
    families: dict[str, Any],
    shape: tuple[str, list[Position]],
    labels: tuple[str, tuple[str, ...]],
) -> Case:
    """Two rows of benign values in one combination of argument families and kinds."""
    template, positions = shape
    case_id, tags = labels
    columns, rows, arguments = _arguments(form=form, families=families, positions=positions)
    expression: str = re.sub(r"\{(\d+)\}", lambda match: arguments[int(match.group(1))], template)
    description: str = f"{form['construct']} over benign values of each declared argument kind."
    query: str | None = form.get("query")
    if query is not None and EXPRESSION_SLOT in query:
        return Case(
            id=case_id,
            description=description,
            columns=(Column(HAZARD_GROUP_COLUMN, "VARCHAR"), *columns),
            rows=tuple(
                (_group(form=form, number=number), *row) for number, row in enumerate(rows, start=1)
            ),
            tags=tags,
            query=query.replace(EXPRESSION_SLOT, expression),
            order="ordered",
        )
    return Case(
        id=case_id,
        description=description,
        columns=tuple(columns),
        rows=tuple(tuple(row) for row in rows),
        tags=tags,
        expression=expression,
    )


def _extra_case(
    *,
    form: dict[str, Any],
    families: dict[str, Any],
    positions: list[Position],
    labels: tuple[str, tuple[str, ...]],
) -> Case:
    """The form's declared extra rows, in its first combination of argument families."""
    slug, tags = labels
    columns, _, arguments = _arguments(form=form, families=families, positions=positions)
    expression: str = re.sub(
        r"\{(\d+)\}", lambda match: arguments[int(match.group(1))], form["template"]
    )
    return Case(
        id=f"{slug}_extra_rows",
        description=f"{form['construct']} over rows pairing values a single hazard cannot.",
        columns=tuple(columns),
        rows=tuple(tuple(row) for row in form["extra_rows"]),
        tags=tags,
        expression=expression,
    )


def _partition_case(
    *,
    form: dict[str, Any],
    families: dict[str, Any],
    shape: tuple[str, list[Position]],
    labels: tuple[str, tuple[str, ...]],
) -> Case | None:
    """The form's window partitioned by a key of one family, by two keys, or not at all."""
    partition, positions = shape
    case_id, tags = labels
    columns, cell_rows, arguments = _arguments(form=form, families=families, positions=positions)
    first_row, second_row = cell_rows[0], cell_rows[1]
    template: str = form["template"]
    keys: list[Column] = []
    values: list[tuple[str, ...]] = [("'a'",), ("'a'",), ("'b'",)]
    if partition == UNPARTITIONED:
        template = template.replace(PARTITION_CLAUSE + " ", "").replace(PARTITION_CLAUSE, "")
    elif partition == SEVERAL_PARTITIONS:
        template = template.replace(PARTITION_CLAUSE, f"{PARTITION_CLAUSE}, {SECOND_GROUP_COLUMN}")
        keys.append(Column(SECOND_GROUP_COLUMN, "VARCHAR"))
        values = [("'a'", "'x'"), ("'a'", "'x'"), ("'a'", "'y'")]
    elif families.get(partition, {}).get("sql_type") is not None:
        key: dict[str, Any] = families[partition]
        values = [(key["benign"],), (key["benign"],), (key["other"],)]
        keys.append(Column(HAZARD_GROUP_COLUMN, key["sql_type"]))
    else:
        return None
    if partition != SEVERAL_PARTITIONS and keys:
        group_columns: list[Column] = keys
    else:
        group_columns = [Column(HAZARD_GROUP_COLUMN, "VARCHAR"), *keys]
    expression: str = re.sub(r"\{(\d+)\}", lambda match: arguments[int(match.group(1))], template)
    rows: tuple[tuple[str, ...], ...] = (
        (*values[0], *first_row),
        (*values[1], *second_row),
        (*values[2], *first_row),
    )
    return Case(
        id=case_id,
        description=f"{form['construct']} partitioned by {partition.replace('_', ' ')}.",
        columns=(*group_columns, *columns),
        rows=rows,
        tags=tags,
        query=form["query"].replace(EXPRESSION_SLOT, expression),
        order="ordered",
    )


def family_values(declared: dict[str, Any], count: int = 0) -> list[str]:
    """A family's unremarkable values in order: benign, other, third where it has one, then as
    many of its `more` values as `count` arguments of the family need."""
    values: list[str] = [
        declared[key] for key in ("benign", "other", "third") if declared.get(key) is not None
    ]
    return values + list(declared.get("more", []))[: max(0, count - len(values))]


def distinct_fill(
    *, form: dict[str, Any], families: dict[str, Any], types: list[str], skip: int | None = None
) -> list[str]:
    """Each argument's cell: the form's override, else its family's next value, so arguments of
    one family differ. The `skip` position, which a hazard replaces, uses up no value."""
    overrides: dict[str, str] = form.get("benign", {})
    counts: dict[str, int] = {family: types.count(family) for family in types}
    seen: dict[str, int] = {}
    cells: list[str] = []
    for index, family in enumerate(types):
        if str(index) in overrides:
            cells.append(overrides[str(index)])
            continue
        values: list[str] = family_values(families.get(family, {}), counts[family]) or ["NULL"]
        cells.append(values[seen.get(family, 0) % len(values)])
        if index != skip:
            seen[family] = seen.get(family, 0) + 1
    return cells


def _arguments(
    *, form: dict[str, Any], families: dict[str, Any], positions: list[Position]
) -> tuple[list[Column], list[list[str]], list[str]]:
    """The columns, rows of cells and template arguments of a combination: benign then other,
    or, when a family repeats, each rotation of its values and then a row of equal values."""
    columns: list[Column] = []
    cells: list[tuple[str | None, list[str], int]] = []
    arguments: list[str] = []
    overrides: dict[str, str] = form.get("benign", {})
    types: dict[str, str] = form.get("sql_types", {})
    seen: dict[str, int] = {}
    for index, (family, literal) in enumerate(positions):
        if family == NULL_FAMILY:
            arguments.append("NULL")
            continue
        if family not in families:
            arguments.append(overrides[str(index)])
            continue
        declared: dict[str, Any] = families[family]
        if literal:
            arguments.append(overrides.get(str(index), declared["benign"]))
            continue
        name: str = f"{HAZARD_ARGUMENT_PREFIX}{index}"
        arguments.append(name)
        columns.append(Column(name, types.get(str(index), declared["sql_type"])))
        if str(index) in overrides:
            cells.append((None, [overrides[str(index)]], 0))
            continue
        cells.append((family, [], seen.get(family, 0)))
        seen[family] = seen.get(family, 0) + 1
    values_of: dict[str, list[str]] = {
        family: family_values(families[family], count) for family, count in seen.items()
    }
    resolved: list[tuple[list[str], int, int]] = [
        (values_of[family], occurrence, seen[family]) if family else (fixed, 0, 1)
        for family, fixed, occurrence in cells
    ]
    if not any(count > 1 for count in seen.values()):
        rows: list[list[str]] = [
            [values[shift % len(values)] for values, _, _ in resolved] for shift in range(2)
        ]
        return columns, rows, arguments
    rows = [
        [_rotated(values, occurrence, count, shift) for values, occurrence, count in resolved]
        for shift in range(max(max(len(values), count) for values, _, count in resolved))
    ]
    rows.append([values[0] for values, _, _ in resolved])
    return columns, rows, arguments


def _rotated(values: list[str], occurrence: int, count: int, shift: int) -> str:
    """The cell of a family's `occurrence`-th argument in row `shift`: a rotation of its values
    when it has one per argument, else, as for booleans, the other value in that argument's own
    row only, so no two arguments are equal on every row."""
    if len(values) >= count:
        return values[(occurrence + shift) % len(values)]
    return values[1] if shift % count == occurrence else values[0]


def _group(*, form: dict[str, Any], number: int) -> str:
    """The group of a benign case's row: one group, or one per row for forms that need it."""
    return f"'g{number:03d}'" if form.get("row_groups") else "'a'"
