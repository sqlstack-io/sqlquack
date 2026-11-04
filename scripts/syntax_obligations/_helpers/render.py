"""Render an obligation's case SQL from its form's grammar trees and its case spec."""

from __future__ import annotations

import json
from dataclasses import dataclass, field

from scripts.syntax_obligations._helpers.derive import operand_class
from scripts.syntax_obligations._helpers.grammar import is_repetition, placeholder_name, render
from scripts.syntax_obligations.constants import (
    ACTION_ALL_OPTIONS,
    ACTION_CHOICE_BRANCH,
    ACTION_DROP_NODE,
    ACTION_HAND,
    ACTION_NEST_OPERANDS,
    ACTION_QUOTE_IDENTIFIERS,
    ACTION_REPLACE_NODE,
    ACTION_RESERVED_WORD,
    BUILTIN_NAMES,
    CLASS_FILL,
    CLOSE_REPEAT_AFTER,
    CONTEXT_BLOCK,
    CONTEXT_PROCEDURE,
    CONTEXT_RAW,
    CONTEXT_TOP,
    DOUBLE_QUOTE,
    EXPECT_ACCEPT,
    EXPECT_REJECT,
    HAND_ALSO_SQL,
    HAND_CORE,
    HAND_SQL,
    LABEL_SLOT,
    NAME_FILL,
    NODE_CHOICE,
    NODE_OPTIONAL,
    PROCEDURE_WRAPPER,
    QUERY_CLASS,
    RESERVED_WORD,
    SINGLE_CHARACTER_LITERAL_LENGTH,
    STATEMENT_CLASS,
    STATEMENT_TERMINATOR,
    STRING_CLASS,
    TOKEN_ELISION,
    TOKEN_LITERAL,
    TOKEN_PLACEHOLDER,
)
from scripts.syntax_obligations.models import FormStyle, Node, Rendered, Spec


@dataclass
class _Choices:
    """The rendering choices for one case."""

    full: bool = False
    path: frozenset[int] = frozenset()
    branch: dict[int, int] = field(default_factory=dict)
    replace: dict[int, str] = field(default_factory=dict)
    drop: frozenset[int] = frozenset()
    quote: str = ""


def submissions(
    *, trees: list[tuple[Node, ...]], style: FormStyle, spec: Spec, depths: tuple[int, ...]
) -> list[Rendered]:
    """The submissions of a case spec of one form; most specs give one, some give several."""
    return _Renderer(trees=trees, style=style).submissions(spec=spec, depths=depths)


