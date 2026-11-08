"""Point the shared obligation builder at the DuckDB root (see the root README)."""

from __future__ import annotations

import json
import re
import tomllib
from collections.abc import Callable
from pathlib import Path

from scripts.syntax_obligations import constants as shared
from scripts.syntax_obligations._helpers import derive, grammar, ids, obligations, render
from scripts.syntax_obligations.constants import ACTION_HAND, HAND_CASE_KEYS, PAIR_SIZE
from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.models import Draft, FormStyle, Spec
from scripts.syntax_obligations.types import Key, Row
from scripts.syntax_obligations_duckdb.constants import (
    CALL_NAME,
    DD_CLASS_RULES,
    DD_REPEAT_FILLS,
    DD_TOKEN,
    FAMILIES,
    FAMILY_CONTEXTS,
    FILLS,
    FORM_CONTEXTS,
    FORM_ELISIONS,
    FORM_FILLS,
    FORM_PREFIX,
    GRAMMAR_REWRITES,
    OBLIGATIONS_DIR,
    PROTECTED_TEXT,
    RETIRED_FILE,
    RETIRED_KIND,
)

_shared_grammar_read: Callable[..., list[str]] = obligations._grammar_read  # noqa: SLF001
_shared_existing_ids: Callable[[Path], dict[Key, str]] = ids.existing_ids
_shared_class_rules: tuple[tuple[re.Pattern[str], str], ...] = derive.CLASS_RULES


def style(*, form: str, family: str) -> FormStyle:
    """The submission style of a family prefix or of one form, by its longest context prefix."""
    owners: list[str] = [key for key in FORM_CONTEXTS if form.startswith(key)]
    context: tuple[str, str] = (
        FORM_CONTEXTS[max(owners, key=len)] if owners else FAMILY_CONTEXTS[family]
    )
    prefix, suffix = context
    return FormStyle(
        prefix=prefix,
        suffix=suffix,
        fills={**FILLS, **FORM_FILLS.get(form, {})},
        elisions=FORM_ELISIONS.get(form, {}),
        terminate=False,
    )


def dd_limit_drafts(*, path: Path, family: str, forms: set[str]) -> list[Draft]:
    """The hand-written limits of one family's `dd.` forms, as the shared reader builds them."""
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


def dd_grammar_read(*, form: Row, readings: dict[str, Row]) -> list[str]:
    """The shared grammar read, each called name upper-cased: bare lower case is a placeholder."""
    lines: list[str] = _shared_grammar_read(form=form, readings=readings)
    if str(form["id"]) in readings:
        return lines
    return [_called_names_upper(_rewritten(line)) for line in lines]


def _rewritten(line: str) -> str:
    """The line with each GRAMMAR_REWRITES pattern replaced."""
    for pattern, replacement in GRAMMAR_REWRITES:
        line = pattern.sub(replacement, line)
    return line


def _called_names_upper(line: str) -> str:
    """The line with every word written directly before `(` in upper case."""
    parts: list[str] = PROTECTED_TEXT.split(line)
    return "".join(
        part
        if PROTECTED_TEXT.fullmatch(part)
        else CALL_NAME.sub(lambda match: match.group(1).upper() + " (", part)
        for part in parts
    )


def retired_ids(root: Path) -> set[str]:
    """The obligation ids the root has retired; they are never issued again."""
    path: Path = root / RETIRED_FILE
    if not path.exists():
        return set()
    groups: list[Row] = tomllib.loads(path.read_text()).get("retired", [])
    retired: set[str] = set()
    for group in groups:
        retired.update(str(identifier) for identifier in group["ids"])
    return retired


def dd_existing_ids(path: Path) -> dict[Key, str]:
    """The kept ids of one obligations file, plus the root's retired ids under keys no draft has."""
    kept: dict[Key, str] = _shared_existing_ids(path)
    for identifier in retired_ids(path.parent.parent):
        kept[(identifier.rsplit(".", 1)[0], RETIRED_KIND, identifier, "", "")] = identifier
    return kept


def install() -> None:
    """Register the DuckDB styles, tokens, operand classes, grammar read and limit reader."""
    derive.CLASS_RULES = (*DD_CLASS_RULES, *_shared_class_rules)
    render.REPEAT_FILLS.update(DD_REPEAT_FILLS)
    setattr(obligations, "_grammar_read", dd_grammar_read)  # noqa: B010
    setattr(grammar, "TOKEN", DD_TOKEN)  # noqa: B010
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
    setattr(obligations, "limit_drafts", dd_limit_drafts)  # noqa: B010
    setattr(obligations, "existing_ids", dd_existing_ids)  # noqa: B010


def issued_ids(root: Path) -> set[str]:
    """Every obligation id written under the root's obligations/."""
    issued: set[str] = set()
    for family in FAMILIES:
        path: Path = root / OBLIGATIONS_DIR / f"{family}.toml"
        if not path.exists():
            continue
        rows: list[dict[str, object]] = tomllib.loads(path.read_text()).get("obligation", [])
        issued.update(str(row["id"]) for row in rows)
    return issued
