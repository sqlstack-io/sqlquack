"""Semi-structured grids, every projection a scalar: paths into documents, and JSON parsing."""

from __future__ import annotations

import json
from decimal import Decimal, InvalidOperation

from scripts.case_generators.constants import (
    BOOLEAN_TEXTS,
    CONVERTIBLE_LIMIT,
    DECIMAL_POINT,
    DOCUMENTS,
    INVALID_JSON_TEXTS,
    JSON_FIDELITY,
    JSON_LONG_DECIMAL_PATTERN,
    JSON_NUMBER_PATTERN,
    JSON_PATHS,
    JSON_STRING_PATTERN,
    JSON_SURROGATE_PATTERN,
    JSON_TEXTS,
    JSON_UNSAFE_PATTERN,
    JSON_WORD_PATTERN,
    MISSING,
    NULL_CELL,
    PARSE_JSON_TEXTS,
    PATHS,
    SCALAR_PROJECTIONS,
    SEMI_STRUCTURED_TAGS,
    SNOWFLAKE_REJECTED_JSON_TEXTS,
    TEXT_PROJECTIONS,
)
from scripts.case_generators.exceptions import StrictJsonError
from scripts.case_generators.models import Case, CaseFile, Column, Path, Projection
from scripts.case_generators.types import Row, Target


def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    """An object from its pairs, raising when a key repeats."""
    keys: list[str] = [key for key, _ in pairs]
    if len(set(keys)) != len(keys):
        raise StrictJsonError(keys)
    return dict(pairs)


def reject_constant(name: str) -> object:
    """Rejects NaN and Infinity, which strict JSON has no spelling for."""
    raise StrictJsonError(name)


def fidelity_trigger(text: str | None) -> bool:
    """Whether converted PARSE_JSON raises json_fidelity for `text`, as duckdb.json_guarded does."""
    return guard_trigger(text=text, written_back=True)


def pattern_trigger(text: str | None) -> bool:
    """Whether `text` holds a pattern the guard checks, a long decimal written back or not."""
    return guard_trigger(text=text, written_back=False)


def guard_trigger(*, text: str | None, written_back: bool) -> bool:
    """Whether the guard raises for `text`; long decimals pass when `written_back` and unchanged."""
    if text is None or not text.strip(" "):
        return False
    try:
        json.loads(text, object_pairs_hook=unique_object, parse_constant=reject_constant)
    except ValueError:
        return True
    tokens: str = JSON_WORD_PATTERN.sub("", JSON_STRING_PATTERN.sub('""', text))
    long_decimal: bool = JSON_LONG_DECIMAL_PATTERN.search(tokens) is not None
    return any(
        (
            JSON_UNSAFE_PATTERN.search(tokens),
            long_decimal and not (written_back and numbers_written_back(tokens)),
            JSON_SURROGATE_PATTERN.search(text),
        )
    )


def numbers_written_back(tokens: str) -> bool:
    """Whether DuckDB writes each number in `tokens` as written: shortest digits, positional."""
    return all(
        DECIMAL_POINT not in token or format(Decimal(repr(float(token))), "f") == token
        for token in JSON_NUMBER_PATTERN.findall(tokens)
    )


def divergence_for(*, text: str, accepted: bool) -> str | None:
    """json_fidelity when the guard raises for `text` and Snowflake returns a value for it."""
    rejected: bool = text in SNOWFLAKE_REJECTED_JSON_TEXTS
    return JSON_FIDELITY if fidelity_trigger(text) and (accepted or not rejected) else None


def walk(*, document: str | None, steps: tuple[str | int, ...]) -> object:
    """The value at `steps`, or MISSING; a SQL NULL document yields MISSING."""
    if document is None:
        return MISSING
    value: object = json.loads(document)
    for step in steps:
        if isinstance(step, int) and isinstance(value, list) and step < len(value):
            value = value[step]
        elif isinstance(step, str) and isinstance(value, dict) and step in value:
            value = value[step]
        else:
            return MISSING
    return value


def numeric(value: object) -> bool:
    """Whether a JSON value casts to a number without raising."""
    if isinstance(value, bool):
        return False
    if isinstance(value, int | float):
        return Decimal(value).copy_abs() < CONVERTIBLE_LIMIT
    if isinstance(value, str):
        try:
            return Decimal(value.strip()).copy_abs() < CONVERTIBLE_LIMIT
        except InvalidOperation:
            return False
    return False


def converts(*, value: object, target: Target) -> bool:
    """Whether a path value casts to a number or a boolean without raising."""
    if value is MISSING or value is None:
        return True
    if target == Target.BOOLEAN:
        return isinstance(value, bool) or (isinstance(value, str) and value in BOOLEAN_TEXTS)
    return numeric(value)


def cell(document: str | None) -> str:
    """A VARIANT cell: PARSE_JSON of the document, or NULL."""
    if document is None:
        return NULL_CELL
    return "PARSE_JSON('" + document.replace("'", "''") + "')"


def text_case(*, path: Path, projection: Projection) -> Case:
    """A projection of one path over every document."""
    expression: str = projection.template.format(path=path.expression)
    return Case(
        id=f"{path.slug}_{projection.slug}",
        description=f"{expression} over objects, arrays, scalars, JSON null and SQL NULL.",
        columns=(Column("v", "VARIANT"),),
        rows=tuple((cell(document),) for document in DOCUMENTS),
        expression=expression,
        tags=("null", *path.tags),
    )


