"""Point the shared obligation builder at the Databricks root (see the root README)."""

from __future__ import annotations

import dataclasses
import json
import re
import tomllib
from collections import Counter
from collections.abc import Callable
from pathlib import Path

from scripts.grammar_obligations import constants as shared
from scripts.grammar_obligations._helpers import derive, grammar, ids, obligations
from scripts.grammar_obligations.classes import renderer, walk
from scripts.grammar_obligations.classes.renderer import Renderer
from scripts.grammar_obligations.classes.walk import Walk
from scripts.grammar_obligations.constants import (
    ACTION_DROP_NODE,
    ACTION_HAND,
    ACTION_RESERVED_WORD,
    CLOSE_PAREN,
    EXPECT_ACCEPT,
    HAND_CASE_KEYS,
    IDENTIFIER_RESERVED,
    KIND_NEGATIVE,
    NODE_OPTIONAL,
    OPEN_PAREN,
    PAIR_SIZE,
    RESERVED_WORD,
)
from scripts.grammar_obligations.exceptions import ObligationError
from scripts.grammar_obligations.models import Choices, Draft, FormStyle, Node, Rendered, Spec
from scripts.grammar_obligations.types import Key, Row
from scripts.syntax_obligations_databricks.constants import (
    ALIAS_RESERVED_NOTE,
    ALIAS_RESERVED_WORD,
    BARE_NAME_PREFIXES,
    BARE_OPERAND_FILL,
    DB_CLASS_RULES,
    DB_REPEAT_FILLS,
    DB_TOKEN,
    FAMILIES,
    FAMILY_CONTEXTS,
    FILLS,
    FORM_CLASSES,
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
_shared_token: Callable[..., str] = Renderer._token  # noqa: SLF001
_shared_submissions: Callable[..., list[Rendered]] = Renderer.submissions
_shared_spec: Callable[..., Spec] = Walk.spec
_shared_walk_token: Callable[..., None] = Walk.token
_shared_class_note: Callable[..., str] = derive._class_note  # noqa: SLF001
_shared_class_rules: tuple[tuple[re.Pattern[str], str], ...] = grammar.CLASS_RULES
_shared_choices: Callable[..., Choices] = Renderer._choices  # noqa: SLF001
_shared_derive_trees: Callable[..., tuple[list[Draft], list[tuple[Node, ...]], Counter[str]]] = (
    derive.derive_trees
)


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


def db_token(self: object, *, node: Node, choices: object, dollar_quoted: bool = False) -> str:
    """The shared token, with a quoted-identifier fill in backticks (double quotes are strings)."""
    text: str = _shared_token(self, node=node, choices=choices, dollar_quoted=dollar_quoted)
    quoted: str = str(getattr(choices, "quote", ""))
    if quoted and quoted == grammar.placeholder_name(node):
        return backticked(text)
    return text


def db_spec(self: object, *, action: str, node: Node | None = None, payload: str = "") -> Spec:
    """The shared spec; a reserved-word spec carries its placeholder, which picks the word."""
    if action == ACTION_RESERVED_WORD and node is not None and not payload:
        payload = node.text
    return _shared_spec(self, action=action, node=node, payload=payload)


def db_walk_token(self: Walk, *, node: Node, where: str, omission_is_absence: bool) -> None:
    """The shared operand drafts, with the classes the form's page gives its operands."""
    overrides: dict[str, str] = FORM_CLASSES.get(
        _longest(form=self.form, keys=set(FORM_CLASSES)), {}
    )
    shared: Callable[[str], str] = walk.operand_class
    setattr(walk, "operand_class", lambda name: overrides.get(name) or shared(name))  # noqa: B010
    try:
        _shared_walk_token(self, node=node, where=where, omission_is_absence=omission_is_absence)
    finally:
        setattr(walk, "operand_class", shared)  # noqa: B010


def db_class_note(*, label: str, description: str, placeholder: str) -> str:
    """The shared identifier class note; the reserved class read from the reserved-words page."""
    if label != IDENTIFIER_RESERVED:
        return _shared_class_note(label=label, description=description, placeholder=placeholder)
    return ALIAS_RESERVED_NOTE if placeholder in TABLE_ALIAS_PLACEHOLDERS else KEYWORD_NOTE


def db_submissions(self: object, *, spec: Spec, depths: tuple[int, ...] = ()) -> list[Rendered]:
    """The shared submissions, with the reserved word and its backticked twin (see the README)."""
    if spec.action != ACTION_RESERVED_WORD:
        return _shared_submissions(self, spec=spec, depths=depths)
    alias: bool = spec.payload in TABLE_ALIAS_PLACEHOLDERS
    word: str = ALIAS_RESERVED_WORD if alias else RESERVED_WORD
    renderer.RESERVED_WORD = word
    try:
        found: list[Rendered] = _shared_submissions(self, spec=spec, depths=depths)
    finally:
        renderer.RESERVED_WORD = RESERVED_WORD
    quoted: str = f'"{word}"'
    return [
        dataclasses.replace(
            item,
            sql=item.sql.replace(quoted, backticked(quoted)),
            expect=item.expect if alias else EXPECT_ACCEPT,
        )
        for item in found
    ]


def _siblings(*, nodes: tuple[Node, ...], key: int) -> tuple[tuple[Node, ...], int] | None:
    """The sequence holding the node whose `id()` is `key`, and its position there."""
    for position, node in enumerate(nodes):
        if id(node) == key:
            return nodes, position
        for branch in node.branches:
            if (found := _siblings(nodes=branch, key=key)) is not None:
                return found
    return None


def _call_parts(*, trees: list[tuple[Node, ...]], spec: Spec) -> tuple[Node, ...] | None:
    """The nodes between a dropped `(` and its matching `)`, then that `)`; None for other drops."""
    if spec.action != ACTION_DROP_NODE or spec.production >= len(trees):
        return None
    found: tuple[tuple[Node, ...], int] | None = _siblings(
        nodes=trees[spec.production], key=spec.node
    )
    if found is None or found[0][found[1]].text != OPEN_PAREN:
        return None
    nodes, start = found
    depth: int = 0
    for position in range(start, len(nodes)):
        depth += {OPEN_PAREN: 1, CLOSE_PAREN: -1}.get(nodes[position].text, 0)
        if depth == 0:
            return nodes[start + 1 : position + 1]
    return None


def db_choices(self: Renderer, spec: Spec) -> Choices:
    """The shared choices; a dropped `(` drops its `)`, and a marked call's operand is a number."""
    choices: Choices = _shared_choices(self, spec)
    parts: tuple[Node, ...] | None = _call_parts(trees=self.trees, spec=spec)
    if not parts:
        return choices
    choices = dataclasses.replace(choices, drop=choices.drop | {id(parts[-1])})
    if spec.payload != BARE_OPERAND_FILL:
        return choices
    if (operand := _first_operand(nodes=parts[:-1], choices=choices)) is not None:
        choices.replace[id(operand)] = BARE_OPERAND_FILL
    return choices


def _first_operand(*, nodes: tuple[Node, ...], choices: Choices) -> Node | None:
    """The first operand the case writes: absent options skipped, chosen branches entered."""
    for node in nodes:
        if node.kind == NODE_OPTIONAL and id(node) not in choices.path and not choices.full:
            continue
        if node.branches:
            branch: tuple[Node, ...] = node.branches[choices.branch.get(id(node), 0)]
            return _first_operand(nodes=branch, choices=choices)
        return node if grammar.placeholder_name(node) else None
    return None


def db_derive_trees(
    *, form: str, grammar: list[str], unpacked: bool = False
) -> tuple[list[Draft], list[tuple[Node, ...]], Counter[str]]:
    """Shared drafts less a missing `(` around no required part where a bare name is valid."""
    drafts, trees, classes = _shared_derive_trees(form=form, grammar=grammar, unpacked=unpacked)
    kept: list[Draft] = []
    for draft in drafts:
        parts: tuple[Node, ...] | None = (
            _call_parts(trees=trees, spec=draft.spec)
            if draft.kind == KIND_NEGATIVE and draft.spec is not None
            else None
        )
        if parts is None or not form.startswith(BARE_NAME_PREFIXES) or draft.spec is None:
            kept.append(draft)
        elif any(node.kind != NODE_OPTIONAL for node in parts[:-1]):
            spec: Spec = dataclasses.replace(draft.spec, payload=BARE_OPERAND_FILL)
            kept.append(dataclasses.replace(draft, spec=spec))
    return kept, trees, classes


def install() -> None:
    """Register the Databricks styles, tokens, operand classes and limit reader."""
    grammar.CLASS_RULES = (*DB_CLASS_RULES, *_shared_class_rules)
    renderer.REPEAT_FILLS.update(DB_REPEAT_FILLS)
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
    setattr(Renderer, "_token", db_token)  # noqa: B010
    setattr(Renderer, "submissions", db_submissions)  # noqa: B010
    setattr(Walk, "spec", db_spec)  # noqa: B010
    setattr(Walk, "token", db_walk_token)  # noqa: B010
    setattr(derive, "_class_note", db_class_note)  # noqa: B010
    setattr(Renderer, "_choices", db_choices)  # noqa: B010
    setattr(obligations, "derive_trees", db_derive_trees)  # noqa: B010
