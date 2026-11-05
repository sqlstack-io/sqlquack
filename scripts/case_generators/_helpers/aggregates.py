"""Numeric functions over the NUMBER and FLOAT grid, percentiles, and HASH properties."""

from __future__ import annotations

from decimal import Decimal, localcontext

from scripts.case_generators._helpers.numbers import largest, literal, samples
from scripts.case_generators.constants import (
    BITAND_ROWS,
    DECIMAL_CONTEXT_DIGITS,
    GRID_TYPES,
    HASH_INTEGER_PAIRS,
    HASH_MIXED_PAIRS,
    HASH_PROPERTIES,
    HASH_PROPERTY_COLUMNS,
    HASH_SCALAR_PAIRS,
    HASH_SORTED_JSON_PAIRS,
    HASH_TEXT_PAIRS,
    HASH_UNSORTED_JSON_PAIRS,
    INTEGER_TYPES,
    LARGE_SAMPLE,
    NULL_CELL,
    NUMERIC_FUNCTIONS,
    ORDERED_AGGREGATES,
    PERCENTILE_CONTINUOUS,
    PERCENTILE_DIRECTIONS,
    PERCENTILE_EXACT_FRACTIONS,
    PERCENTILE_EXACT_MAX_PRECISION,
    PERCENTILE_FRACTIONS,
    PERCENTILE_FUNCTIONS,
    PERCENTILE_GROUPS,
    PERCENTILE_INTERPOLATION,
    PERCENTILE_TYPES,
    ROUNDING_FUNCTIONS,
    SAFE_AT_LIMITS_FUNCTIONS,
)
from scripts.case_generators.models import Case, CaseFile, Column, NumberType
from scripts.case_generators.types import Row


def number_text(*, value: Decimal, scale: int) -> str:
    """A NUMBER literal with exactly `scale` decimals."""
    return f"{value:.{scale}f}"


def group_values(*, precision: int, scale: int) -> dict[str, list[Decimal | None]]:
    """Each group's values: extremes and a duplicate, a pair, one value, all NULL, a progression."""
    with localcontext() as context:
        context.prec = DECIMAL_CONTEXT_DIGITS
        unit: Decimal = Decimal(1).scaleb(-scale)
        largest: Decimal = Decimal(10) ** (precision - scale) - unit
        step: Decimal = Decimal(10) ** (precision - scale - 1)
        return {
            "a": [-largest, Decimal(-1), Decimal(0), unit, unit, largest, None],
            "b": [unit * 3, Decimal(-2)],
            "c": [Decimal(7)],
            "d": [None, None],
            "e": [unit, *(step * index for index in range(1, 10))],
        }


def rows(*, precision: int, scale: int) -> tuple[Row, ...]:
    """Every group's rows."""
    values: dict[str, list[Decimal | None]] = group_values(precision=precision, scale=scale)
    result: list[Row] = []
    for group in PERCENTILE_GROUPS:
        for value in values[group]:
            cell: str = NULL_CELL if value is None else number_text(value=value, scale=scale)
            result.append((f"'{group}'", cell))
    return tuple(result)


def projection(*, function: str, direction: str) -> str:
    """One output column per fraction."""
    return ", ".join(
        f"{function}({fraction}) WITHIN GROUP (ORDER BY x {direction}) AS p{index}"
        for index, fraction in enumerate(PERCENTILE_FRACTIONS)
    )


def percentile_case(*, function: str, direction: str, number_type: tuple[int, int]) -> Case:
    """One function and direction over one NUMBER type, every fraction a column."""
    precision, scale = number_type
    type_name: str = f"NUMBER({precision},{scale})"
    return Case(
        id=f"{function.lower()}_{direction.lower()}_n{precision}_{scale}",
        description=f"{function} {direction} of {type_name} at fractions from 0 to 1, per group.",
        columns=(Column("g", "VARCHAR"), Column("x", type_name)),
        rows=rows(precision=precision, scale=scale),
        query=(
            f"SELECT g, {projection(function=function, direction=direction)} "
            "FROM input GROUP BY g ORDER BY g"
        ),
        order="ordered",
        tags=("null", "all-null", "duplicates", "boundary"),
        expected_divergence=interpolated(function),
    )


def interpolated(function: str) -> str | None:
    """The divergence PERCENTILE_CONT relies on where it would interpolate between values."""
    return PERCENTILE_INTERPOLATION if function == PERCENTILE_CONTINUOUS else None


