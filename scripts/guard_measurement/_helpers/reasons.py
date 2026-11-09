"""Raise messages read as reasons, and values redacted to shapes."""

from __future__ import annotations

import re

from scripts.guard_measurement.constants import (
    DECIMAL_POINT,
    DOUBLE_DIGITS,
    EXPONENT_MARKER,
    GUARD_RAISE,
    JSON_NUMBER,
    JSON_STRING,
    JSON_WORD,
    LONG_INTEGER_DIGITS,
    NUMBER_FIDELITY_REASON,
    QUOTED_VALUE,
    SHAPE_LENGTH,
    SIX_ZEROS,
    SNOWFLAKE_RAISE,
    TRAILING_COMMA,
    ZERO_DIGITS,
)
from scripts.guard_measurement.types import Origin


def classify(*, message: str) -> tuple[str, str]:
    """The origin and reason of a raise: a guard, a Snowflake error, or DuckDB's own error."""
    snowflake: re.Match[str] | None = SNOWFLAKE_RAISE.search(message)
    if snowflake is not None:
        return Origin.SNOWFLAKE_ERROR, f"Snowflake error {snowflake.group(1)}: {snowflake.group(2)}"
    guard: re.Match[str] | None = GUARD_RAISE.search(message)
    if guard is not None:
        return Origin.DIVERGENCE, f"{guard.group(1)}: {guard.group(2)}"
    first_line: str = (message.strip().splitlines() or [""])[0]
    return Origin.DUCKDB_ERROR, shape(value=QUOTED_VALUE.sub("'…'", first_line))


def number_detail(*, reason: str, values: tuple[str | None, ...]) -> tuple[str, str | None]:
    """For the number-fidelity raise, the reason narrowed by the first offending token found."""
    if NUMBER_FIDELITY_REASON not in reason:
        return reason, None
    for value in values:
        found: tuple[str, str | None] | None = _offending(text=value or "")
        if found is not None:
            return f"{reason} ({found[0]})", found[1]
    return reason, None


def shape(*, value: str) -> str:
    """`value` with digits as 9 and letters as a, cut to a short prefix."""
    redacted: str = re.sub(r"[A-Za-z]", "a", re.sub(r"[0-9]", "9", value))
    return redacted if len(redacted) <= SHAPE_LENGTH else f"{redacted[:SHAPE_LENGTH]}…"


def _offending(*, text: str) -> tuple[str, str | None] | None:
    """The first problem in JSON text outside strings, with the offending token's shape."""
    bare: str = JSON_STRING.sub('""', text)
    for token in JSON_NUMBER.findall(bare):
        detail: str | None = _number_problem(token=token)
        if detail is not None:
            return detail, shape(value=token)
    word: re.Match[str] | None = JSON_WORD.search(bare)
    if word is not None:
        return "bare word", shape(value=word.group(0))
    return ("trailing comma", None) if TRAILING_COMMA.search(bare) is not None else None


def _number_problem(*, token: str) -> str | None:
    """Why DuckDB may not keep this JSON number as Snowflake does, or None."""
    if EXPONENT_MARKER.search(token) is not None:
        return "exponent"
    mantissa: str = token.lstrip("-")
    digits: str = mantissa.replace(DECIMAL_POINT, "").lstrip("0")
    if DECIMAL_POINT not in mantissa:
        return f"{len(digits)}-digit integer" if len(digits) >= LONG_INTEGER_DIGITS else None
    fraction: str = mantissa.split(DECIMAL_POINT)[1]
    problems: list[tuple[bool, str]] = [
        (set(fraction) <= ZERO_DIGITS, "fraction of zeros"),
        (len(digits) > DOUBLE_DIGITS, f"{len(digits)} significant digits"),
        (SIX_ZEROS in fraction, "six zeros in a fraction"),
    ]
    return next((problem for applies, problem in problems if applies), None)
