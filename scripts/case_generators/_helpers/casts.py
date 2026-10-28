"""Casts between scalar types, and the conversion functions spelled as calls."""

from __future__ import annotations

import re
from decimal import Decimal, InvalidOperation

from scripts.case_generators.constants import (
    BOUNDARY_MARKERS,
    CAST_NUMBER_LIMITS,
    CAST_SOURCES,
    CAST_TARGETS,
    CAST_TIME,
    CAST_VARCHAR,
    CASTS,
    CONVERSION_FUNCTIONS,
    DATE_TEXT_FORMATS,
    DATE_TEXT_VALUES,
    EPOCH_SCALES,
    EPOCH_WHOLE_SECONDS,
    NAIVE_SOURCES,
    NANOSECOND_TARGETS,
    NULL_CELL,
    NUMBER_TEXT_EXTRAS,
    NUMBER_TEXT_TYPES,
    OUTSIDE_NANOSECOND_YEARS,
    PAD_FILLERS,
    PAD_FUNCTIONS,
    PAD_LENGTHS,
    PAD_TEXTS,
    REGEXP_PARAMETER_PATTERNS,
    REGEXP_PARAMETER_SETS,
    SEARCH_FUNCTIONS,
    SEARCH_PARTS,
    SEARCH_TEXTS,
    SOURCE_VALUES,
    SUBMICRO,
    TEXT_INVALID,
    TEXT_SEARCH,
    TEXT_VALID,
    TIME_SUBMICRO,
    TIMESTAMP_RANGE,
    TIMESTAMP_TEXT_FORMATS,
    TIMESTAMP_TEXT_VALUES,
    TIMESTAMP_TZ_SUBMICRO,
    WHOLE_MICROSECONDS,
    ZONED_NANOSECOND_TARGETS,
)
from scripts.case_generators.models import Case, CaseFile, CastType, Column, NumberType
from scripts.case_generators.types import Row


def has_boundary(values: tuple[str, ...]) -> bool:
    """Whether the values include an extreme of their type."""
    for value in values:
        if any(marker in value for marker in BOUNDARY_MARKERS):
            return True
    return False


def fits_target(*, value: str, target: CastType) -> bool:
    """Whether a numeric sample stays inside a NUMBER target; other values always do."""
    limit: Decimal | None = CAST_NUMBER_LIMITS.get(target)
    if limit is None:
        return True
    try:
        return Decimal(value).copy_abs() < limit
    except InvalidOperation:
        return True


def has_submicro(value: str) -> bool:
    """Whether a timestamp literal has a non-zero digit below the microsecond."""
    match: re.Match[str] | None = SUBMICRO.search(value)
    return match is not None and match.group(1) != WHOLE_MICROSECONDS


def divergence(*, value: str, source: CastType, target: CastType) -> str | None:
    """The known divergence a date or naive timestamp relies on when cast to `target`, if any."""
    if source.name not in NAIVE_SOURCES:
        return None
    if target.name in NANOSECOND_TARGETS and value.startswith(OUTSIDE_NANOSECOND_YEARS):
        return TIMESTAMP_RANGE
    if target.name in ZONED_NANOSECOND_TARGETS and has_submicro(value):
        return TIMESTAMP_TZ_SUBMICRO
    return None


def divergence_case(
    *, source: CastType, target: CastType, name: str, values: tuple[str, ...]
) -> Case:
    """CAST of values that rely on the known divergence `name`."""
    return Case(
        id=f"{source.slug}_to_{target.slug}_{name}",
        description=(
            f"CAST from {source.name} to {target.name} of values DuckDB cannot hold "
            f"(divergence {name})."
        ),
        columns=(Column("x", source.name),),
        rows=tuple((value,) for value in values),
        expression=f"CAST(x AS {target.name})",
        tags=("boundary",),
        expected_divergence=name,
    )