class _Renderer:
    """Renders the cases of one form."""

    def __init__(self, *, trees: list[tuple[Node, ...]], style: FormStyle) -> None:
        self.trees: list[tuple[Node, ...]] = trees
        self.style: FormStyle = style
        self._used: dict[str, list[str]] = {}

    def submissions(self, *, spec: Spec, depths: tuple[int, ...] = ()) -> list[Rendered]:
        """The submissions of a case spec; most specs give one, some give several."""
        if spec.action == ACTION_HAND:
            return self._hand(fields=json.loads(spec.payload), spec=spec, depths=depths)
        if spec.action == ACTION_NEST_OPERANDS:
            return [
                Rendered(
                    self.wrap(
                        self.text(
                            production=spec.production,
                            choices=self._nest_at(spec=spec, depth=depth),
                        )
                    ),
                    spec.expect,
                    spec.stage,
                )
                for depth in depths
            ]
        if spec.action == ACTION_RESERVED_WORD:
            return [
                Rendered(
                    self.wrap(
                        self.text(
                            production=spec.production, choices=self._at(spec=spec, replace=word)
                        )
                    ),
                    expect,
                )
                for word, expect in (
                    (RESERVED_WORD, EXPECT_REJECT),
                    (f'"{RESERVED_WORD}"', EXPECT_ACCEPT),
                )
            ]
        text: str = self.text(production=spec.production, choices=self._choices(spec))
        return [Rendered(self.wrap(text), spec.expect, spec.stage)]

    def wrap(self, text: str) -> str:
        """Prefix, opened cursors, text and suffix (`{label}` filled), after the declarations."""
        labels: list[str] = self._used.get("label", [])
        suffix: str = self.style.suffix.replace(
            LABEL_SLOT, labels[0] if labels else self.style.default_label
        )
        opened: str = "".join(
            f"OPEN {name} ; "
            for name in dict.fromkeys(self._used.get(self.style.opens, []))
            if name != RESERVED_WORD
        )
        whole: str = f"{self.style.prefix}{opened}{self._terminated(text)}{suffix}".strip()
        declarations: str = self._declarations()
        if not declarations:
            return whole
        if whole.startswith("DECLARE "):
            return f"DECLARE {declarations} {whole.removeprefix('DECLARE ')}"
        return f"DECLARE {declarations} {whole}"

    def text(self, *, production: int, choices: _Choices) -> str:
        """The rendered form text of one production; records the names it fills in."""
        self._used = {}
        return " ".join(self._sequence(nodes=self.trees[production], choices=choices))

    def _declarations(self) -> str:
        """Declarations, by `uses`, of the objects the form names, so the case is self-contained."""
        parts: list[str] = []
        seen: set[str] = set()
        for name, template in self.style.uses.items():
            for used in self._used.get(name, []):
                if used in seen or used == RESERVED_WORD or used in BUILTIN_NAMES:
                    continue
                seen.add(used)
                parts.append(f"{template.replace('{}', used)} ;")
        return " ".join(parts)

    def _terminated(self, text: str) -> str:
        if self.style.terminate and not text.rstrip().endswith(STATEMENT_TERMINATOR):
            return f"{text} {STATEMENT_TERMINATOR}"
        return text

    def _choices(self, spec: Spec) -> _Choices:
        choices: _Choices = _Choices(full=spec.action == ACTION_ALL_OPTIONS, path=spec.path)
        choices.branch.update(dict(spec.chosen))
        if spec.action == ACTION_CHOICE_BRANCH:
            index: int = int(spec.payload)
            for twin in self._twins(spec):
                choices.branch.setdefault(twin, index)
            choices.branch[spec.node] = index
            choices.path = spec.path | {spec.node}
        elif spec.action == ACTION_DROP_NODE:
            choices.drop = frozenset({spec.node})
        elif spec.action == ACTION_REPLACE_NODE:
            choices.replace[spec.node] = spec.payload
        elif spec.action == ACTION_QUOTE_IDENTIFIERS:
            choices.quote = spec.payload
        return choices

    @staticmethod
    def _at(*, spec: Spec, replace: str) -> _Choices:
        return _Choices(path=spec.path, branch=dict(spec.chosen), replace={spec.node: replace})

    def _nest_at(self, *, spec: Spec, depth: int) -> _Choices:
        return self._at(spec=spec, replace=self._nest(label=spec.payload, depth=depth))

    def _twins(self, spec: Spec) -> list[int]:
        """Same-width choices of the production, selected together (DO ... END FOR, LOOP ...)."""
        groups: list[Node] = _groups(self.trees[spec.production])
        width: int = next((len(n.branches) for n in groups if id(n) == spec.node), 0)
        return [
            id(n)
            for n in groups
            if n.kind == NODE_CHOICE and len(n.branches) == width and id(n) != spec.node
        ]

    def _hand(
        self, *, fields: dict[str, str], spec: Spec, depths: tuple[int, ...]
    ) -> list[Rendered]:
        expect: str = fields.get("expect", spec.expect)
        stage: str = fields.get("stage", "")
        if HAND_CORE in fields:
            return [
                Rendered(self.nested_hand(fields=fields, depth=depth), expect, stage)
                for depth in depths
            ]
        context: str = fields.get("context", CONTEXT_BLOCK)
        texts: list[tuple[str, str, str]] = []
        if HAND_SQL in fields:
            texts.append((fields[HAND_SQL], expect, stage))
        if HAND_ALSO_SQL in fields:
            texts.append(
                (
                    fields[HAND_ALSO_SQL],
                    fields.get("also_expect", expect),
                    fields.get("also_stage", ""),
                )
            )
        rendered: list[Rendered] = []
        for text, want, later in texts or [("", expect, stage)]:
            self._used = {}
            body: str = text or self.text(production=0, choices=_Choices())
            if context == CONTEXT_RAW:
                sql: str = body
            elif context == CONTEXT_TOP:
                sql = self._terminated(body)
            elif context == CONTEXT_PROCEDURE:
                sql = PROCEDURE_WRAPPER.format(block=self.wrap(body))
            else:
                sql = self.wrap(body)
            rendered.append(Rendered(sql, want, later))
        return rendered

    def nested_hand(self, *, fields: dict[str, str], depth: int) -> str:
        """A hand limit nested `depth` levels: outer_open, open*(depth-1), core, closes."""
        self._used = {}
        inner: int = depth - 1
        parts: list[str] = [
            fields["outer_open"],
            *[fields["open"]] * inner,
            fields[HAND_CORE],
            *[fields["close"]] * inner,
            fields["outer_close"],
        ]
        return self.wrap(" ".join(parts))

    def _nest(self, *, label: str, depth: int) -> str:
        """An operand nested `depth` levels; a statement's innermost is the form's own fill."""
        if label == QUERY_CLASS:
            return "SELECT * FROM ( " * (depth - 1) + "SELECT 1" + " )" * (depth - 1)
        core: str = self.style.fills.get(STATEMENT_CLASS, CLASS_FILL[STATEMENT_CLASS])
        return "BEGIN " * depth + f"{core} ;" + " END ;" * (depth - 1) + " END"

    def _sequence(self, *, nodes: tuple[Node, ...], choices: _Choices) -> list[str]:
        """Rendered tokens; a replaced operand is still declared under its valid name."""
        out: list[str] = []
        for node in nodes:
            key: int = id(node)
            if key in choices.drop:
                continue
            if key in choices.replace:
                text: str = choices.replace[key]
                name: str = placeholder_name(node)
                if text in (RESERVED_WORD, f'"{RESERVED_WORD}"'):
                    self._used.setdefault(name, []).append(text)
                elif name:
                    self._token(node=node, choices=_Choices())
                out.append(text)
            elif node.kind == NODE_OPTIONAL:
                out += self._optional(node=node, choices=choices, before=out)
            elif node.kind == NODE_CHOICE:
                out += self._sequence(
                    nodes=node.branches[choices.branch.get(key, 0)], choices=choices
                )
            elif node.token_kind != TOKEN_ELISION:
                out.append(self._token(node=node, choices=choices))
        return out

    def _optional(self, *, node: Node, choices: _Choices, before: list[str]) -> list[str]:
        key: int = id(node)
        if not (choices.full or key in choices.path):
            return []
        clause: str | None = self.style.clauses.get(render((node,)))
        if clause is not None:
            return [clause]
        branch: tuple[Node, ...] = node.branches[choices.branch.get(key, 0)]
        if is_repetition(branch) and not any(placeholder_name(n) for n in _tokens(branch)):
            start: int = max(
                (i for i, text in enumerate(before) if text in CLOSE_REPEAT_AFTER), default=-1
            )
            return [",", *before[start + 1 :]]
        return self._sequence(nodes=branch, choices=choices)

    def _token(self, *, node: Node, choices: _Choices) -> str:
        name: str = placeholder_name(node)
        if not name:
            quoted: bool = (
                node.token_kind == TOKEN_LITERAL
                and len(node.text) == SINGLE_CHARACTER_LITERAL_LENGTH
            )
            return node.text[1] if quoted and node.text[0] == DOUBLE_QUOTE else node.text
        fill: str = self.style.fills.get(name) or NAME_FILL.get(name) or ""
        label: str = operand_class(name)
        if not fill:
            fill = CLASS_FILL.get(STRING_CLASS if node.token_kind == TOKEN_LITERAL else label, "v1")
        if choices.quote == name and node.token_kind == TOKEN_PLACEHOLDER:
            fill = f'"{fill} Q"'
        self._used.setdefault(name, []).append(fill)
        return fill


def _tokens(nodes: tuple[Node, ...]) -> list[Node]:
    found: list[Node] = []
    for node in nodes:
        if node.branches:
            for branch in node.branches:
                found += _tokens(branch)
        else:
            found.append(node)
    return found


def _groups(nodes: tuple[Node, ...]) -> list[Node]:
    found: list[Node] = []
    for node in nodes:
        if node.branches:
            found.append(node)
            for branch in node.branches:
                found += _groups(branch)
    return found
