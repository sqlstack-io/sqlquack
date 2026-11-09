"""VARIANTs ordered, compared, aggregated and read as numbers: each kind alone and mixed."""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import replace

from scripts.case_generators._helpers.objects import parsed
from scripts.case_generators.constants import NULL_CELL
from scripts.case_generators.models import Case, CaseFile, Column
from scripts.case_generators.types import Row
from scripts.family_cases.constants import (
    VARIANT_AGGREGATED_DIVERGENCES,
    VARIANT_AGGREGATED_TYPES,
    VARIANT_EQUAL_PAIRS,
    VARIANT_EXTREME_DIVERGENCES,
    VARIANT_NUMBER_EDGES,
    VARIANT_NUMBER_FORMS,
    VARIANT_NUMBER_TEXTS,
    VARIANT_ORDER,
    VARIANT_ORDER_DIVERGENCES,
    VARIANT_ORDER_GROUPS,
    VARIANT_SPELLING_DIVERGENCE,
)


def group_rows(documents: tuple[str, ...]) -> tuple[Row, ...]:
    """Each document numbered, then JSON null and SQL NULL."""
    rows: list[Row] = [(str(index), parsed(document)) for index, document in enumerate(documents)]
    rows.append((str(len(documents)), parsed("null")))
    rows.append((str(len(documents) + 1), NULL_CELL))
    return tuple(rows)


def group_columns() -> tuple[Column, ...]:
    """A distinct key `k` and the VARIANT `v`."""
    return (Column("k", "NUMBER(4,0)"), Column("v", "VARIANT"))


def order_cases(group: tuple[str, tuple[str, ...]]) -> list[Case]:
    """ORDER BY, MIN and MAX, MIN_BY and MAX_BY and ordered ARRAY_AGG over one group of kinds."""
    slug, documents = group
    rows: tuple[Row, ...] = group_rows(documents)
    described: str = ", ".join(documents)
    base: Case = Case(
        id=slug,
        description=described,
        columns=group_columns(),
        rows=rows,
        tags=("null", "json-null"),
        order="ordered",
        expected_divergence=VARIANT_ORDER_DIVERGENCES.get(slug),
    )
    return [
        replace(
            base,
            id=f"order_by_{slug}",
            description=f"ORDER BY a VARIANT over {described}, JSON null and NULL.",
            query="SELECT k, TO_JSON(v) AS j FROM input ORDER BY v NULLS LAST, k",
            tags=("null", "json-null", "ties", "mixed-types"),
        ),
        replace(
            base,
            id=f"order_by_desc_{slug}",
            description=f"ORDER BY a VARIANT descending over {described}, JSON null and NULL.",
            query="SELECT k, TO_JSON(v) AS j FROM input ORDER BY v DESC NULLS FIRST, k",
            tags=("null", "json-null", "ties", "mixed-types"),
        ),
        replace(
            base,
            id=f"min_max_{slug}",
            description=f"MIN and MAX of a VARIANT over {described}, JSON null and NULL.",
            query="SELECT TO_JSON(MIN(v)) AS mn, TO_JSON(MAX(v)) AS mx FROM input",
            tags=("null", "json-null"),
            expected_divergence=VARIANT_EXTREME_DIVERGENCES.get(slug),
        ),
        replace(
            base,
            id=f"min_by_max_by_{slug}",
            description=f"MIN_BY and MAX_BY of a key by a VARIANT over {described}.",
            query="SELECT MIN_BY(k, v) AS lo, MAX_BY(k, v) AS hi FROM input",
            tags=("null", "json-null"),
        ),
        replace(
            base,
            id=f"array_agg_ordered_{slug}",
            description=f"ARRAY_AGG of keys WITHIN GROUP (ORDER BY a VARIANT) over {described}.",
            query="SELECT ARRAY_AGG(k) WITHIN GROUP (ORDER BY v, k) AS ks FROM input",
            tags=("null", "json-null"),
        ),
    ]


def equality_case() -> Case:
    """EQUAL_NULL, =, and IS DISTINCT FROM over pairs of VARIANTs of like and unlike kinds."""
    rows: list[Row] = [(parsed(left), parsed(right)) for left, right in VARIANT_EQUAL_PAIRS]
    rows.extend(((parsed("null"), NULL_CELL), (NULL_CELL, NULL_CELL), (parsed("1"), NULL_CELL)))
    return Case(
        id="equal_null_pairs",
        description=(
            "EQUAL_NULL, = and IS DISTINCT FROM of VARIANT pairs: numbers spelled differently, "
            "text against numbers and booleans, objects with keys reordered, arrays, JSON null "
            "and NULL."
        ),
        columns=(Column("v", "VARIANT"), Column("w", "VARIANT")),
        rows=tuple(rows),
        query=(
            "SELECT EQUAL_NULL(v, w) AS en, v = w AS eq, v IS DISTINCT FROM w AS distinct_from "
            "FROM input ORDER BY ordinal"
        ),
        order="ordered",
        tags=("null", "json-null", "mixed-types", "ties"),
        expected_divergence=VARIANT_SPELLING_DIVERGENCE,
    )