def cast_cases(*, source: CastType, target: CastType) -> list[Case]:
    """CAST of `source` samples to `target`: in range, beyond a NUMBER target, or diverging."""
    values: tuple[str, ...] = SOURCE_VALUES[source]
    diverging: dict[str, tuple[str, ...]] = {}
    for value in values:
        name: str | None = divergence(value=value, source=source, target=target)
        if name is not None:
            diverging[name] = (*diverging.get(name, ()), value)
    held: tuple[str, ...] = tuple(
        value for value in values if divergence(value=value, source=source, target=target) is None
    )
    inside: tuple[str, ...] = tuple(
        value for value in held if fits_target(value=value, target=target)
    )
    beyond: tuple[str, ...] = tuple(value for value in held if value not in inside)
    tags: list[str] = ["null"]
    if has_boundary(inside):
        tags.append("boundary")
    cases: list[Case] = [
        Case(
            id=f"{source.slug}_to_{target.slug}",
            description=(
                f"CAST from {source.name} to {target.name}: converts, or raises for the type pair."
            ),
            columns=(Column("x", source.name),),
            rows=tuple((value,) for value in inside),
            expression=f"CAST(x AS {target.name})",
            tags=tuple(tags),
        )
    ]
    if beyond:
        cases.append(
            Case(
                id=f"{source.slug}_to_{target.slug}_beyond_type",
                description=(
                    f"CAST from {source.name} to {target.name} of values it cannot hold "
                    "raises or converts."
                ),
                columns=(Column("x", source.name),),
                rows=tuple((value,) for value in beyond),
                expression=f"CAST(x AS {target.name})",
                tags=("invalid", "boundary"),
            )
        )
    for name, diverging_values in diverging.items():
        cases.append(
            divergence_case(source=source, target=target, name=name, values=diverging_values)
        )
    return cases


def overlong_case(source: CastType) -> Case:
    """CAST of the longest sample of `source` to VARCHAR(1)."""
    longest: str = max((value for value in SOURCE_VALUES[source] if value != NULL_CELL), key=len)
    return Case(
        id=f"{source.slug}_to_varchar_overlong",
        description=(
            f"CAST of {source.name} to VARCHAR(1) when the text is longer raises or converts."
        ),
        columns=(Column("x", source.name),),
        rows=((longest,),),
        expression="CAST(x AS VARCHAR(1))",
        tags=("invalid", "boundary"),
    )


def invalid_text_case(*, target: CastType, index: int) -> Case:
    """CAST of one invalid text to `target`; the first keeps the plain `_invalid` id."""
    invalid: str = TEXT_INVALID[target][index]
    suffix: str = "" if index == 0 else f"_{index + 1}"
    return Case(
        id=f"varchar_to_{target.slug}_invalid{suffix}",
        description=f"CAST of the text {invalid} to {target.name} raises or converts.",
        columns=(Column("x", CAST_VARCHAR.name),),
        rows=((invalid,),),
        expression=f"CAST(x AS {target.name})",
        tags=("invalid",),
    )


def text_cases(target: CastType) -> tuple[Case, ...]:
    """Text to `target`: valid spellings, and each invalid spelling alone."""
    column: tuple[Column, ...] = (Column("x", CAST_VARCHAR.name),)
    invalid: tuple[Case, ...] = tuple(
        invalid_text_case(target=target, index=index) for index in range(len(TEXT_INVALID[target]))
    )
    return (
        Case(
            id=f"varchar_to_{target.slug}",
            description=(
                f"CAST of text spellings to {target.name}, including whitespace and edge forms."
            ),
            columns=column,
            rows=tuple((value,) for value in TEXT_VALID[target]),
            expression=f"CAST(x AS {target.name})",
            tags=("null", "boundary"),
            expected_divergence=TIME_SUBMICRO if target == CAST_TIME else None,
        ),
        *invalid,
    )


def try_cast_case(target: CastType) -> Case:
    """TRY_CAST of valid and invalid text to `target`."""
    return Case(
        id=f"varchar_try_cast_{target.slug}",
        description=(f"TRY_CAST of valid and invalid text to {target.name}: invalid text is NULL."),
        columns=(Column("x", CAST_VARCHAR.name),),
        rows=(
            *((value,) for value in TEXT_VALID[target]),
            *((value,) for value in TEXT_INVALID[target]),
        ),
        expression=f"TRY_CAST(x AS {target.name})",
        tags=("null", "invalid", "boundary"),
        expected_divergence=TIME_SUBMICRO if target == CAST_TIME else None,
    )


def source_file(source: CastType) -> CaseFile:
    """CAST from one source type to every other target, and to an overlong VARCHAR."""
    cases: list[Case] = []
    for target in CAST_TARGETS:
        if target != source:
            cases.extend(cast_cases(source=source, target=target))
    cases.append(overlong_case(source))
    return CaseFile(
        generator=CASTS,
        name=f"from_{source.slug}",
        category="conversion",
        construct="node:cast",
        cases=tuple(cases),
        header=(f"CAST from {source.name} to every scalar target type.",),
    )


