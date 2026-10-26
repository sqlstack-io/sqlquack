"""Grids over NUMBER and FLOAT operand pairs: arithmetic and comparison."""

from __future__ import annotations

from decimal import Decimal
from itertools import product

from scripts.case_generators._helpers.numbers import (
    common_digits,
    division_guarded,
    exact,
    fit,
    fits,
    literal,
    result_type,
    samples,
    within_128_bits,
)
from scripts.case_generators.constants import (
    ADDITIVE_OPERATORS,
    ARITHMETIC_TYPES,
    COMPARISON_QUERY,
    FUNCTIONS,
    GRID_TYPES,
    LARGE_SAMPLE,
    MAX_PRECISION,
    MIXED_TYPES,
    NULL_CELL,
    NULL_POSITIONS,
    NUMERIC_ARITHMETIC,
    NUMERIC_COMPARISON,
    NUMERIC_RANGE,
    OPERATOR_FILES,
    RANGE_EDGE_FACTOR,
    RANGE_EDGE_LEADING,
    SAME_TYPES,
    SAMPLE_PAIRS,
)
from scripts.case_generators.models import (
    Case,
    CaseFile,
    Column,
    Function,
    NumberType,
    OperatorFile,
    Variant,
)
from scripts.case_generators.types import Operator, Pair, Row


def sample_pairs(*, left: NumberType, right: NumberType) -> list[Pair]:
    """Distinct operand pairs from the two types' samples, in a fixed order."""
    left_values: dict[str, Decimal] = samples(left)
    right_values: dict[str, Decimal] = samples(right)
    pairs: list[Pair] = []
    for names in SAMPLE_PAIRS:
        if (names.left is not None and names.left not in left_values) or (
            names.right is not None and names.right not in right_values
        ):
            continue
        pair: Pair = (
            None if names.left is None else left_values[names.left],
            None if names.right is None else right_values[names.right],
        )
        if pair not in pairs:
            pairs.append(pair)
    return pairs


def arithmetic_rows(
    *, operator: Operator, left: NumberType, right: NumberType
) -> tuple[list[Pair], list[Pair], list[Pair]]:
    """Pairs inside the result type, beyond it, and dividing by `numeric_range` divisors."""
    target: NumberType = result_type(operator=operator, left=left, right=right)
    inside: list[Pair] = []
    beyond: list[Pair] = []
    guarded: list[Pair] = []
    for a, b in sample_pairs(left=left, right=right):
        if operator == Operator.DIVIDE and b is not None and b.is_zero():
            continue
        if (
            operator == Operator.DIVIDE
            and a is not None
            and b is not None
            and division_guarded(left=left, right=right, divisor=b)
        ):
            guarded.append((a, b))
        elif (
            a is not None
            and b is not None
            and not fits(value=exact(operator=operator, left=a, right=b), number_type=target)
        ):
            beyond.append((a, b))
        else:
            inside.append((a, b))
    return inside, beyond, guarded


def present_values(rows: list[Pair]) -> list[Decimal]:
    """Every non-NULL operand of the rows."""
    values: list[Decimal] = []
    for row in rows:
        values.extend(value for value in row if value is not None)
    return values


def tags_for(*, rows: list[Pair], left: NumberType, right: NumberType) -> tuple[str, ...]:
    """The edge-case inputs the rows cover."""
    values: list[Decimal] = present_values(rows)
    tags: list[str] = []
    if any(None in row for row in rows):
        tags.append("null")
    if any(value.is_zero() for value in values):
        tags.append("zero")
    if any(value.is_signed() and not value.is_zero() for value in values):
        tags.append("negative")
    larges: set[Decimal] = {samples(left)[LARGE_SAMPLE], samples(right)[LARGE_SAMPLE]}
    if any(value.copy_abs() in larges for value in values):
        tags.append("boundary")
    return tuple(tags)


def cells(*, rows: list[Pair], left: NumberType, right: NumberType) -> tuple[Row, ...]:
    """The rows as case cells."""
    return tuple(
        (literal(value=a, number_type=left), literal(value=b, number_type=right)) for a, b in rows
    )


def held_beyond_type(
    *, operator: Operator, left: NumberType, right: NumberType, rows: list[Pair]
) -> tuple[list[Pair], list[Pair]]:
    """Rows beyond the result type Snowflake still holds in 128 bits, and the rest."""
    target: NumberType = result_type(operator=operator, left=left, right=right)
    held: list[Pair] = [
        (a, b)
        for a, b in rows
        if a is not None
        and b is not None
        and within_128_bits(value=exact(operator=operator, left=a, right=b), number_type=target)
    ]
    return held, [row for row in rows if row not in held]