def number_cases(form: tuple[str, str]) -> list[Case]:
    """One numeric reading of VARIANTs over readable values, then each unusual value alone."""
    slug, sql = form
    rows: list[Row] = [(parsed(document),) for document in VARIANT_NUMBER_TEXTS]
    rows.extend(((parsed("null"),), (NULL_CELL,)))
    cases: list[Case] = [
        Case(
            id=slug,
            description=f"{sql} of numbers, numeric text, a boolean, JSON null and NULL.",
            columns=(Column("v", "VARIANT"),),
            rows=tuple(rows),
            query=f"SELECT {sql} AS r FROM input ORDER BY ordinal",
            order="ordered",
            tags=("null", "json-null", "boundary"),
            expected_divergence=VARIANT_SPELLING_DIVERGENCE,
        )
    ]
    cases.extend(
        Case(
            id=f"{slug}_edge_{edge_slug}",
            description=f"{sql} of the VARIANT {document}.",
            columns=(Column("v", "VARIANT"),),
            rows=((parsed(document),),),
            query=f"SELECT {sql} AS r FROM input",
            order="ordered",
            tags=("invalid",),
        )
        for edge_slug, document in VARIANT_NUMBER_EDGES
    )
    return cases


def aggregated_case(typed: tuple[str, str, tuple[str, ...]]) -> Case:
    """ARRAY_AGG of a typed column, no explicit VARIANT cast, read back as JSON text."""
    slug, data_type, values = typed
    rows: list[Row] = [(str(index), value) for index, value in enumerate(values)]
    rows.append((str(len(values)), NULL_CELL))
    return Case(
        id=f"array_agg_{slug}",
        description=f"ARRAY_AGG of a {data_type} column, ordered, as its JSON text and elements.",
        columns=(Column("k", "NUMBER(4,0)"), Column("x", data_type)),
        rows=tuple(rows),
        query=(
            "WITH a AS (SELECT ARRAY_AGG(x) WITHIN GROUP (ORDER BY k) AS arr FROM input) "
            "SELECT TO_JSON(arr) AS j, arr[0]::VARCHAR AS first_text, TYPEOF(arr[0]) AS first_type "
            "FROM a"
        ),
        order="ordered",
        tags=("null", "boundary"),
        expected_divergence=VARIANT_AGGREGATED_DIVERGENCES.get(slug),
    )


def flattened(groups: Iterable[list[Case]]) -> list[Case]:
    """Every case of every group, in order."""
    cases: list[Case] = []
    for group in groups:
        cases.extend(group)
    return cases


def variant_order_files() -> list[CaseFile]:
    """VARIANT ordering and aggregates by kind; equality; numbers read; typed values aggregated."""
    return [
        CaseFile(
            generator=VARIANT_ORDER,
            name="ordering",
            category="comparison",
            construct="node:ordered",
            cases=tuple(flattened(order_cases(group) for group in VARIANT_ORDER_GROUPS)),
            header=("VARIANTs as ORDER BY keys, MIN and MAX, MIN_BY and MAX_BY, by kind.",),
        ),
        CaseFile(
            generator=VARIANT_ORDER,
            name="equality",
            category="comparison",
            construct="fn:EQUAL_NULL",
            cases=(equality_case(),),
            header=("VARIANT equality across spellings and kinds.",),
        ),
        CaseFile(
            generator=VARIANT_ORDER,
            name="numbers",
            category="semi-structured-cast",
            construct="fn:TO_DECIMAL",
            cases=tuple(flattened(number_cases(form) for form in VARIANT_NUMBER_FORMS)),
            header=("TO_DECIMAL and TO_NUMBER of VARIANTs.",),
        ),
        CaseFile(
            generator=VARIANT_ORDER,
            name="typed_aggregates",
            category="semi-structured-typed",
            construct="fn:ARRAY_AGG",
            cases=tuple(aggregated_case(typed) for typed in VARIANT_AGGREGATED_TYPES),
            header=("Dates, timestamps and times aggregated into arrays without a VARIANT cast.",),
        ),
    ]
