"""Render an obligation's case SQL from its form's grammar trees and its case spec."""

from __future__ import annotations

import json
from dataclasses import dataclass, field

from scripts.syntax_obligations._helpers.derive import operand_class
from scripts.syntax_obligations._helpers.grammar import is_repetition, placeholder_name, render
from scripts.syntax_obligations.constants import (
    CLASS_FILL,
    CLOSE_REPEAT_AFTER,
    NAME_FILL,
    NODE_CHOICE,
    NODE_OPTIONAL,
    PROCEDURE_WRAPPER,
    RESERVED_WORD,
    STATEMENT_TERMINATOR,
    TOKEN_ELISION,
    TOKEN_LITERAL,
    TOKEN_PLACEHOLDER,
)
from scripts.syntax_obligations.models import Node, Spec


@dataclass(frozen=True)
class FormStyle:
    """How a family renders one form: fills that override the defaults, clause texts for
    `[ KEYWORD ... ]` repeats, and how the rendered form text becomes a complete submission."""

    prefix: str
    suffix: str
    fills: dict[str, str] = field(default_factory=dict)
    clauses: dict[str, str] = field(default_factory=dict)
    terminate: bool = True


@dataclass
class _Choices:
    """The rendering choices for one case."""

    full: bool = False
    path: frozenset[int] = frozenset()
    branch: dict[int, int] = field(default_factory=dict)
    replace: dict[int, str] = field(default_factory=dict)
    drop: frozenset[int] = frozenset()
    quote: bool = False