def multiply_edge_rows(*, left: NumberType, right: NumberType) -> list[Pair]:
    """Products between 10^38 and 2^127 at the result scale, where both operand types allow them."""
    if left.is_float or right.is_float:
        return []
    target: NumberType = result_type(operator=Operator.MULTIPLY, left=left, right=right)
    operand: Decimal = RANGE_EDGE_LEADING.scaleb(MAX_PRECISION - 1 - target.scale)
    rows: list[Pair] = [(operand, RANGE_EDGE_FACTOR), (-operand, RANGE_EDGE_FACTOR)]
    if not fits(value=operand, number_type=left):
        return []
    product: Decimal = exact(operator=Operator.MULTIPLY, left=operand, right=RANGE_EDGE_FACTOR)
    if fits(value=product, number_type=target):
        return []
    return rows


def range_cases(
    *, entry: OperatorFile, left: NumberType, right: NumberType, rows: list[Pair]
) -> list[Case]:
    """Results beyond the type that Snowflake holds in 128 bits: `numeric_range` for `+` and `-`."""
    operator: Operator = entry.operator
    if operator == Operator.MULTIPLY:
        rows = [
            *rows,
            *(row for row in multiply_edge_rows(left=left, right=right) if row not in rows),
        ]
    if not rows:
        return []
    columns: tuple[Column, ...] = (Column("a", left.name), Column("b", right.name))
    typed: str = result_type(operator=operator, left=left, right=right).name
    if operator in ADDITIVE_OPERATORS:
        return [
            Case(
                id=f"{left.slug}_{entry.slug}_{right.slug}_numeric_range",
                description=(
                    f"{left.name} {operator} {right.name} beyond {typed} but within 128 bits, "
                    "which DuckDB's DECIMAL(38) cannot hold (divergence numeric_range)."
                ),
                columns=columns,
                rows=cells(rows=rows, left=left, right=right),
                expression=f"a {operator} b",
                tags=("boundary",),
                expected_divergence=NUMERIC_RANGE,
            )
        ]
    return [
        Case(
            id=f"{left.slug}_{entry.slug}_{right.slug}_range_edge",
            description=(
                f"{left.name} {operator} {right.name} beyond {typed} but within 128 bits, "
                "which DuckDB's DECIMAL(38) cannot hold (divergence numeric_range)."
            ),
            columns=columns,
            rows=cells(rows=rows, left=left, right=right),
            expression=f"a {operator} b",
            tags=("boundary",),
            expected_divergence=NUMERIC_RANGE,
        )
    ]


def pair_cases(*, entry: OperatorFile, left: NumberType, right: NumberType) -> list[Case]:
    """The case for one operand pair, and its rows beyond the result type, if any."""
    operator: Operator = entry.operator
    inside, all_beyond, guarded = arithmetic_rows(operator=operator, left=left, right=right)
    held: list[Pair] = []
    beyond: list[Pair] = all_beyond
    if operator != Operator.DIVIDE:
        held, beyond = held_beyond_type(operator=operator, left=left, right=right, rows=all_beyond)
    typed: str = result_type(operator=operator, left=left, right=right).name
    columns: tuple[Column, ...] = (Column("a", left.name), Column("b", right.name))
    cases: list[Case] = [
        Case(
            id=f"{left.slug}_{entry.slug}_{right.slug}",
            description=(
                f"{left.name} {operator} {right.name}, typed {typed}, over zero, "
                "negative, fractional, NULL and the largest values that fit."
            ),
            columns=columns,
            rows=cells(rows=inside, left=left, right=right),
            expression=f"a {operator} b",
            tags=tags_for(rows=inside, left=left, right=right),
        )
    ]
    if beyond:
        cases.append(
            Case(
                id=f"{left.slug}_{entry.slug}_{right.slug}_beyond_type",
                description=(
                    f"{left.name} {operator} {right.name} whose exact result exceeds "
                    f"{typed}: raises or converts."
                ),
                columns=columns,
                rows=cells(rows=beyond, left=left, right=right),
                expression=f"a {operator} b",
                tags=("boundary",),
            )
        )
    if guarded:
        cases.append(
            Case(
                id=f"{left.slug}_{entry.slug}_{right.slug}_numeric_range",
                description=(
                    f"{left.name} {operator} {right.name} by divisors whose exact quotient "
                    "needs more than 128 bits in DuckDB (divergence numeric_range)."
                ),
                columns=columns,
                rows=cells(rows=guarded, left=left, right=right),
                expression=f"a {operator} b",
                tags=("boundary",),
                expected_divergence=NUMERIC_RANGE,
            )
        )
    if operator != Operator.DIVIDE:
        cases.extend(range_cases(entry=entry, left=left, right=right, rows=held))
    return cases


def arithmetic_files() -> list[CaseFile]:
    """One file per operator, with a case per operand type pair."""
    files: list[CaseFile] = []
    for entry in OPERATOR_FILES:
        cases: list[Case] = []
        for left in ARITHMETIC_TYPES:
            for right in ARITHMETIC_TYPES:
                cases.extend(pair_cases(entry=entry, left=left, right=right))
        files.append(
            CaseFile(
                generator=NUMERIC_ARITHMETIC,
                name=entry.slug,
                category="numeric",
                construct=entry.construct,
                cases=tuple(cases),
                header=(f"Every NUMBER and FLOAT operand pair for `{entry.operator}`.",),
            )
        )
    return files


