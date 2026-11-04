"""Render an obligation's case SQL from its form's grammar trees and its case spec."""

from __future__ import annotations

import json
from dataclasses import dataclass, field

from scripts.syntax_obligations._helpers.derive import operand_class
from scripts.syntax_obligations._helpers.grammar import is_repetition, placeholder_name, render
from scripts.syntax_obligations.constants import (
    BUILTIN_NAMES,
    CLASS_FILL,
    CLOSE_REPEAT_AFTER,
    EXPECT_ACCEPT,
    EXPECT_REJECT,
    LABEL_SLOT,
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
    """How a family renders one form.

    fills override the default placeholder fills; clauses replace `[ KEYWORD ... ]` repeats;
    prefix and suffix make the form text a complete submission (a suffix may hold `{label}`,
    the label the form used, else `default_label`); uses maps a placeholder that names an
    existing object to the declaration that creates it, so the case is self-contained; opens
    names the cursor placeholder whose cursors the block opens before the form runs.
    """

    prefix: str
    suffix: str
    fills: dict[str, str] = field(default_factory=dict)
    clauses: dict[str, str] = field(default_factory=dict)
    uses: dict[str, str] = field(default_factory=dict)
    opens: str = ""
    default_label: str = "l0"
    terminate: bool = True


@dataclass(frozen=True)
class Rendered:
    """One submission and its expected outcome: accept (no syntax error) or reject (a syntax
    error); `stage` names the later error an accepted case is expected to reach, if any."""

    sql: str
    expect: str
    stage: str = ""


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
        self._used: dict[str, list[str]] = {}

    def submissions(self, spec: Spec, depths: tuple[int, ...] = ()) -> list[Rendered]:
        """The submissions of a case spec; most specs give one, some give several."""
        if spec.action == "hand":
            return self._hand(json.loads(spec.payload), spec, depths)
        if spec.action == "nest":
            return [
                Rendered(
                    self.wrap(self.text(spec.production, self._nest_at(spec, depth))),
                    spec.expect,
                    spec.stage,
                )
                for depth in depths
            ]
        if spec.action == "reserved":
            return [
                Rendered(self.wrap(self.text(spec.production, self._at(spec, word))), expect)
                for word, expect in (
                    (RESERVED_WORD, EXPECT_REJECT),
                    (f'"{RESERVED_WORD}"', EXPECT_ACCEPT),
                )
            ]
        text: str = self.text(spec.production, self._choices(spec))
        return [Rendered(self.wrap(text), spec.expect, spec.stage)]

    def wrap(self, text: str) -> str:
        """The form text as a complete submission, declaring the objects it names."""
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

    def text(self, production: int, choices: _Choices) -> str:
        """The rendered form text of one production; records the names it fills in."""
        self._used = {}
        return " ".join(self._sequence(self.trees[production], choices))

    def _declarations(self) -> str:
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
    def _at(spec: Spec, replace: str) -> _Choices:
        return _Choices(path=spec.path, branch=dict(spec.chosen), replace={spec.node: replace})

    def _nest_at(self, spec: Spec, depth: int) -> _Choices:
        return self._at(spec, self._nest(spec.payload, depth))

    def _twins(self, spec: Spec) -> list[int]:
        """Choices of the same production with as many branches as the chosen one, which a
        documented pairing (DO ... END FOR, LOOP ... END LOOP) selects together."""
        groups: list[Node] = _groups(self.trees[spec.production])
        width: int = next((len(n.branches) for n in groups if id(n) == spec.node), 0)
        return [
            id(n)
            for n in groups
            if n.kind == NODE_CHOICE and len(n.branches) == width and id(n) != spec.node
        ]

    def _hand(self, fields: dict[str, str], spec: Spec, depths: tuple[int, ...]) -> list[Rendered]:
        expect: str = fields.get("expect", spec.expect)
        stage: str = fields.get("stage", "")
        if "core" in fields:
            return [Rendered(self.nested_hand(fields, depth), expect, stage) for depth in depths]
        context: str = fields.get("context", "block")
        texts: list[tuple[str, str, str]] = []
        if "sql" in fields:
            texts.append((fields["sql"], expect, stage))
        if "also_sql" in fields:
            texts.append(
                (
                    fields["also_sql"],
                    fields.get("also_expect", expect),
                    fields.get("also_stage", ""),
                )
            )
        rendered: list[Rendered] = []
        for text, want, later in texts or [("", expect, stage)]:
            self._used = {}
            body: str = text or self.text(0, _Choices())
            if context == "raw":
                sql: str = body
            elif context == "top":
                sql = self._terminated(body)
            elif context == "procedure":
                sql = PROCEDURE_WRAPPER.format(block=self.wrap(body))
            else:
                sql = self.wrap(body)
            rendered.append(Rendered(sql, want, later))
        return rendered

    def nested_hand(self, fields: dict[str, str], depth: int) -> str:
        """A hand limit nested `depth` levels: outer_open, open*(depth-1), core, closes."""
        self._used = {}
        inner: int = depth - 1
        parts: list[str] = [
            fields["outer_open"],
            *[fields["open"]] * inner,
            fields["core"],
            *[fields["close"]] * inner,
            fields["outer_close"],
        ]
        return self.wrap(" ".join(parts))

    def _nest(self, label: str, depth: int) -> str:
        """An operand nested `depth` levels; the innermost statement is the form's own fill,
        so a loop's BREAK still ends it."""
        if label == "query":
            return "SELECT * FROM ( " * (depth - 1) + "SELECT 1" + " )" * (depth - 1)
        core: str = self.style.fills.get("statement", CLASS_FILL["statement"])
        return "BEGIN " * depth + f"{core} ;" + " END ;" * (depth - 1) + " END"

    def _sequence(self, nodes: tuple[Node, ...], choices: _Choices) -> list[str]:
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
                    # The wrapper declares the object under its valid name, so a negative
                    # differs from its positive only in the operand under test.
                    self._token(node, _Choices())
                out.append(text)
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
