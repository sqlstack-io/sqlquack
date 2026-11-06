"""Point the shared obligation builder at the PostgreSQL root (see the root README)."""

from __future__ import annotations

import json
import re
import tomllib
from collections.abc import Callable
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
    GRAMMAR_REWRITES,
    PG_CLASS_RULES,
    PROTECTED_TEXT,
)

_shared_grammar_read: Callable[..., list[str]] = obligations._grammar_read  # noqa: SLF001
_shared_class_rules: tuple[tuple[re.Pattern[str], str], ...] = derive.CLASS_RULES


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
    """The hand-written limits of one family's `pg.` forms, as the shared reader builds them."""
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
    """The shared grammar read, a form's bare words upper-cased unless it has a reading."""
    lines: list[str] = _shared_grammar_read(form=form, readings=readings)
    if str(form["id"]) in readings:
        return lines
    return [_rewritten(_upper_bare_words(line)) for line in lines]


def _rewritten(line: str) -> str:
    """The line with each documented-list spelling the renderer cannot copy written out."""
    for pattern, replacement in GRAMMAR_REWRITES:
        line = pattern.sub(replacement, line)
    return line


def _upper_bare_words(line: str) -> str:
    """The line with everything outside placeholders and quoted text in upper case."""
    parts: list[str] = PROTECTED_TEXT.split(line)
    return "".join(part if PROTECTED_TEXT.fullmatch(part) else part.upper() for part in parts)


def install() -> None:
    """Register the Postgres styles, class rules, grammar read and limit reader."""
    derive.CLASS_RULES = (*PG_CLASS_RULES, *_shared_class_rules)
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