def comparison_rows(*, left: NumberType, right: NumberType) -> list[Pair]:
    """Ties across scales, NULLs, zeros, negatives and the largest values."""
    left_values: dict[str, Decimal] = samples(left)
    right_values: dict[str, Decimal] = samples(right)
    small: Decimal = left_values["small"]
    candidates: list[Pair] = [
        (small, fit(value=small, number_type=right)),
        (left_values["zero"], right_values["zero"]),
        (left_values["negative"], right_values["small"]),
        (left_values[LARGE_SAMPLE], right_values[LARGE_SAMPLE]),
        (left_values["negative"], right_values["negative"]),
        (None, right_values["small"]),
        (left_values["small"], None),
        (None, None),
    ]
    rows: list[Pair] = []
    for pair in candidates:
        if pair not in rows:
            rows.append(pair)
    return rows


def coerces(*, row: Pair, left: NumberType, right: NumberType) -> bool:
    """Whether both operands of a row fit the common type of the comparison."""
    return all(
        common_digits(value=value, left=left, right=right) for value in row if value is not None
    )


def comparison_cases(*, left: NumberType, right: NumberType) -> list[Case]:
    """The six comparisons of one operand type pair, and of rows beyond their common type."""
    tags: list[str] = ["null", "ties"]
    if left != right:
        tags.append("mixed-types")
    rows: list[Pair] = comparison_rows(left=left, right=right)
    held: list[Pair] = [row for row in rows if coerces(row=row, left=left, right=right)]
    beyond: list[Pair] = [row for row in rows if row not in held]
    columns: tuple[Column, ...] = (Column("a", left.name), Column("b", right.name))
    cases: list[Case] = [
        Case(
            id=f"{left.slug}_vs_{right.slug}",
            description=f"The six comparisons of {left.name} with {right.name}.",
            columns=columns,
            rows=cells(rows=held, left=left, right=right),
            query=COMPARISON_QUERY,
            order="ordered",
            tags=tuple(tags),
        )
    ]
    if beyond:
        cases.append(
            Case(
                id=f"{left.slug}_vs_{right.slug}_beyond_common_type",
                description=(
                    f"Comparing {left.name} with {right.name} for values beyond their common "
                    "type: raises or converts."
                ),
                columns=columns,
                rows=cells(rows=beyond, left=left, right=right),
                query=COMPARISON_QUERY,
                order="ordered",
                tags=("boundary",),
            )
        )
    return cases


def comparison_files() -> list[CaseFile]:
    """One case per operand type pair, all six comparison operators at once."""
    cases: list[Case] = []
    for left in GRID_TYPES:
        for right in GRID_TYPES:
            cases.extend(comparison_cases(left=left, right=right))
    return [
        CaseFile(
            generator=NUMERIC_COMPARISON,
            name="comparisons",
            category="comparison",
            construct="node:eq",
            cases=tuple(cases),
            header=("Every NUMBER and FLOAT operand pair under the six comparison operators.",),
        )
    ]


def masked(*, values: tuple[str, ...], mask: tuple[bool, ...]) -> Row:
    """The values with NULL where the mask is set."""
    return tuple(
        NULL_CELL if is_null else value for is_null, value in zip(mask, values, strict=True)
    )


def patterns(values: tuple[str, ...]) -> list[Row]:
    """Every combination of NULL and the given value in each position, all-NULL last."""
    return [masked(values=values, mask=mask) for mask in product((False, True), repeat=len(values))]


def variant_case(*, function: Function, variant: Variant) -> Case:
    """The function over every NULL pattern of one argument variant."""
    rows: list[Row] = patterns(variant.values)
    if variant.slug == SAME_TYPES:
        rows += list(function.extra_rows)
    tags: list[str] = ["null", "all-null", *function.extra_tags]
    if variant.slug == MIXED_TYPES:
        tags.append("mixed-types")
    return Case(
        id=f"{function.slug}_{variant.slug}",
        description=(
            f"{function.expression} with NULL in every combination of positions, "
            f"over {variant.slug.replace('_', ' ')}."
        ),
        columns=variant.columns,
        rows=tuple(rows),
        expression=function.expression,
        tags=tuple(tags),
    )


def function_file(function: Function) -> CaseFile:
    """One function's file, one case per argument variant."""
    return CaseFile(
        generator=NULL_POSITIONS,
        name=function.slug,
        category=function.category,
        construct=function.construct,
        cases=tuple(
            variant_case(function=function, variant=variant) for variant in function.variants
        ),
        header=(f"NULL in every argument position of {function.expression}.",),
    )


def null_position_files() -> list[CaseFile]:
    """One file per function."""
    return [function_file(function) for function in FUNCTIONS]