def cast_files() -> list[CaseFile]:
    """One file per source type, and text's CAST and TRY_CAST files."""
    files: list[CaseFile] = [source_file(source) for source in CAST_SOURCES]
    text: list[Case] = []
    for target in TEXT_VALID:
        text.extend(text_cases(target))
    files.append(
        CaseFile(
            generator=CASTS,
            name="from_varchar",
            category="conversion",
            construct="node:cast",
            cases=tuple(text),
            header=("CAST from text to every scalar target type.",),
        )
    )
    files.append(
        CaseFile(
            generator=CASTS,
            name="try_from_varchar",
            category="conversion",
            construct="node:try_cast",
            cases=tuple(try_cast_case(target) for target in TEXT_VALID),
            header=("TRY_CAST from text to every scalar target type.",),
        )
    )
    return files


def number_texts(number_type: NumberType) -> tuple[str, ...]:
    """Text at a NUMBER type's limits: the largest value, one digit more, half-way rounding."""
    whole: int = number_type.precision - number_type.scale
    unit: Decimal = Decimal(1).scaleb(-number_type.scale)
    largest: Decimal = Decimal(10) ** whole - unit
    half: Decimal = unit / 2
    return (
        f"'{largest}'",
        f"'-{largest}'",
        f"'{Decimal(10) ** whole}'",
        f"'{unit + half}'",
        f"'-{unit + half}'",
        f"'{largest + half}'",
        "'0'",
    )


def number_cases(number_type: NumberType) -> list[Case]:
    """TRY_TO_NUMBER over limits and near-miss texts; TO_NUMBER over the readable limits."""
    column: tuple[Column, ...] = (Column("s", "VARCHAR"),)
    sizes: str = f"{number_type.precision}, {number_type.scale}"
    limits: tuple[Row, ...] = tuple((text,) for text in number_texts(number_type))
    return [
        Case(
            id=f"try_to_number_{number_type.slug}",
            description=f"TRY_TO_NUMBER(s, {sizes}) at the type's limits and over near-miss text.",
            columns=column,
            rows=(*limits, *single_rows(NUMBER_TEXT_EXTRAS), (NULL_CELL,)),
            expression=f"TRY_TO_NUMBER(s, {sizes})",
            tags=("null", "invalid", "boundary"),
        ),
        Case(
            id=f"to_number_{number_type.slug}",
            description=f"TO_NUMBER(s, {sizes}) of text within the type's range.",
            columns=column,
            rows=(*(row for row in limits if row != limits[2] and row != limits[5]), (NULL_CELL,)),
            expression=f"TO_NUMBER(s, {sizes})",
            tags=("null", "boundary"),
        ),
    ]


def epoch_case(scale: int) -> Case:
    """TO_TIMESTAMP of a NUMBER with `scale` decimals: whole seconds and the smallest fractions."""
    unit: Decimal = Decimal(1).scaleb(-scale)
    values: list[str] = []
    for seconds in EPOCH_WHOLE_SECONDS:
        values.append(f"{Decimal(seconds) + unit * (-1 if seconds < 0 else 1):.{scale}f}")
        values.append(f"{Decimal(seconds):.{scale}f}")
    precision: int = 10 + scale
    return Case(
        id=f"to_timestamp_scale_{scale}",
        description=f"TO_TIMESTAMP of NUMBER({precision},{scale}): epoch seconds with a fraction.",
        columns=(Column("n", f"NUMBER({precision},{scale})"),),
        rows=(*single_rows(tuple(values)), (NULL_CELL,)),
        expression="TO_TIMESTAMP(n)",
        tags=("null", "boundary"),
    )


def single_rows(values: tuple[str, ...]) -> tuple[Row, ...]:
    """One row per value."""
    return tuple((value,) for value in values)


def pad_rows() -> tuple[Row, ...]:
    """Every text with every length."""
    rows: list[Row] = []
    for text in PAD_TEXTS:
        rows.extend((text, length) for length in PAD_LENGTHS)
    return tuple(rows)


def pad_case(*, function: str, slug: str, filler: str) -> Case:
    """One pad function and pad over every text and length."""
    rows: tuple[Row, ...] = pad_rows()
    return Case(
        id=f"{function.lower()}_{slug}",
        description=f"{function}(s, n, {filler}) over short, empty, blank and non-ASCII text.",
        columns=(Column("s", "VARCHAR"), Column("n", "NUMBER(10,0)")),
        rows=rows,
        expression=f"{function}(s, n, {filler})",
        tags=("null", "empty", "whitespace", "unicode"),
    )


def pad_cases() -> tuple[Case, ...]:
    """Each pad function with each pad."""
    cases: list[Case] = []
    for function in PAD_FUNCTIONS:
        cases.extend(
            pad_case(function=function, slug=slug, filler=filler) for slug, filler in PAD_FILLERS
        )
    return tuple(cases)