def exact_rows(*, precision: int, scale: int) -> tuple[Row, ...]:
    """Five distinct values, extremes included; four equal values; one value; all NULL."""
    unit: Decimal = Decimal(1).scaleb(-scale)
    largest: Decimal = Decimal(10) ** (precision - scale) - unit
    values: dict[str, list[Decimal | None]] = {
        "a": [largest, -largest, Decimal(0), unit, -unit, None],
        "b": [Decimal(3), Decimal(3), Decimal(3), Decimal(3)],
        "c": [Decimal(-7)],
        "d": [None, None],
    }
    result: list[Row] = []
    for group, listed in values.items():
        result.extend(
            (f"'{group}'", NULL_CELL if value is None else number_text(value=value, scale=scale))
            for value in listed
        )
    return tuple(result)


def exact_case(*, direction: str, number_type: tuple[int, int]) -> Case:
    """PERCENTILE_CONT at fractions landing on a row, where Snowflake does not interpolate."""
    precision, scale = number_type
    type_name: str = f"NUMBER({precision},{scale})"
    columns: str = ", ".join(
        f"PERCENTILE_CONT({fraction}) WITHIN GROUP (ORDER BY x {direction}) AS p{index}"
        for index, fraction in enumerate(PERCENTILE_EXACT_FRACTIONS)
    )
    return Case(
        id=f"percentile_cont_exact_{direction.lower()}_n{precision}_{scale}",
        description=f"PERCENTILE_CONT {direction} of {type_name} at fractions landing on rows.",
        columns=(Column("g", "VARCHAR"), Column("x", type_name)),
        rows=exact_rows(precision=precision, scale=scale),
        query=f"SELECT g, {columns} FROM input GROUP BY g ORDER BY g",
        order="ordered",
        tags=("null", "all-null", "duplicates", "boundary"),
    )


def exact_window_case() -> Case:
    """PERCENTILE_CONT at fractions landing on rows, OVER (PARTITION BY g)."""
    return Case(
        id="percentile_cont_exact_window",
        description="PERCENTILE_CONT at 0.25 and 0.5 OVER (PARTITION BY g), landing on rows.",
        columns=(Column("g", "VARCHAR"), Column("x", "NUMBER(10,2)")),
        rows=exact_rows(precision=10, scale=2),
        query=(
            "SELECT g, x, PERCENTILE_CONT(0.25) WITHIN GROUP (ORDER BY x) OVER (PARTITION BY g) "
            "AS q, PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY x) OVER (PARTITION BY g) AS m "
            "FROM input ORDER BY g, x"
        ),
        order="ordered",
        tags=("null", "all-null", "duplicates"),
    )


def window_case(function: str) -> Case:
    """One function over whole partitions, beside each row."""
    return Case(
        id=f"{function.lower()}_window",
        description=f"{function} at 0.25 and 0.5 OVER (PARTITION BY g), on every row.",
        columns=(Column("g", "VARCHAR"), Column("x", "NUMBER(10,2)")),
        rows=rows(precision=10, scale=2),
        query=(
            f"SELECT g, x, {function}(0.25) WITHIN GROUP (ORDER BY x) OVER (PARTITION BY g) "
            f"AS q, {function}(0.5) WITHIN GROUP (ORDER BY x) OVER (PARTITION BY g) AS m "
            "FROM input ORDER BY g, x"
        ),
        order="ordered",
        tags=("null", "all-null", "duplicates"),
        expected_divergence=interpolated(function),
    )


def empty_case(function: str) -> Case:
    """One function over no rows at all."""
    return Case(
        id=f"{function.lower()}_empty",
        description=f"{function} at 0.5 over no rows.",
        columns=(Column("g", "VARCHAR"), Column("x", "NUMBER(10,2)")),
        rows=rows(precision=10, scale=2),
        query=(
            f"SELECT {function}(0.5) WITHIN GROUP (ORDER BY x) AS m FROM input WHERE g = 'none'"
        ),
        order="ordered",
        tags=("empty-input",),
    )


def percentile_files() -> list[CaseFile]:
    """One file per function: every type and direction, and the window form."""
    files: list[CaseFile] = []
    for function in PERCENTILE_FUNCTIONS:
        cases: list[Case] = []
        for direction in PERCENTILE_DIRECTIONS:
            cases.extend(
                percentile_case(function=function, direction=direction, number_type=number_type)
                for number_type in PERCENTILE_TYPES
            )
        cases.extend((window_case(function), empty_case(function)))
        if function == PERCENTILE_CONTINUOUS:
            for direction in PERCENTILE_DIRECTIONS:
                cases.extend(
                    exact_case(direction=direction, number_type=number_type)
                    for number_type in PERCENTILE_TYPES
                    if number_type[0] <= PERCENTILE_EXACT_MAX_PRECISION
                )
            cases.append(exact_window_case())
        files.append(
            CaseFile(
                generator=ORDERED_AGGREGATES,
                name=function.lower(),
                category="aggregate",
                construct=f"fn:{function}",
                cases=tuple(cases),
                header=(f"{function} WITHIN GROUP over NUMBER types and fractions.",),
            )
        )
    return files


