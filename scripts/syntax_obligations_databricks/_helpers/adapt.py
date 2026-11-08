"""Point the shared obligation builder at the Databricks root (see the root README)."""

from __future__ import annotations

import dataclasses
import json
import re
import tomllib
from collections.abc import Callable
from pathlib import Path

from scripts.syntax_obligations import constants as shared
from scripts.syntax_obligations._helpers import derive, grammar, ids, obligations, render
from scripts.syntax_obligations.constants import (
    ACTION_HAND,
    ACTION_RESERVED_WORD,
    EXPECT_ACCEPT,
    HAND_CASE_KEYS,
    IDENTIFIER_RESERVED,
    PAIR_SIZE,
    RESERVED_WORD,
)
from scripts.syntax_obligations.exceptions import ObligationError
from scripts.syntax_obligations.models import Draft, FormStyle, Node, Rendered, Spec
from scripts.syntax_obligations.types import Key, Row
from scripts.syntax_obligations_databricks.constants import (
    ALIAS_RESERVED_NOTE,
    ALIAS_RESERVED_WORD,
    DB_CLASS_RULES,
    DB_REPEAT_FILLS,
    DB_TOKEN,
    FAMILIES,
    FAMILY_CONTEXTS,
    FILLS,
    FORM_CONTEXTS,
    FORM_ELISIONS,
    FORM_FILLS,
    FORM_PREFIX,
    IDENTIFIER_QUOTE,
    KEYWORD_NOTE,
    RETIRED_FILE,
    RETIRED_KIND,
    TABLE_ALIAS_PLACEHOLDERS,
)

_shared_existing_ids: Callable[[Path], dict[Key, str]] = ids.existing_ids
_shared_token: Callable[..., str] = render._Renderer._token  # noqa: SLF001
_shared_submissions: Callable[..., list[Rendered]] = render._Renderer.submissions  # noqa: SLF001
_shared_spec: Callable[..., Spec] = derive._Walk.spec  # noqa: SLF001
_shared_class_note: Callable[..., str] = derive._class_note  # noqa: SLF001
_shared_class_rules: tuple[tuple[re.Pattern[str], str], ...] = derive.CLASS_RULES


def _longest(*, form: str, keys: set[str]) -> str:
    """The longest key that is a prefix of the form, or the empty string."""
    owners: list[str] = [key for key in keys if form.startswith(key)]
    return max(owners, key=len) if owners else ""


def style(*, form: str, family: str) -> FormStyle:
    """The submission style of a family prefix or of one form, by its longest matching keys."""
    context_key: str = _longest(form=form, keys=set(FORM_CONTEXTS))
    context: tuple[str, str] = (
        FORM_CONTEXTS[context_key] if context_key else FAMILY_CONTEXTS[family]
    )
    prefix, suffix = context
    fills_key: str = _longest(form=form, keys=set(FORM_FILLS))
    return FormStyle(
        prefix=prefix,
        suffix=suffix,
        fills={**FILLS, **FORM_FILLS.get(fills_key, {})},
        elisions=FORM_ELISIONS.get(form, {}),
        terminate=False,
    )


def db_limit_drafts(*, path: Path, family: str, forms: set[str]) -> list[Draft]:
    """The hand-written limits of one family's `db.` forms, as the shared reader builds them."""
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


def db_existing_ids(path: Path) -> dict[Key, str]:
    """The kept ids of one obligations file, plus the root's retired ids under keys no draft has."""
    kept: dict[Key, str] = _shared_existing_ids(path)
    for identifier in retired_ids(path.parent.parent):
        kept[(identifier.rsplit(".", 1)[0], RETIRED_KIND, identifier, "", "")] = identifier
    return kept


def backticked(text: str) -> str:
    """A double-quoted identifier as Databricks quotes identifiers, in backticks."""
    if len(text) > 1 and text.startswith('"') and text.endswith('"'):
        return f"{IDENTIFIER_QUOTE}{text[1:-1]}{IDENTIFIER_QUOTE}"
    return text


def db_token(self: object, *, node: object, choices: object, dollar_quoted: bool = False) -> str:
    """The shared token, with a quoted-identifier fill in backticks (double quotes are strings)."""
    text: str = _shared_token(self, node=node, choices=choices, dollar_quoted=dollar_quoted)
    quoted: str = str(getattr(choices, "quote", ""))
    if quoted and quoted == render.placeholder_name(node):  # type: ignore[arg-type]
        return backticked(text)
    return text


def db_spec(self: object, *, action: str, node: Node | None = None, payload: str = "") -> Spec:
    """The shared spec; a reserved-word spec carries its placeholder, which picks the word."""
    if action == ACTION_RESERVED_WORD and node is not None and not payload:
        payload = node.text
    return _shared_spec(self, action=action, node=node, payload=payload)


def db_class_note(*, label: str, description: str, placeholder: str) -> str:
    """The shared identifier class note; the reserved class read from the reserved-words page."""
    if label != IDENTIFIER_RESERVED:
        return _shared_class_note(label=label, description=description, placeholder=placeholder)
    return ALIAS_RESERVED_NOTE if placeholder in TABLE_ALIAS_PLACEHOLDERS else KEYWORD_NOTE


def db_submissions(self: object, *, spec: Spec, depths: tuple[int, ...] = ()) -> list[Rendered]:
    """The shared submissions; the reserved word is ANTI for a table alias, else an accepted
    SELECT, and its quoted twin is written in backticks."""
    if spec.action != ACTION_RESERVED_WORD:
        return _shared_submissions(self, spec=spec, depths=depths)
    alias: bool = spec.payload in TABLE_ALIAS_PLACEHOLDERS
    word: str = ALIAS_RESERVED_WORD if alias else RESERVED_WORD
    render.RESERVED_WORD = word
    try:
        found: list[Rendered] = _shared_submissions(self, spec=spec, depths=depths)
    finally:
        render.RESERVED_WORD = RESERVED_WORD
    quoted: str = f'"{word}"'
    return [
        dataclasses.replace(
            item,
            sql=item.sql.replace(quoted, backticked(quoted)),
            expect=item.expect if alias else EXPECT_ACCEPT,
        )
        for item in found
    ]


def install() -> None:
    """Register the Databricks styles, tokens, operand classes and limit reader."""
    derive.CLASS_RULES = (*DB_CLASS_RULES, *_shared_class_rules)
    render.REPEAT_FILLS.update(DB_REPEAT_FILLS)
    setattr(grammar, "TOKEN", DB_TOKEN)  # noqa: B010
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
    setattr(obligations, "limit_drafts", db_limit_drafts)  # noqa: B010
    setattr(obligations, "existing_ids", db_existing_ids)  # noqa: B010
    setattr(render._Renderer, "_token", db_token)  # noqa: B010, SLF001
    setattr(render._Renderer, "submissions", db_submissions)  # noqa: B010, SLF001
    setattr(derive._Walk, "spec", db_spec)  # noqa: B010, SLF001
    setattr(derive, "_class_note", db_class_note)  # noqa: B010
