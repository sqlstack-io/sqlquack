"""Single realistic mistakes in valid statements, one mistake per mutant."""

from __future__ import annotations

import re
from collections.abc import Callable, Iterable

from scripts.referee_cases._helpers.data import environments
from scripts.referee_cases.constants import (
    ALL_GROUP_KEY,
    ARGUMENT_SEPARATOR,
    BOOLEAN_TYPE,
    CALL_OPEN,
    GENERATED_ENVIRONMENT,
    JOIN_KEYWORD,
    MISSPELLING_SUFFIX,
    MUTANT_KIND,
    ORDERS_COLUMN_REPLACEMENTS,
    PLACEHOLDER_ARGUMENT,
    QUOTE_CHARACTERS,
    SHARED_COLUMNS,
    SHORTEST_MISSPELLING,
    TEMPORAL_PREFIXES,
    TYPE_COLUMN_PREFIX,
    TYPE_COLUMN_REPLACEMENTS,
    VALID_KIND,
)
from scripts.referee_cases.models import Environment, Statement

_TOKEN: re.Pattern[str] = re.compile(r"'(?:[^']|'')*'|\"[^\"]*\"|[A-Za-z_][A-Za-z0-9_$]*")
_CALL: re.Pattern[str] = re.compile(r"\b([A-Z][A-Z0-9_]*)\(([^()]*)\)")
_GROUP_KEYS: re.Pattern[str] = re.compile(r" GROUP BY ([a-z_.]+(?:, [a-z_.]+)+)")
_GROUP_KEY: re.Pattern[str] = re.compile(r" GROUP BY ([a-z_.]+)\b(?! \()")
_ALIAS: re.Pattern[str] = re.compile(r"\bAS ([a-z_][a-z0-9_]*)\b")
_QUALIFIED: re.Pattern[str] = re.compile(r"\b[a-z]\.([a-z_]+)\b")
_SINGLE_ORDERS: re.Pattern[str] = re.compile(r" FROM orders(?: WHERE| GROUP| ORDER| QUALIFY|$)")


def mutant_statements(bases: Iterable[Statement]) -> tuple[Statement, ...]:
    """Every mutation that applies to each valid base, without repeating any SQL."""
    environment: Environment = environments()[GENERATED_ENVIRONMENT]
    types: dict[str, str] = _column_types(environment)
    names: frozenset[str] = frozenset(types) | {table.name for table in environment.tables}
    mutations: tuple[tuple[str, Callable[[str], str | None]], ...] = (
        ("misspelt_column", lambda sql: _misspelt_column(sql=sql, types=types, names=names)),
        ("misspelt_alias", lambda sql: _misspelt_alias(sql=sql, names=names)),
        ("dropped_argument", _dropped_argument),
        ("extra_argument", _extra_argument),
        ("wrong_type", lambda sql: _wrong_type(sql=sql, types=types)),
        ("missing_group_by", _missing_group_by),
        ("ambiguous_name", _ambiguous_name),
    )
    valid: list[Statement] = [base for base in bases if base.kind == VALID_KIND]
    seen: set[str] = {base.sql for base in valid}
    mutants: list[Statement] = []
    for base in valid:
        for mutation, mutate in mutations:
            sql: str | None = mutate(base.sql)
            if sql is None or sql in seen:
                continue
            seen.add(sql)
            mutants.append(
                Statement(
                    name=f"{base.name}/{mutation}",
                    sql=sql,
                    kind=MUTANT_KIND,
                    mutation=mutation,
                    base=base.name,
                )
            )
    return tuple(mutants)


def misspelt(*, name: str, names: frozenset[str]) -> str:
    """`name` without its middle character, or with one more when short or naming another."""
    middle: int = len(name) // 2
    candidate: str = name[:middle] + name[middle + 1 :]
    if candidate in names or len(candidate) < SHORTEST_MISSPELLING:
        return name + MISSPELLING_SUFFIX
    return candidate