def function_slug(template: str) -> str:
    """An identifier fragment for a function template, such as `round_1` or `round_minus_1`."""
    name, _, rest = template.partition("(")
    digits: str = rest.removeprefix("{x}").strip(", )").replace("-", "minus_")
    return f"{name.lower()}_{digits}" if digits else name.lower()


def signed_samples(*, number_type: NumberType, with_largest: bool) -> tuple[Row, ...]:
    """The type's samples, their negations and NULL; the largest only when asked."""
    values: list[Decimal] = [
        value
        for name, value in samples(number_type).items()
        if with_largest or name != LARGE_SAMPLE
    ]
    texts: list[str] = [literal(value=value, number_type=number_type) for value in values]
    negated: list[str] = [negative(text) for text in texts if Decimal(text) != 0]
    return (*((text,) for text in [*texts, *negated]), (NULL_CELL,))


def negative(text: str) -> str:
    """A literal's negation, written without arithmetic so no digits are rounded."""
    return text.removeprefix("-") if text.startswith("-") else f"-{text}"


def numeric_function_case(*, template: str, number_type: NumberType, at_limits: bool) -> Case:
    """One function over one type's signed samples, or over its largest values alone."""
    suffix: str = "_largest" if at_limits else ""
    rows: tuple[Row, ...] = signed_samples(
        number_type=number_type, with_largest=template in SAFE_AT_LIMITS_FUNCTIONS
    )
    if at_limits:
        top: Decimal = largest(number_type) if not number_type.is_float else Decimal("1e300")
        text: str = literal(value=top, number_type=number_type)
        rows = ((text,), (negative(text),))
    expression: str = template.format(x="x")
    return Case(
        id=f"{function_slug(template)}_{number_type.slug}{suffix}",
        description=(
            f"{expression} of {number_type.name}"
            + (" at its largest values: raises or converts." if at_limits else ", signed and NULL.")
        ),
        columns=(Column("x", number_type.name),),
        rows=rows,
        expression=expression,
        tags=("boundary", "negative") if at_limits else ("null", "zero", "negative", "boundary"),
    )


def bitand_case(number_type: NumberType) -> Case:
    """BITAND of integers, negative ones and NULL."""
    return Case(
        id=f"bitand_{number_type.slug}",
        description=f"BITAND of {number_type.name} values, negative ones and NULL.",
        columns=(Column("a", number_type.name), Column("b", number_type.name)),
        rows=BITAND_ROWS,
        expression="BITAND(a, b)",
        tags=("null", "zero", "negative", "boundary"),
    )


def numeric_function_files() -> list[CaseFile]:
    """One file per function family: sign functions, rounding functions and BITAND."""
    signs: list[Case] = []
    for template in SAFE_AT_LIMITS_FUNCTIONS:
        signs.extend(
            numeric_function_case(template=template, number_type=number_type, at_limits=False)
            for number_type in GRID_TYPES
        )
    rounding: list[Case] = []
    for template in ROUNDING_FUNCTIONS:
        for number_type in GRID_TYPES:
            rounding.append(
                numeric_function_case(template=template, number_type=number_type, at_limits=False)
            )
            rounding.append(
                numeric_function_case(template=template, number_type=number_type, at_limits=True)
            )
    return [
        CaseFile(
            generator=NUMERIC_FUNCTIONS,
            name="sign",
            category="numeric",
            construct="fn:ABS",
            cases=tuple(signs),
            header=("ABS and SIGN over every NUMBER and FLOAT grid type.",),
        ),
        CaseFile(
            generator=NUMERIC_FUNCTIONS,
            name="rounding",
            category="numeric",
            construct="fn:ROUND",
            cases=tuple(rounding),
            header=(
                "ROUND, FLOOR, CEIL and TRUNC over the grid types, and at their largest values.",
            ),
        ),
        CaseFile(
            generator=NUMERIC_FUNCTIONS,
            name="bitand",
            category="numeric",
            construct="fn:BITAND",
            cases=tuple(bitand_case(number_type) for number_type in INTEGER_TYPES),
            header=("BITAND over integer types.",),
        ),
    ]


def hash_files() -> list[CaseFile]:
    """The HASH property file."""
    return [
        CaseFile(
            generator=HASH_PROPERTIES,
            name="properties",
            category="numeric",
            construct="fn:HASH",
            cases=(*pair_cases(), *order_cases(), *aggregate_cases()),
            header=(
                "HASH properties only: equal inputs hash alike, results are signed 64-bit, NULL",
                "has a hash, and argument order matters. No case outputs a hash value.",
            ),
        )
    ]


