"""NUMBER sample values and the arithmetic result types sqlquack-semantics types Snowflake with."""

from __future__ import annotations

from decimal import ROUND_HALF_UP, Decimal, localcontext

from scripts.case_generators.constants import (
    ADDITIVE_OPERATORS,
    DIVISION_EXTRA_SCALE,
    FLOAT,
    FLOAT_LIMIT,
    FLOAT_PLAIN_LIMIT,
    FLOAT_SAMPLES,
    LARGE_SAMPLE,
    MAX_PRECISION,
    NULL_CELL,
    NUMBER_SAMPLE_NEGATIVE,
    NUMBER_SAMPLE_SMALL,
    SCALE_CAP,
    UNSCALED_BITS,
    WORKING_PRECISION,
)
from scripts.case_generators.models import NumberType
from scripts.case_generators.types import Operator


def largest(number_type: NumberType) -> Decimal:
    """The largest value of a NUMBER type: all nines."""
    with localcontext() as context:
        context.prec = WORKING_PRECISION
        return Decimal(10) ** (number_type.precision - number_type.scale) - Decimal(10) ** (
            -number_type.scale
        )


def fit(*, value: Decimal, number_type: NumberType) -> Decimal:
    """`value` rounded half up to the type's scale."""
    if number_type.is_float:
        return value
    with localcontext() as context:
        context.prec = WORKING_PRECISION
        return value.quantize(Decimal(10) ** -number_type.scale, rounding=ROUND_HALF_UP)


def samples(number_type: NumberType) -> dict[str, Decimal]:
    """Named sample values of a type: zero, small, negative, fractional and the largest."""
    if number_type.is_float:
        return dict(FLOAT_SAMPLES)
    values: dict[str, Decimal] = {
        "zero": Decimal(0),
        "small": fit(value=NUMBER_SAMPLE_SMALL, number_type=number_type),
        "negative": fit(value=NUMBER_SAMPLE_NEGATIVE, number_type=number_type),
        LARGE_SAMPLE: largest(number_type),
    }
    if number_type.scale > 0:
        values["fraction"] = Decimal(10) ** -number_type.scale
    return values


def literal(*, value: Decimal | None, number_type: NumberType) -> str:
    """A case cell: the value as a Snowflake literal, or NULL."""
    if value is None:
        return NULL_CELL
    if number_type.is_float:
        return format(value.normalize(), "f") if abs(value) < FLOAT_PLAIN_LIMIT else f"{value:E}"
    return format(fit(value=value, number_type=number_type), "f")


def capped(*, leading: int, scale: int) -> NumberType:
    """A NUMBER of `leading` integer digits and `scale`, capped at 38 digits."""
    return NumberType(min(leading + scale, MAX_PRECISION), min(scale, MAX_PRECISION))


def result_type(*, operator: Operator, left: NumberType, right: NumberType) -> NumberType:
    """The result type of `left <operator> right`."""
    if left.is_float or right.is_float:
        return FLOAT
    left_leading: int = left.precision - left.scale
    right_leading: int = right.precision - right.scale
    if operator in ADDITIVE_OPERATORS:
        return capped(
            leading=max(left_leading, right_leading) + 1, scale=max(left.scale, right.scale)
        )
    if operator == Operator.MULTIPLY:
        ceiling: int = max(left.scale, right.scale, SCALE_CAP)
        return capped(
            leading=left_leading + right_leading, scale=min(left.scale + right.scale, ceiling)
        )
    scale: int = max(left.scale, min(left.scale + DIVISION_EXTRA_SCALE, SCALE_CAP))
    return capped(leading=left_leading + right.scale, scale=scale)


def division_guarded(*, left: NumberType, right: NumberType, divisor: Decimal) -> bool:
    """Whether exact division raises `numeric_range`, as duckdb.exact_decimal_divide guards it."""
    if left.is_float or right.is_float:
        return False
    result: NumberType = result_type(operator=Operator.DIVIDE, left=left, right=right)
    shift: int = result.scale + right.scale - min(left.scale, result.scale + right.scale)
    if right.precision + shift <= MAX_PRECISION:
        return False
    with localcontext() as context:
        context.prec = WORKING_PRECISION
        unscaled: Decimal = divisor.copy_abs() * Decimal(10) ** right.scale
        return unscaled >= Decimal(10) ** (MAX_PRECISION - shift)


def common_digits(*, value: Decimal, left: NumberType, right: NumberType) -> bool:
    """Whether `value` fits the common type comparison coerces both operands to."""
    if left.is_float or right.is_float:
        return True
    scale: int = max(left.scale, right.scale)
    with localcontext() as context:
        context.prec = WORKING_PRECISION
        return value.copy_abs() < Decimal(10) ** (MAX_PRECISION - scale)


def within_128_bits(*, value: Decimal, number_type: NumberType) -> bool:
    """Whether `value` at the type's scale fits the signed 128-bit integer Snowflake keeps."""
    with localcontext() as context:
        context.prec = WORKING_PRECISION
        unscaled: Decimal = fit(value=value, number_type=number_type).scaleb(number_type.scale)
        return unscaled.copy_abs() < Decimal(2) ** UNSCALED_BITS


def exact(*, operator: Operator, left: Decimal, right: Decimal) -> Decimal:
    """The exact result, to 100 significant digits."""
    with localcontext() as context:
        context.prec = WORKING_PRECISION
        if operator == Operator.ADD:
            return left + right
        if operator == Operator.SUBTRACT:
            return left - right
        if operator == Operator.MULTIPLY:
            return left * right
        return left / right


def fits(*, value: Decimal, number_type: NumberType) -> bool:
    """Whether `value`, rounded to the type's scale, fits its integer digits."""
    if number_type.is_float:
        return abs(value) < FLOAT_LIMIT
    with localcontext() as context:
        context.prec = WORKING_PRECISION
        rounded: Decimal = fit(value=value, number_type=number_type)
        limit: Decimal = Decimal(10) ** (number_type.precision - number_type.scale)
        return rounded.copy_abs() < limit
