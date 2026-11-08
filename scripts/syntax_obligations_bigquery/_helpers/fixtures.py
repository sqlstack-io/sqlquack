"""The declared fixture set: which fixtures a case text names, recorded as the case's setup."""

from __future__ import annotations

from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations_bigquery.constants import ABSENT_OBJECTS, FIXTURES, NAMED_OBJECT
from scripts.syntax_obligations_bigquery.models import Fixture


def case_setup(sql: str) -> list[str]:
    """The declared fixtures a case text names, with those they need, in FIXTURES order."""
    named: set[str] = set(NAMED_OBJECT.findall(sql))
    known: dict[str, Fixture] = {fixture.name: fixture for fixture in FIXTURES}
    unknown: list[str] = sorted(named - set(known) - set(ABSENT_OBJECTS))
    if unknown:
        raise ObligationError(
            f"a case names objects with no fixture and no reason: {unknown}: {sql}"
        )
    wanted: set[str] = named & set(known)
    for fixture in reversed(FIXTURES):
        if fixture.name in wanted:
            wanted.update(fixture.needs)
    return [fixture.ddl for fixture in FIXTURES if fixture.name in wanted]
