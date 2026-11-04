"""Derive obligation drafts mechanically from one form's normalized grammar."""

from __future__ import annotations

import itertools
from collections import Counter

from scripts.syntax_obligations._helpers.grammar import (
    is_repetition,
    parse,
    placeholder_name,
    render,
)
from scripts.syntax_obligations.constants import (
    CLASS_RULES,
    DEFAULT_CLASS,
    IDENTIFIER_ANCHOR,
    IDENTIFIER_CLASSES,
    KIND_ALTERNATIVE,
    KIND_IDENTIFIER,
    KIND_INTERACTION,
    KIND_NEGATIVE,
    KIND_NESTING,
    KIND_OPERAND_CLASS,
    KIND_OPERAND_MISSING,
    KIND_OPTION_ABSENT,
    KIND_OPTION_PRESENT,
    KIND_POSITIVE,
    NESTED_CLASSES,
    NESTING_DEPTHS,
    NO_BOUND,
    NODE_CHOICE,
    NODE_OPTIONAL,
    PARENS,
    STRING_CLASS,
    SUBGRAMMAR_CLASS,
    TARGET_WIDTH,
    TOKEN_KEYWORD,
    TOKEN_LITERAL,
    TOKEN_PUNCT,
)
from scripts.syntax_obligations.models import Draft, Node


def operand_class(name: str) -> str:
    """The operand class of a placeholder name, by the first matching rule."""
    for pattern, label in CLASS_RULES:
        if pattern.search(name):
            return label
    return DEFAULT_CLASS


class _Walk:
    """Collects drafts for one form while walking its grammar trees."""

    def __init__(self, form: str) -> None:
        self.form: str = form
        self.drafts: list[Draft] = []
        self.seen: Counter[str] = Counter()
        self.identifiers: dict[str, None] = {}
        self.classes: set[str] = set()
        self.dropped: dict[str, None] = {}

    def target(self, text: str, tail: str = "") -> str:
        """A per-form unique target, shortened before `tail`; repeats get `#2`, `#3` …"""
        width: int = max(TARGET_WIDTH - len(tail), TARGET_WIDTH // 2)
        short: str = (text if len(text) <= width else text[: width - 1] + "…") + tail
        self.seen[short] += 1
        return short if self.seen[short] == 1 else f"{short} #{self.seen[short]}"

    def add(self, kind: str, target: str, note: str) -> None:
        self.drafts.append(Draft(form=self.form, kind=kind, target=target, note=note))

    def sequence(
        self, nodes: tuple[Node, ...], where: str, top: bool, branch: bool = False
    ) -> list[str]:
        """Walk a sequence; returns the option_present targets of its non-repetition options.

        A lone placeholder in an option or a choice branch has no separate missing-operand case:
        omitting it is the option's absence or the whole choice's omission."""
        options: list[str] = []
        lone: bool = len(nodes) == 1 and (branch or where.startswith("["))
        for position, node in enumerate(nodes):
            if node.kind == NODE_OPTIONAL:
                options += self.optional(node, where)
                continue
            if node.kind == NODE_CHOICE:
                self.choice(node, where)
            else:
                self.token(node, where, lone=lone)
            if top and position > 0 and self._droppable(node):
                self.dropped[render((node,))] = None
        return options

    @staticmethod
    def _droppable(node: Node) -> bool:
        if node.kind == NODE_CHOICE:
            return True
        return node.token_kind == TOKEN_KEYWORD or (
            node.token_kind == TOKEN_PUNCT and node.text in PARENS
        )

    def token(self, node: Node, where: str, lone: bool) -> None:
        name: str = placeholder_name(node)
        if not name:
            return
        label: str = STRING_CLASS if node.token_kind == TOKEN_LITERAL else operand_class(name)
        self.classes.add(label)
        suffix: str = f" in {where}" if where else ""
        if label == "identifier":
            self.identifiers[node.text] = None
        target: str = self.target(f"{node.text}{suffix}")
        if not lone:
            self.add(KIND_OPERAND_MISSING, target, f"omit {node.text}")
        if label != SUBGRAMMAR_CLASS:
            self.add(KIND_OPERAND_CLASS, target, f"a valid {label}, and a wrong class")

    def optional(self, node: Node, where: str) -> list[str]:
        text: str = render((node,))
        repeated: bool = all(is_repetition(branch) for branch in node.branches)
        present: str = self.target(text)
        self.add(
            KIND_OPTION_PRESENT, present, "the list continued at least once" if repeated else ""
        )
        self.add(KIND_OPTION_ABSENT, present, "")
        if len(node.branches) > 1:
            for branch in node.branches:
                self.add(KIND_ALTERNATIVE, self.target(text, f" → {render(branch)}"), "")
        if repeated:
            return []
        for branch in node.branches:
            self.sequence(branch, present, top=False)
        return [present]

    def choice(self, node: Node, where: str) -> None:
        text: str = render((node,))
        for branch in node.branches:
            self.add(KIND_ALTERNATIVE, self.target(text, f" → {render(branch)}"), "")
        for branch in node.branches:
            self.sequence(branch, where, top=False, branch=True)


def derive(form: str, grammar: list[str]) -> list[Draft]:
    """Drafts in a fixed order: positive, then per production its parts, then form-wide cases."""
    walk: _Walk = _Walk(form)
    walk.add(KIND_POSITIVE, "", "the documented form with every required part and no optional part")
    interactions: list[tuple[str, str]] = []
    for production in grammar:
        options: list[str] = walk.sequence(parse(production), "", top=True)
        interactions += list(itertools.combinations(options, 2))
    for keyword in walk.dropped:
        walk.add(KIND_NEGATIVE, "", f"the form without its required {keyword}")
    for first, second in interactions:
        walk.drafts.append(
            Draft(
                form=form,
                kind=KIND_INTERACTION,
                target="",
                note="both options together",
                pair=(first, second),
            )
        )
    for placeholder in walk.identifiers:
        for label, description in IDENTIFIER_CLASSES:
            walk.drafts.append(
                Draft(
                    form=form,
                    kind=KIND_IDENTIFIER,
                    target=placeholder,
                    note=f"{label}: {description}",
                    anchor=IDENTIFIER_ANCHOR,
                )
            )
    for label in sorted(walk.classes & NESTED_CLASSES):
        walk.drafts.append(
            Draft(
                form=form,
                kind=KIND_NESTING,
                target="",
                note=f"{label} operands nested; {NO_BOUND}",
                depths=NESTING_DEPTHS,
            )
        )
    return walk.drafts
