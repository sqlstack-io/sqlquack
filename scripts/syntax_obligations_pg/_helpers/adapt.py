"""Point the shared obligation builder at the PostgreSQL root.

The shared builder in scripts/syntax_obligations keys its submission styles and hand-written
limits by the Snowflake form prefix. `install` adds a style for every Postgres family and form
context, and reads [[limit]] rows of `pg.` forms, so the shared derivation, numbering and
rendering run unchanged on the Postgres catalogue.
"""

from __future__ import annotations

import json
import re
import tomllib
from pathlib import Path

from scripts.syntax_obligations import constants as shared
from scripts.syntax_obligations._helpers import derive, obligations
from scripts.syntax_obligations.constants import ACTION_HAND, HAND_CASE_KEYS, PAIR_SIZE
from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.models import Draft, FormStyle, Spec
from scripts.syntax_obligations.types import Row
from scripts.syntax_obligations_pg.constants import (
    FAMILIES,
    FAMILY_CONTEXTS,
    FILLS,
    FORM_CONTEXTS,
    FORM_ELISIONS,
    FORM_FILLS,
    FORM_PREFIX,
    PG_CLASS_RULES,
)

SHARED_GRAMMAR_READ = obligations._grammar_read  # noqa: SLF001
SHARED_CLASS_RULES = derive.CLASS_RULES
PROTECTED: re.Pattern[str] = re.compile(r"(<[^<>]*>|'[^']*'|\"[^\"]*\")")


def style(*, form: str, family: str) -> FormStyle:
    """The submission style of a family prefix or of one form."""
    prefix, suffix = FORM_CONTEXTS.get(form, FAMILY_CONTEXTS[family])
    return FormStyle(
        prefix=prefix,
        suffix=suffix,
        fills={**FILLS, **FORM_FILLS.get(form, {})},
        elisions=FORM_ELISIONS.get(form, {}),
        terminate=False,
    )


def pg_limit_drafts(*, path: Path, family: str, forms: set[str]) -> list[Draft]:
    """The hand-written limits of one family's `pg.` forms, in file order, as the shared
    reader builds them for `sf.` forms."""
    if not path.exists():
        return []
    drafts: list[Draft] = []
    for row in tomllib.loads(path.read_text()).get("limit", []):
        form: str = str(row["form"])
        if not form.startswith(f"{FORM_PREFIX}.{family}."):
            continue
        if form not in forms:
            raise ObligationError(f"limit on {form}, which is not a catalogued form")
        pair: list[str] = [str(member) for member in row.get("pair", [])]
        drafts.append(
            Draft(
                form=form,
                kind=str(row["kind"]),
                target=str(row.get("target", "")),
                note=str(row["note"]),
                anchor=str(row.get("anchor", "")),
                depths=tuple(int(depth) for depth in row.get("depths", [])),
                pair=(pair[0], pair[1]) if len(pair) == PAIR_SIZE else (),
                required=bool(row.get("required", True)),
                spec=Spec(
                    ACTION_HAND,
                    payload=json.dumps(
                        {key: row[key] for key in HAND_CASE_KEYS if key in row}, sort_keys=True
                    ),
                    expect=obligations._limit_expect(row),  # noqa: SLF001
                ),
            )
        )
    return drafts


def pg_grammar_read(*, form: Row, readings: dict[str, Row]) -> list[str]:
    """The shared grammar read, with the bare words of a `pg.` form without a reading in upper
    case: Postgres synopses write keywords and type names in lower case and every placeholder
    in angle brackets, while the shared deriver reads a lower-case word as a placeholder."""
    lines: list[str] = SHARED_GRAMMAR_READ(form=form, readings=readings)
    if str(form["id"]) in readings:
        return lines
    return [
        "".join(
            part if PROTECTED.fullmatch(part) else part.upper() for part in PROTECTED.split(line)
        )
        for line in lines
    ]


def install() -> None:
    """Register the Postgres styles, class rules, grammar read and limit reader with the
    shared builder."""
    derive.CLASS_RULES = (*PG_CLASS_RULES, *SHARED_CLASS_RULES)
    setattr(obligations, "_grammar_read", pg_grammar_read)  # noqa: B010
    for family in FAMILIES:
        styles: dict[str, FormStyle] = shared.FAMILY_STYLES.setdefault(family, {})
        styles[f"{FORM_PREFIX}.{family}."] = style(form="", family=family)
        styles.update(
            {
                form: style(form=form, family=family)
                for form in {*FORM_CONTEXTS, *FORM_FILLS, *FORM_ELISIONS}
                if form.startswith(f"{FORM_PREFIX}.{family}.")
            }
        )
    setattr(obligations, "limit_drafts", pg_limit_drafts)  # noqa: B010
