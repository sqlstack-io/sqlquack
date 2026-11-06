"""Text read as TIMESTAMP_TZ and TIMESTAMP_LTZ, and what zoned values give: spellings x forms."""

from __future__ import annotations

from scripts.case_generators._helpers.temporal_texts import literal
from scripts.case_generators.constants import NULL_CELL
from scripts.case_generators.models import Case, CaseFile, Column
from scripts.family_cases.constants import (
    LOCAL_ZONE_MARK,
    ZONED_ATTACHED_OFFSETS,
    ZONED_COLUMN_TYPES,
    ZONED_COLUMN_VALUES,
    ZONED_COMPARISONS,
    ZONED_DATES,
    ZONED_EDGE_TEXTS,
    ZONED_FRACTIONS,
    ZONED_LENIENT_FORMS,
    ZONED_LITERAL_TYPES,
    ZONED_LITERALS,
    ZONED_OFFSET_DIVERGENCE,
    ZONED_OFFSET_LITERALS,
    ZONED_OFFSET_PROJECTIONS,
    ZONED_PROJECTIONS,
    ZONED_SEPARATORS,
    ZONED_SPACED_OFFSETS,
    ZONED_SPELLING_DIVERGENCE,
    ZONED_SPELLING_EDGES,
    ZONED_STRICT_EDGES_DEFERRED,
    ZONED_STRICT_FORMS,
    ZONED_SUBMICRO_DIVERGENCE,
    ZONED_SUBMICRO_PROJECTIONS,
    ZONED_SUBMICRO_COMPARISONS,
    ZONED_TEXTS,
    ZONED_WALL_TIMES,
)


def zoned_spellings() -> tuple[str, ...]:
    """Every recorded-shape spelling: both separators, fractions, attached and spaced offsets."""
    texts: list[str] = []
    for wall in ZONED_WALL_TIMES:
        date, time = wall.split(" ")
        for separator in ZONED_SEPARATORS:
            for fraction in ZONED_FRACTIONS:
                texts.extend(
                    f"{date}{separator}{time}{fraction}{offset}"
                    for offset in ZONED_ATTACHED_OFFSETS
                )
        for fraction in ZONED_FRACTIONS:
            texts.extend(f"{wall}{fraction}{offset}" for offset in ZONED_SPACED_OFFSETS)
    texts.extend(ZONED_DATES)
    return tuple(texts)


def spelling_rows() -> tuple[tuple[str, ...], ...]:
    """One row per spelling, then NULL."""
    rows: list[tuple[str, ...]] = [(literal(text),) for text in zoned_spellings()]
    rows.append((NULL_CELL,))
    return tuple(rows)


def projection_divergence(*, form_sql: str, projection_slug: str) -> str | None:
    """What a projection over the grid's +09:00, -05:30 and nanosecond rows raises."""
    if LOCAL_ZONE_MARK in form_sql:
        return ZONED_SUBMICRO_DIVERGENCE if projection_slug in ZONED_SUBMICRO_PROJECTIONS else None
    return ZONED_OFFSET_DIVERGENCE if projection_slug in ZONED_OFFSET_PROJECTIONS else None


def projection_case(*, form: tuple[str, str, str], projection: tuple[str, str]) -> Case:
    """One projection of one form over every spelling."""
    form_slug, form_sql, _ = form
    projection_slug, projection_sql = projection
    return Case(
        id=f"{form_slug}_{projection_slug}",
        description=(
            f"{projection_sql} of v = {form_sql}, over ISO text with ' ' or 'T', fractions, "
            "no offset, Z, attached and spaced offsets, dates alone and NULL."
        ),
        columns=(Column("s", "VARCHAR"),),
        rows=spelling_rows(),
        query=(
            f"WITH z AS (SELECT ordinal, {form_sql} AS v FROM input) "
            f"SELECT {projection_sql} AS r FROM z ORDER BY ordinal"
        ),
        order="ordered",
        tags=("null", "boundary"),
        expected_divergence=projection_divergence(
            form_sql=form_sql, projection_slug=projection_slug
        ),
    )


def edge_case(*, form: tuple[str, str, str], edge: tuple[str, str]) -> Case:
    """One unusual text read by one form, alone, so a raise is the case's own outcome."""
    form_slug, form_sql, _ = form
    edge_slug, text = edge
    return Case(
        id=f"{form_slug}_edge_{edge_slug}",
        description=f"{form_sql} of the text {literal(text)}, as an instant at UTC.",
        columns=(Column("s", "VARCHAR"),),
        rows=((literal(text),),),
        query=(
            f"WITH z AS (SELECT {form_sql} AS v FROM input) "
            "SELECT CONVERT_TIMEZONE('UTC', v)::TIMESTAMP_NTZ AS r FROM z"
        ),
        order="ordered",
        tags=("invalid",),
        expected_divergence=ZONED_SPELLING_DIVERGENCE
        if edge_slug in ZONED_SPELLING_EDGES
        else None,
    )


