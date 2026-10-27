"""Case files as TOML text, written the same way every time."""

from __future__ import annotations

from scripts.case_generators.constants import (
    CONTROL_LIMIT,
    DELETE_CHARACTER,
    GENERATED_NOTICE,
    GENERATED_ORIGIN_PREFIX,
    TOML_TAB,
)
from scripts.case_generators.models import Case, CaseFile


def quoted(text: str) -> str:
    """A TOML basic string, control characters escaped."""
    escaped: str = text.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
    return '"' + "".join(map(_visible, escaped)) + '"'


def _visible(character: str) -> str:
    """A control character TOML forbids as an escape; other characters unchanged."""
    control: bool = (ord(character) < CONTROL_LIMIT and character != TOML_TAB) or ord(
        character
    ) == DELETE_CHARACTER
    return f"\\u{ord(character):04x}" if control else character


def string_list(items: tuple[str, ...]) -> str:
    """A one-line TOML array of strings."""
    return "[" + ", ".join(quoted(item) for item in items) + "]"


def column_list(case: Case) -> str:
    """The case's columns as a TOML array of inline tables."""
    return ", ".join(
        f"{{ name = {quoted(column.name)}, type = {quoted(column.data_type)} }}"
        for column in case.columns
    )


def case_text(*, case: Case, generator: str) -> str:
    """One `[[case]]` table."""
    lines: list[str] = [
        "[[case]]",
        f"id = {quoted(case.id)}",
        f"description = {quoted(case.description)}",
        f"columns = [{column_list(case)}]",
        "rows = [",
        *(f"  {string_list(row)}," for row in case.rows),
        "]",
    ]
    if case.expression is not None:
        lines.append(f"expression = {quoted(case.expression)}")
    if case.query is not None:
        lines.append(f"query = {quoted(case.query)}")
    if case.order is not None:
        lines.append(f"order = {quoted(case.order)}")
    lines.append(f"tags = {string_list(case.tags)}")
    lines.append(f"origin = {quoted(GENERATED_ORIGIN_PREFIX + generator)}")
    if case.expected_divergence is not None:
        lines.append(f"expected_divergence = {quoted(case.expected_divergence)}")
    return "\n".join(lines) + "\n"


def file_text(case_file: CaseFile) -> str:
    """A whole case file."""
    head: list[str] = [GENERATED_NOTICE, *(f"# {line}" for line in case_file.header), ""]
    head += [
        f"category = {quoted(case_file.category)}",
        f"construct = {quoted(case_file.construct)}",
        f"session_profile = {quoted(case_file.session_profile)}",
    ]
    body: list[str] = [
        case_text(case=case, generator=case_file.generator) for case in case_file.cases
    ]
    return "\n".join(head) + "\n\n" + "\n".join(body)