def pair_cases() -> tuple[Case, ...]:
    """HASH(a) = HASH(b) and per-value properties for pairs of each type."""
    grids: tuple[
        tuple[str, str, str, tuple[tuple[str, str], ...], str | None, tuple[str, ...]], ...
    ] = (
        ("text", "VARCHAR", "VARCHAR", HASH_TEXT_PAIRS, None, ("empty", "unicode")),
        (
            "integers",
            "NUMBER(10,0)",
            "NUMBER(38,0)",
            HASH_INTEGER_PAIRS,
            None,
            ("zero", "negative", "boundary"),
        ),
        ("text_and_number", "VARCHAR", "NUMBER(10,0)", HASH_MIXED_PAIRS, None, ("zero",)),
        ("booleans_and_dates", "BOOLEAN", "DATE", HASH_SCALAR_PAIRS, None, ()),
    )
    cases: list[Case] = [
        Case(
            id=f"hash_pairs_{slug}",
            description=f"HASH of {left} and {right} pairs: equal values hash alike, in range.",
            columns=(Column("a", left), Column("b", right)),
            rows=pairs,
            query=f"SELECT {HASH_PROPERTY_COLUMNS} FROM input ORDER BY ordinal",
            order="ordered",
            tags=("null", "equality", *extra),
            expected_divergence=divergence,
        )
        for slug, left, right, pairs, divergence, extra in grids
    ]
    cases.extend(
        Case(
            id=f"hash_pairs_json_{slug}",
            description=f"HASH of parsed JSON pairs with keys {slug}: equal documents hash alike.",
            columns=(Column("a", "VARCHAR"), Column("b", "VARCHAR")),
            rows=pairs,
            query=(
                "SELECT HASH(PARSE_JSON(a)) = HASH(PARSE_JSON(b)) AS same, "
                "HASH(PARSE_JSON(a)) IS NULL AS null_hash FROM input ORDER BY ordinal"
            ),
            order="ordered",
            tags=("json", "equality"),
            expected_divergence=divergence,
        )
        for slug, pairs, divergence in (
            ("sorted", HASH_SORTED_JSON_PAIRS, None),
            ("unsorted", HASH_UNSORTED_JSON_PAIRS, None),
        )
    )
    return tuple(cases)


def order_cases() -> tuple[Case, ...]:
    """Several arguments: order matters, and NULL arguments still hash."""
    return (
        Case(
            id="hash_argument_order",
            description="HASH(a, b) against HASH(b, a), HASH(a, b) and HASH(a || b).",
            columns=(Column("a", "VARCHAR"), Column("b", "VARCHAR")),
            rows=HASH_TEXT_PAIRS,
            query=(
                "SELECT HASH(a, b) = HASH(b, a) AS swapped, HASH(a, b) = HASH(a, b) AS stable, "
                "HASH(a, b) = HASH(a || b) AS concatenated, HASH(a, b) IS NULL AS null_hash "
                "FROM input ORDER BY ordinal"
            ),
            order="ordered",
            tags=("null", "order"),
        ),
        Case(
            id="hash_null_types",
            description="HASH of NULL of each type, against each other.",
            columns=(Column("a", "VARCHAR"),),
            rows=(("NULL",),),
            query=(
                "SELECT HASH(NULL) IS NULL AS null_hash, "
                "HASH(CAST(NULL AS VARCHAR)) = HASH(CAST(NULL AS NUMBER(10,0))) AS text_number, "
                "HASH(CAST(NULL AS VARCHAR)) = HASH(NULL) AS text_untyped, "
                "HASH(a, NULL) = HASH(NULL, a) AS null_pair FROM input"
            ),
            order="ordered",
            tags=("null",),
        ),
    )


def aggregate_cases() -> tuple[Case, ...]:
    """Distinct values keep distinct hashes over many rows."""
    return (
        Case(
            id="hash_distinct_count",
            description="COUNT(DISTINCT HASH(a)) equals COUNT(DISTINCT a) over non-NULL values.",
            columns=(Column("a", "VARCHAR"), Column("b", "VARCHAR")),
            rows=HASH_TEXT_PAIRS,
            query=(
                "SELECT COUNT(DISTINCT HASH(a)) = COUNT(DISTINCT a) AS distinct_kept, "
                "COUNT(DISTINCT HASH(a, b)) = COUNT(DISTINCT a || '|' || b) AS pairs_kept "
                "FROM input WHERE a IS NOT NULL AND b IS NOT NULL"
            ),
            order="ordered",
            tags=("equality",),
        ),
    )