def text_format_cases() -> list[Case]:
    """TO_VARCHAR of dates and timestamps in each format."""
    dates: list[Case] = [
        Case(
            id=f"date_{slug}",
            description=f"TO_VARCHAR(d, '{written}') of dates across the year range.",
            columns=(Column("d", "DATE"),),
            rows=single_rows(DATE_TEXT_VALUES),
            expression=f"TO_VARCHAR(d, '{written}')",
            tags=("null", "boundary"),
        )
        for slug, written in DATE_TEXT_FORMATS
    ]
    timestamps: list[Case] = [
        Case(
            id=f"timestamp_{slug}",
            description=f"TO_VARCHAR(t, '{written}') of naive timestamps; fractions are dropped.",
            columns=(Column("t", "TIMESTAMP_NTZ"),),
            rows=single_rows(TIMESTAMP_TEXT_VALUES),
            expression=f"TO_VARCHAR(t, '{written}')",
            tags=("null", "boundary"),
        )
        for slug, written in TIMESTAMP_TEXT_FORMATS
    ]
    return [*dates, *timestamps]


def search_cases() -> tuple[Case, ...]:
    """Each text search function with each part over every text."""
    rows: tuple[Row, ...] = single_rows(SEARCH_TEXTS)
    cases: list[Case] = []
    for function in SEARCH_FUNCTIONS:
        cases.extend(
            Case(
                id=f"{function.lower()}_{slug}",
                description=f"{function}(s, {part}) over cased, empty, blank and non-ASCII text.",
                columns=(Column("s", "VARCHAR"),),
                rows=rows,
                expression=f"{function}(s, {part})",
                tags=("null", "empty", "whitespace", "unicode"),
            )
            for slug, part in SEARCH_PARTS
        )
    return tuple(cases)


def regexp_parameter_cases() -> tuple[Case, ...]:
    """REGEXP_LIKE with each parameter string and pattern over every text."""
    rows: tuple[Row, ...] = single_rows(SEARCH_TEXTS)
    cases: list[Case] = []
    for slug, pattern in REGEXP_PARAMETER_PATTERNS:
        cases.extend(
            Case(
                id=f"regexp_like_{slug}_{name}",
                description=f"REGEXP_LIKE(s, '{pattern}', '{parameters}') over every text.",
                columns=(Column("s", "VARCHAR"),),
                rows=rows,
                expression=f"REGEXP_LIKE(s, '{pattern}', '{parameters}')",
                tags=("null", "empty", "whitespace", "unicode"),
            )
            for name, parameters in REGEXP_PARAMETER_SETS
        )
    return tuple(cases)


def text_search_files() -> list[CaseFile]:
    """Text search and REGEXP_LIKE parameter grids."""
    return [
        CaseFile(
            generator=TEXT_SEARCH,
            name="search_functions",
            category="string",
            construct="fn:CONTAINS",
            cases=search_cases(),
            header=("CONTAINS, STARTSWITH, ENDSWITH and EDITDISTANCE over parts and texts.",),
        ),
        CaseFile(
            generator=TEXT_SEARCH,
            name="regexp_parameters",
            category="string",
            construct="fn:REGEXP_LIKE",
            cases=regexp_parameter_cases(),
            header=("REGEXP_LIKE with c, i and s parameters over patterns and texts.",),
        ),
    ]


def conversion_function_files() -> list[CaseFile]:
    """The four conversion-function files."""
    numbers: list[Case] = []
    for number_type in NUMBER_TEXT_TYPES:
        numbers.extend(number_cases(number_type))
    return [
        CaseFile(
            generator=CONVERSION_FUNCTIONS,
            name="to_number_sizes",
            category="conversion",
            construct="fn:TRY_TO_NUMBER",
            cases=tuple(numbers),
            header=("TO_NUMBER and TRY_TO_NUMBER with a precision and scale.",),
        ),
        CaseFile(
            generator=CONVERSION_FUNCTIONS,
            name="to_timestamp_scales",
            category="temporal",
            construct="fn:TO_TIMESTAMP",
            cases=tuple(epoch_case(scale) for scale in EPOCH_SCALES),
            header=("TO_TIMESTAMP of NUMBERs with a scale, as seconds with a fraction.",),
        ),
        CaseFile(
            generator=CONVERSION_FUNCTIONS,
            name="pads",
            category="string",
            construct="fn:LPAD",
            cases=pad_cases(),
            header=("LPAD and RPAD over pads, lengths and texts.",),
        ),
        CaseFile(
            generator=CONVERSION_FUNCTIONS,
            name="text_formats",
            category="temporal",
            construct="fn:TO_VARCHAR",
            cases=tuple(text_format_cases()),
            header=("TO_VARCHAR of dates and timestamps with a format.",),
        ),
    ]