class Renderer:
    """Renders the cases of one form."""

    def __init__(self, *, trees: list[tuple[Node, ...]], style: FormStyle) -> None:
        self.trees: list[tuple[Node, ...]] = trees
        self.style: FormStyle = style

    def submissions(self, spec: Spec, depths: tuple[int, ...] = ()) -> list[str]:
        """The complete SQL texts of a case spec; most specs give one, some give several."""
        if spec.action == "hand":
            fields: dict[str, str] = json.loads(spec.payload)
            if "core" in fields:
                return [self.nested_hand(fields, depth) for depth in depths]
            return self._hand(fields)
        if spec.action == "nest":
            return [self.wrap(text) for text in self._nested(spec, depths)]
        if spec.action == "reserved":
            return [
                self.wrap(self.text(spec.production, self._at(spec, replace=word)))
                for word in (RESERVED_WORD, f'"{RESERVED_WORD}"')
            ]
        return [self.wrap(self.text(spec.production, self._choices(spec)))]

    def wrap(self, text: str) -> str:
        """The form text as a complete submission in the family's context."""
        return f"{self.style.prefix}{self._terminated(text)}{self.style.suffix}".strip()

    def text(self, production: int, choices: _Choices) -> str:
        """The rendered form text of one production."""
        return " ".join(self._sequence(self.trees[production], choices))

    def _terminated(self, text: str) -> str:
        if self.style.terminate and not text.rstrip().endswith(STATEMENT_TERMINATOR):
            return f"{text} {STATEMENT_TERMINATOR}"
        return text

    def _choices(self, spec: Spec) -> _Choices:
        choices: _Choices = _Choices(full=spec.action == "max", path=spec.path)
        choices.branch.update(dict(spec.chosen))
        if spec.action == "branch":
            index: int = int(spec.payload)
            for twin in self._twins(spec):
                choices.branch.setdefault(twin, index)
            choices.branch[spec.node] = index
            choices.path = spec.path | {spec.node}
        elif spec.action == "drop":
            choices.drop = frozenset({spec.node})
        elif spec.action == "replace":
            choices.replace[spec.node] = spec.payload
        elif spec.action == "quote":
            choices.quote = True
        return choices

    @staticmethod
    def _at(spec: Spec, *, replace: str) -> _Choices:
        return _Choices(path=spec.path, branch=dict(spec.chosen), replace={spec.node: replace})

    def _twins(self, spec: Spec) -> list[int]:
        """Choices of the same production with as many branches as the chosen one, which a
        documented pairing (DO ... END FOR, LOOP ... END LOOP) selects together."""
        widths: dict[int, int] = {
            id(n): len(n.branches) for n in _groups(self.trees[spec.production])
        }
        width: int = widths.get(spec.node, 0)
        return [
            id(n)
            for n in _groups(self.trees[spec.production])
            if n.kind == NODE_CHOICE and len(n.branches) == width and id(n) != spec.node
        ]

    def _hand(self, fields: dict[str, str]) -> list[str]:
        context: str = fields.get("context", "block")
        texts: list[str] = [fields[key] for key in ("sql", "also_sql") if key in fields]
        if not texts:
            texts = [self.text(0, _Choices())]
        if context == "raw":
            return texts
        if context == "top":
            return [self._terminated(text) for text in texts]
        if context == "procedure":
            return [PROCEDURE_WRAPPER.format(block=self.wrap(text)) for text in texts]
        return [self.wrap(text) for text in texts]

    def nested_hand(self, fields: dict[str, str], depth: int) -> str:
        """A hand limit nested `depth` levels: outer_open, open*(depth-1), core, closes."""
        inner: int = depth - 1
        parts: list[str] = [
            fields["outer_open"],
            *[fields["open"]] * inner,
            fields["core"],
            *[fields["close"]] * inner,
            fields["outer_close"],
        ]
        return self.wrap(" ".join(parts))

    def _nested(self, spec: Spec, depths: tuple[int, ...]) -> list[str]:
        node: Node | None = self._first_of_class(spec.payload)
        if node is None:
            return []
        return [
            self.text(0, _Choices(replace={id(node): self._nest(spec.payload, depth)}))
            for depth in depths
        ]

    def _nest(self, label: str, depth: int) -> str:
        """An operand nested `depth` levels; the innermost statement is the form's own fill,
        so a loop's BREAK still ends it."""
        if label == "query":
            return "SELECT * FROM ( " * (depth - 1) + "SELECT 1" + " )" * (depth - 1)
        core: str = self.style.fills.get("statement", CLASS_FILL["statement"])
        return "BEGIN " * depth + f"{core} ;" + " END ;" * (depth - 1) + " END"

    def _first_of_class(self, label: str) -> Node | None:
        for node in _tokens(self.trees[0]):
            name: str = placeholder_name(node)
            if name and operand_class(name) == label:
                return node
        return None

    def _sequence(self, nodes: tuple[Node, ...], choices: _Choices) -> list[str]:
        out: list[str] = []
        for node in nodes:
            key: int = id(node)
            if key in choices.drop:
                continue
            if key in choices.replace:
                out.append(choices.replace[key])
            elif node.kind == NODE_OPTIONAL:
                out += self._optional(node, choices, out)
            elif node.kind == NODE_CHOICE:
                out += self._sequence(node.branches[choices.branch.get(key, 0)], choices)
            elif node.token_kind != TOKEN_ELISION:
                out.append(self._token(node, choices))
        return out

    def _optional(self, node: Node, choices: _Choices, before: list[str]) -> list[str]:
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
        return self._sequence(branch, choices)

    def _token(self, node: Node, choices: _Choices) -> str:
        name: str = placeholder_name(node)
        if not name:
            quoted: bool = node.token_kind == TOKEN_LITERAL and len(node.text) == 3
            return node.text[1] if quoted and node.text[0] == '"' else node.text
        fill: str = self.style.fills.get(name) or NAME_FILL.get(name) or ""
        label: str = operand_class(name)
        if not fill:
            fill = CLASS_FILL.get(
                "string_literal" if node.token_kind == TOKEN_LITERAL else label, "v1"
            )
        if choices.quote and label == "identifier" and node.token_kind == TOKEN_PLACEHOLDER:
            return f'"{fill} Q"'
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