def lenient_edges_case(form: tuple[str, str, str]) -> Case:
    """Every unusual text read by one TRY_ form together: NULL or a value, never a raise."""
    form_slug, form_sql, _ = form
    rows: list[tuple[str, ...]] = [(literal(text),) for _, text in ZONED_EDGE_TEXTS]
    rows.append((NULL_CELL,))
    return Case(
        id=f"{form_slug}_edges",
        description=f"{form_sql} of unusual texts, as instants at UTC.",
        columns=(Column("s", "VARCHAR"),),
        rows=tuple(rows),
        query=(
            f"WITH z AS (SELECT ordinal, {form_sql} AS v FROM input) "
            "SELECT CONVERT_TIMEZONE('UTC', v)::TIMESTAMP_NTZ AS r FROM z ORDER BY ordinal"
        ),
        order="ordered",
        tags=("null", "invalid"),
        expected_divergence=ZONED_SPELLING_DIVERGENCE,
    )


def literal_case(*, spelling: tuple[str, str], target: str) -> Case:
    """A text literal cast to `target`, folded by Snowflake, read back session-independently."""
    spelling_slug, text = spelling
    value: str = f"{literal(text)}::{target}"
    return Case(
        id=f"literal_{spelling_slug}_{target.lower()}",
        description=f"The literal {value}: kept, as an instant at UTC, its wall time and year.",
        columns=(Column("k", "NUMBER(2,0)"),),
        rows=(("1",),),
        query=(
            f"SELECT {value} AS kept, CONVERT_TIMEZONE('UTC', {value})::TIMESTAMP_NTZ AS instant, "
            f"{value}::TIMESTAMP_NTZ AS wall, YEAR({value}) AS y FROM input"
        ),
        order="ordered",
        tags=("boundary",),
        expected_divergence=(
            ZONED_OFFSET_DIVERGENCE
            if spelling_slug in ZONED_OFFSET_LITERALS and target == ZONED_LITERAL_TYPES[0]
            else None
        ),
    )


def null_literal_case(target: str) -> Case:
    """A NULL text cast to `target`."""
    return Case(
        id=f"literal_null_{target.lower()}",
        description=f"NULL text cast to {target}, and its year.",
        columns=(Column("k", "NUMBER(2,0)"),),
        rows=(("1",),),
        query=(
            f"SELECT CAST(NULL AS VARCHAR)::{target} AS kept, "
            f"YEAR(CAST(NULL AS VARCHAR)::{target}) AS y FROM input"
        ),
        order="ordered",
        tags=("null",),
    )


def comparison_case(*, column_type: str, comparison: tuple[str, str]) -> Case:
    """A zoned column compared with text, a NTZ or a DATE."""
    comparison_slug, comparison_sql = comparison
    type_slug: str = column_type.split("(", 1)[0].lower()
    return Case(
        id=f"{type_slug}_column_{comparison_slug}",
        description=(
            f"{comparison_sql} for a {column_type} column at offsets either side of a year end."
        ),
        columns=(Column("z", column_type),),
        rows=tuple((value,) for value in ZONED_COLUMN_VALUES),
        query=f"SELECT {comparison_sql} AS r FROM input ORDER BY ordinal",
        order="ordered",
        tags=("null", "boundary"),
        expected_divergence=(
            ZONED_SUBMICRO_DIVERGENCE if comparison_slug in ZONED_SUBMICRO_COMPARISONS else None
        ),
    )


def form_file(form: tuple[str, str, str]) -> CaseFile:
    """Every projection of one form, then its unusual texts: alone if strict, together if TRY_."""
    cases: list[Case] = [
        projection_case(form=form, projection=projection) for projection in ZONED_PROJECTIONS
    ]
    if form in ZONED_STRICT_FORMS:
        cases.extend(
            edge_case(form=form, edge=edge)
            for edge in ZONED_EDGE_TEXTS
            if edge[0] not in ZONED_STRICT_EDGES_DEFERRED
        )
    else:
        cases.append(lenient_edges_case(form))
    return CaseFile(
        generator=ZONED_TEXTS,
        name=form[0],
        category="temporal",
        construct=form[2],
        cases=tuple(cases),
        header=(f"{form[1]} of text in every recorded spelling, and what the value gives.",),
    )


def literal_cases() -> list[Case]:
    """Each literal spelling cast to each zoned type, then NULL text."""
    cases: list[Case] = []
    for spelling in ZONED_LITERALS:
        cases.extend(
            literal_case(spelling=spelling, target=target) for target in ZONED_LITERAL_TYPES
        )
    cases.extend(null_literal_case(target) for target in ZONED_LITERAL_TYPES)
    return cases


def comparison_cases() -> list[Case]:
    """Each comparison for each zoned column type."""
    cases: list[Case] = []
    for column_type in ZONED_COLUMN_TYPES:
        cases.extend(
            comparison_case(column_type=column_type, comparison=comparison)
            for comparison in ZONED_COMPARISONS
        )
    return cases


def zoned_text_files() -> list[CaseFile]:
    """Text to zoned timestamps and back, by form; literals; zoned columns against other types."""
    files: list[CaseFile] = [form_file(form) for form in ZONED_STRICT_FORMS + ZONED_LENIENT_FORMS]
    files.append(
        CaseFile(
            generator=ZONED_TEXTS,
            name="literals",
            category="temporal",
            construct="node:cast",
            cases=tuple(literal_cases()),
            header=("Text literals cast to TIMESTAMP_TZ and TIMESTAMP_LTZ, ISO 'T' included.",),
        )
    )
    files.append(
        CaseFile(
            generator=ZONED_TEXTS,
            name="column_comparisons",
            category="temporal",
            construct="node:gte",
            cases=tuple(comparison_cases()),
            header=("Zoned columns compared with text, TIMESTAMP_NTZ and DATE.",),
        )
    )
    return files