def scalar_case(*, path: Path, projection: Projection, target: Target) -> Case:
    """A cast of one path over the documents whose value converts."""
    expression: str = projection.template.format(path=path.expression)
    rows: tuple[Row, ...] = tuple(
        (cell(document),)
        for document in DOCUMENTS
        if converts(value=walk(document=document, steps=path.steps), target=target)
    )
    return Case(
        id=f"{path.slug}_{projection.slug}",
        description=f"{expression} over the documents whose value converts.",
        columns=(Column("v", "VARIANT"),),
        rows=rows,
        expression=expression,
        tags=("null", *path.tags),
    )


def path_file(*, slug: str, construct: str, cases: tuple[Case, ...]) -> CaseFile:
    """A semi-structured file of path cases."""
    return CaseFile(
        generator=JSON_PATHS,
        name=slug,
        category="semi-structured",
        construct=construct,
        cases=cases,
        header=("Paths into a fixed set of documents.",),
    )


def path_files() -> list[CaseFile]:
    """One file per projection, one case per path."""
    files: list[CaseFile] = []
    for projection in TEXT_PROJECTIONS:
        texts: tuple[Case, ...] = tuple(
            text_case(path=path, projection=projection) for path in PATHS
        )
        files.append(path_file(slug=projection.slug, construct=projection.construct, cases=texts))
    for projection in SCALAR_PROJECTIONS:
        target: Target = projection.target or Target.NUMBER
        cases: tuple[Case, ...] = tuple(
            scalar_case(path=path, projection=projection, target=target) for path in PATHS
        )
        files.append(path_file(slug=projection.slug, construct=projection.construct, cases=cases))
    return files


def literal(text: str) -> str:
    """A VARCHAR cell."""
    return "'" + text.replace("'", "''") + "'"


def parse_cases(projection: Projection) -> tuple[Case, Case]:
    """TRY_PARSE_JSON and PARSE_JSON under a projection, over the texts the guard passes."""
    clean: tuple[str, ...] = tuple(text for text in JSON_TEXTS if not pattern_trigger(text))
    blank: tuple[str, ...] = tuple(text for text in INVALID_JSON_TEXTS if not pattern_trigger(text))
    valid_rows: tuple[Row, ...] = (*((literal(text),) for text in clean), (NULL_CELL,))
    attempt: str = projection.template.format(path="TRY_PARSE_JSON(x)")
    parse: str = projection.template.format(path="PARSE_JSON(x)")
    return (
        Case(
            id=f"try_parse_json_{projection.slug}",
            description=f"{attempt} over JSON text the guard passes, empty and blank text.",
            columns=(Column("x", "VARCHAR"),),
            rows=(*valid_rows, *((literal(text),) for text in blank)),
            expression=attempt,
            tags=(*SEMI_STRUCTURED_TAGS, "invalid"),
        ),
        Case(
            id=f"parse_json_{projection.slug}",
            description=f"{parse} over valid JSON text the guard passes.",
            columns=(Column("x", "VARCHAR"),),
            rows=valid_rows,
            expression=parse,
            tags=SEMI_STRUCTURED_TAGS,
        ),
    )


def guarded_cases(*, function: str, texts: tuple[str, ...], accepted: bool) -> list[Case]:
    """TYPEOF of `function` over each text the guard raises for, alone."""
    triggered: list[str] = [text for text in texts if pattern_trigger(text)]
    return [
        Case(
            id=f"{function.lower()}_guard_{index + 1}",
            description=f"{function} of the text {literal(text)}, which the JSON guard raises for.",
            columns=(Column("x", "VARCHAR"),),
            rows=((literal(text),),),
            expression=f"TYPEOF({function}(x))",
            tags=("invalid",),
            expected_divergence=divergence_for(text=text, accepted=accepted),
        )
        for index, text in enumerate(triggered)
    ]


def invalid_parse_case(index: int) -> Case:
    """PARSE_JSON of one invalid text alone."""
    text: str = INVALID_JSON_TEXTS[index]
    return Case(
        id=f"parse_json_invalid_{index + 1}",
        description=f"PARSE_JSON of the text {literal(text)} raises or converts.",
        columns=(Column("x", "VARCHAR"),),
        rows=((literal(text),),),
        expression="TO_JSON(PARSE_JSON(x))",
        tags=("invalid",),
        expected_divergence=divergence_for(text=text, accepted=False),
    )


def parse_files() -> list[CaseFile]:
    """TRY_PARSE_JSON over all texts; PARSE_JSON over valid texts and each invalid one alone."""
    pairs: list[tuple[Case, Case]] = [parse_cases(projection) for projection in TEXT_PROJECTIONS]
    invalid: list[Case] = [invalid_parse_case(index) for index in range(len(INVALID_JSON_TEXTS))]
    return [
        CaseFile(
            generator=PARSE_JSON_TEXTS,
            name="try_parse_json",
            category="semi-structured",
            construct="fn:TRY_PARSE_JSON",
            cases=(
                *(attempt for attempt, _ in pairs),
                *guarded_cases(
                    function="TRY_PARSE_JSON",
                    texts=(*JSON_TEXTS, *INVALID_JSON_TEXTS),
                    accepted=True,
                ),
            ),
            header=("TRY_PARSE_JSON over valid and invalid JSON text.",),
        ),
        CaseFile(
            generator=PARSE_JSON_TEXTS,
            name="parse_json",
            category="semi-structured",
            construct="fn:PARSE_JSON",
            cases=(
                *(parse for _, parse in pairs),
                *invalid,
                *guarded_cases(function="PARSE_JSON", texts=JSON_TEXTS, accepted=True),
            ),
            header=("PARSE_JSON over valid JSON text, and each invalid text alone.",),
        ),
    ]
