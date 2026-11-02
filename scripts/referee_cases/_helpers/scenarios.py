"""Hand-written scenarios: name resolution, and the structured and VECTOR types."""

from __future__ import annotations

from scripts.referee_cases._helpers.data import scenarios
from scripts.referee_cases.constants import NAMES_DATA, SCENARIO_KIND, STRUCTURED_DATA, VALID_KIND
from scripts.referee_cases.models import Statement


def name_statements() -> tuple[Statement, ...]:
    """Lateral aliases, shadowing, quoting and case, ambiguity, USING, QUALIFY and GROUP BY."""
    return _statements(NAMES_DATA)


def structured_statements() -> tuple[Statement, ...]:
    """Structured ARRAY, OBJECT and MAP columns and VECTOR columns."""
    return _statements(STRUCTURED_DATA)


def _statements(name: str) -> tuple[Statement, ...]:
    kinds: dict[bool, str] = {True: VALID_KIND, False: SCENARIO_KIND}
    return tuple(
        Statement(name=label, sql=sql, kind=kinds[valid]) for label, sql, valid in scenarios(name)
    )
