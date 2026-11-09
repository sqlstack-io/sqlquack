"""Function calls at every argument count up to one past the maximum, and with swept types."""

from __future__ import annotations

from scripts.referee_cases._helpers.data import argument_texts, functions, sweeps
from scripts.referee_cases._helpers.unique import unique_statements
from scripts.referee_cases.constants import (
    ARGUMENT_SEPARATOR,
    ARITY_KIND,
    PLACEHOLDER_ARGUMENT,
    TYPES_KIND,
    VALID_KIND,
    WINDOW_SHAPE,
    WINDOW_SUFFIX,
)
from scripts.referee_cases.models import Function, Statement


def function_statements() -> tuple[Statement, ...]:
    """Each function's canonical call, its argument-count sweep and its type sweeps."""
    texts: dict[str, str] = argument_texts()
    first, second = sweeps()
    statements: list[Statement] = []
    for function in functions():
        canonical: tuple[str, ...] = tuple(texts[kind] for kind in function.arguments)
        statements.extend(
            unique_statements(
                _candidates(function=function, canonical=canonical, sweeps=(first, second))
            )
        )
    return tuple(statements)


def _candidates(
    *, function: Function, canonical: tuple[str, ...], sweeps: tuple[tuple[str, ...], ...]
) -> list[Statement]:
    prefix: str = function.name.lower()
    candidates: list[Statement] = [
        _statement(
            function=function, name=f"{prefix}/canonical", arguments=canonical, kind=VALID_KIND
        )
    ]
    for count in range(_largest_count(function=function, canonical=canonical) + 1):
        candidates.append(
            _statement(
                function=function,
                name=f"{prefix}/arity_{count}",
                arguments=_arguments(canonical=canonical, count=count),
                kind=ARITY_KIND,
            )
        )
    for position, sweep in enumerate(sweeps):
        for column in sweep[: len(sweep) * (position < len(canonical))]:
            candidates.append(
                _statement(
                    function=function,
                    name=f"{prefix}/argument_{position + 1}_{column}",
                    arguments=canonical[:position] + (column,) + canonical[position + 1 :],
                    kind=TYPES_KIND,
                )
            )
    return candidates


def _statement(
    *, function: Function, name: str, arguments: tuple[str, ...], kind: str
) -> Statement:
    suffix: str = WINDOW_SUFFIX if function.shape == WINDOW_SHAPE else ""
    call: str = f"{function.name}({ARGUMENT_SEPARATOR.join(arguments)}){suffix}"
    return Statement(name=name, sql=f"SELECT {call} AS r FROM all_types", kind=kind)


def _largest_count(*, function: Function, canonical: tuple[str, ...]) -> int:
    """One past the maximum, or two past the canonical count for a variadic function."""
    if function.maximum is None:
        return len(canonical) + 2
    return function.maximum + 1


def _arguments(*, canonical: tuple[str, ...], count: int) -> tuple[str, ...]:
    """The first `count` canonical arguments, repeating the last one past the end."""
    filler: str = canonical[-1] if canonical else PLACEHOLDER_ARGUMENT
    return canonical[:count] + (filler,) * max(count - len(canonical), 0)