def _column_types(environment: Environment) -> dict[str, str]:
    types: dict[str, str] = {}
    for table in environment.tables:
        types.update((column.name, column.data_type) for column in table.columns)
    return types


def _identifiers(sql: str) -> list[re.Match[str]]:
    return [match for match in _TOKEN.finditer(sql) if match.group(0)[0] not in QUOTE_CHARACTERS]


def _misspelt_column(*, sql: str, types: dict[str, str], names: frozenset[str]) -> str | None:
    for match in _identifiers(sql):
        following: str = sql[match.end() : match.end() + 1]
        if match.group(0) in types and following != CALL_OPEN:
            replacement: str = misspelt(name=match.group(0), names=names)
            return sql[: match.start()] + replacement + sql[match.end() :]
    return None


def _misspelt_alias(*, sql: str, names: frozenset[str]) -> str | None:
    for definition in _ALIAS.finditer(sql):
        alias: str = definition.group(1)
        for match in _identifiers(sql):
            if match.start() > definition.end() and match.group(0) == alias:
                replacement: str = misspelt(name=alias, names=names)
                return sql[: match.start()] + replacement + sql[match.end() :]
    return None


def _dropped_argument(sql: str) -> str | None:
    call: re.Match[str] | None = _CALL.search(sql)
    if call is None or not call.group(2).strip():
        return None
    arguments: list[str] = call.group(2).split(ARGUMENT_SEPARATOR)
    return f"{sql[: call.start(2)]}{ARGUMENT_SEPARATOR.join(arguments[:-1])}{sql[call.end(2) :]}"


def _extra_argument(sql: str) -> str | None:
    call: re.Match[str] | None = _CALL.search(sql)
    if call is None:
        return None
    arguments: list[str] = [*filter(None, [call.group(2).strip()]), PLACEHOLDER_ARGUMENT]
    return f"{sql[: call.start(2)]}{ARGUMENT_SEPARATOR.join(arguments)}{sql[call.end(2) :]}"


def _wrong_type(*, sql: str, types: dict[str, str]) -> str | None:
    """A column argument of the first call replaced by one of another family in its table."""
    call: re.Match[str] | None = _CALL.search(sql)
    if call is None:
        return None
    for match in _identifiers(call.group(2)):
        column: str = match.group(0)
        data_type: str | None = types.get(column)
        typed_column: bool = column.startswith(TYPE_COLUMN_PREFIX)
        if data_type is None or not (typed_column or _SINGLE_ORDERS.search(sql)):
            continue
        replacements: tuple[str, str] = (
            TYPE_COLUMN_REPLACEMENTS if typed_column else ORDERS_COLUMN_REPLACEMENTS
        )
        temporal: bool = data_type.startswith(TEMPORAL_PREFIXES) or data_type == BOOLEAN_TYPE
        start: int = call.start(2) + match.start()
        return sql[:start] + replacements[temporal] + sql[call.start(2) + match.end() :]
    return None


def _missing_group_by(sql: str) -> str | None:
    keys: re.Match[str] | None = _GROUP_KEYS.search(sql)
    if keys is not None:
        kept: str = ARGUMENT_SEPARATOR.join(keys.group(1).split(ARGUMENT_SEPARATOR)[:-1])
        return f"{sql[: keys.start(1)]}{kept}{sql[keys.end(1) :]}"
    single: re.Match[str] | None = _GROUP_KEY.search(sql)
    if single is None or single.group(1) == ALL_GROUP_KEY:
        return None
    return sql[: single.start()] + sql[single.end() :]


def _ambiguous_name(sql: str) -> str | None:
    if JOIN_KEYWORD not in sql:
        return None
    for match in _QUALIFIED.finditer(sql):
        if match.group(1) in SHARED_COLUMNS:
            return sql[: match.start()] + match.group(1) + sql[match.end() :]
    return None
