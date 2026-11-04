"""One unit test's outcome from its conversion and, when it converted, its DuckDB run."""

from __future__ import annotations

import re

from scripts.duckshop_measure.constants import (
    ACTUAL_COUNT,
    EXPECTED_COUNT,
    GUARD_RAISE,
    MISSING_COUNT,
    UNEXPECTED_COUNT,
)
from scripts.duckshop_measure.models import Conversion, UnitTestOutcome
from scripts.duckshop_measure.types import Executor, Outcome


def failing_step(step: tuple[object, ...]) -> bool:
    """Whether a step has unexpected or missing rows, or another row count than expected."""
    return (
        step[UNEXPECTED_COUNT] != 0
        or step[MISSING_COUNT] != 0
        or step[ACTUAL_COUNT] != step[EXPECTED_COUNT]
    )


def step_text(step: tuple[object, ...]) -> str:
    """A step's values, comma-separated."""
    return ", ".join(str(value) for value in step)


def unit_test_outcome(*, test: str, conversion: Conversion, execute: Executor) -> UnitTestOutcome:
    """Refused, needing Python, or run: matching, differing, stopped by a guard, or erring."""
    if not conversion.converted:
        return UnitTestOutcome(test=test, outcome=Outcome.REFUSED, details=conversion.refusals)
    if conversion.python_functions:
        return UnitTestOutcome(test=test, outcome=Outcome.NEEDS_PYTHON)
    try:
        steps: list[tuple[object, ...]] = execute(conversion.sql)
    except Exception as error:  # noqa: BLE001 - any DuckDB failure is the test's outcome
        message: str = f"{type(error).__name__}: {error}"
        guard: re.Match[str] | None = GUARD_RAISE.search(message)
        if guard is not None:
            return UnitTestOutcome(test=test, outcome=Outcome.GUARD_RAISED, details=(guard[1],))
        return UnitTestOutcome(test=test, outcome=Outcome.DUCKDB_ERROR, details=(message,))
    failing: tuple[str, ...] = tuple(step_text(step) for step in steps if failing_step(step))
    outcome: Outcome = Outcome.DIFFERS if failing else Outcome.MATCHES
    return UnitTestOutcome(test=test, outcome=outcome, details=failing)
