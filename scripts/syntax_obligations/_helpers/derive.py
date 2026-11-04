"""Derive obligation drafts mechanically from one form's normalized grammar."""

from __future__ import annotations

from collections import Counter
from dataclasses import replace

from scripts.syntax_obligations._helpers.grammar import (
    is_repetition,
    parse,
    placeholder_name,
    render,
)
from scripts.syntax_obligations.constants import (
    ACTION_ALL_OPTIONS,
    ACTION_CHOICE_BRANCH,
    ACTION_DROP_NODE,
    ACTION_MIN_OPTIONS,
    ACTION_NEST_OPERANDS,
    ACTION_QUOTE_IDENTIFIERS,
    ACTION_REPLACE_NODE,
    ACTION_RESERVED_WORD,
    CLASS_RULES,
    DEFAULT_CLASS,
    EXPECT_REJECT,
    IDENTIFIER_ANCHOR,
    IDENTIFIER_CLASS,
    IDENTIFIER_CLASSES,
    IDENTIFIER_QUOTED,
    IDENTIFIER_RESERVED,
    KIND_ALTERNATIVE,
    KIND_IDENTIFIER,
    KIND_INTERACTION_ALL,
    KIND_NEGATIVE,
    KIND_NESTING,
    KIND_OPERAND_CLASS,
    KIND_OPERAND_MISSING,
    KIND_OPTION_ABSENT,
    KIND_OPTION_PRESENT,
    KIND_POSITIVE,
    LEXICAL_WRONG,
    LEXICALLY_DISTINCT_WRONG_FILL,
    MIN_INTERACTION_OPTIONS,
    NESTED_CLASSES,
    NESTING_DEPTHS,
    NO_BOUND,
    NODE_CHOICE,
    NODE_OPTIONAL,
    PARENS,
    QUOTED_WITHOUT_BLANK,
    QUOTED_WITHOUT_BLANK_NAMES,
    STRING_CLASS,
    SUBGRAMMAR_CLASS,
    TARGET_WIDTH,
    TOKEN_KEYWORD,
    TOKEN_LITERAL,
    TOKEN_PUNCT,
)
from scripts.syntax_obligations.models import Draft, Node, Spec


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
        self.dropped: dict[str, Spec] = {}
        self.production: int = 0
        self.path: frozenset[int] = frozenset()
        self.identifier_nodes: dict[str, Spec] = {}
        self.quote_specs: dict[str, Spec] = {}
        self.nest_specs: dict[str, Spec] = {}
        self.chosen: tuple[tuple[int, int], ...] = ()

    def target(self, *, text: str, tail: str = "") -> str:
        """A per-form unique target, shortened before `tail`; repeats get `#2`, `#3` …"""
        width: int = max(TARGET_WIDTH - len(tail), TARGET_WIDTH // 2)
        short: str = (text if len(text) <= width else text[: width - 1] + "…") + tail
        self.seen[short] += 1
        return short if self.seen[short] == 1 else f"{short} #{self.seen[short]}"

    def add(self, *, kind: str, target: str, note: str, spec: Spec | None = None) -> None:
        self.drafts.append(Draft(form=self.form, kind=kind, target=target, note=note, spec=spec))

    def reject(self, *, action: str, node: Node, payload: str = "") -> Spec:
        """A spec whose case is expected to be a syntax error."""
        return replace(self.spec(action=action, node=node, payload=payload), expect=EXPECT_REJECT)

    def spec(self, *, action: str, node: Node | None = None, payload: str = "") -> Spec:
        """A case spec at the current production and option path."""
        key: int = id(node) if node else 0
        return Spec(action, self.production, key, self.path, payload, self.chosen)

    def sequence(
        self, *, nodes: tuple[Node, ...], where: str, top: bool, branch: bool = False
    ) -> list[str]:
        """Walk a sequence; returns the option_present targets of its non-repetition options."""
        options: list[str] = []
        omission_is_absence: bool = len(nodes) == 1 and (branch or where.startswith("["))
        for position, node in enumerate(nodes):
            if node.kind == NODE_OPTIONAL:
                options += self.optional(node=node, where=where)
                continue
            if node.kind == NODE_CHOICE:
                self.choice(node=node, where=where)
            else:
                self.token(node=node, where=where, omission_is_absence=omission_is_absence)
            if position > 0 and self._droppable(node):
                inside: str = f" in {where}" if not top and where.startswith("[") else ""
                self.dropped.setdefault(
                    render((node,)) + inside, self.reject(action=ACTION_DROP_NODE, node=node)
                )
        return options

    @staticmethod
    def _droppable(node: Node) -> bool:
        if node.kind == NODE_CHOICE:
            return True
        return node.token_kind == TOKEN_KEYWORD or (
            node.token_kind == TOKEN_PUNCT and node.text in PARENS
        )

    def token(self, *, node: Node, where: str, omission_is_absence: bool) -> None:
        """Operand drafts; a lone operand of an option or branch gets no missing-operand case."""
        name: str = placeholder_name(node)
        if not name:
            return
        label: str = STRING_CLASS if node.token_kind == TOKEN_LITERAL else operand_class(name)
        self.classes.add(label)
        suffix: str = f" in {where}" if where else ""
        if label == IDENTIFIER_CLASS:
            self.identifiers[node.text] = None
            self.identifier_nodes.setdefault(
                node.text, self.spec(action=ACTION_RESERVED_WORD, node=node)
            )
            self.quote_specs.setdefault(
                node.text,
                self.spec(
                    action=ACTION_QUOTE_IDENTIFIERS, node=node, payload=placeholder_name(node)
                ),
            )
        target: str = self.target(text=f"{node.text}{suffix}")
        if label in NESTED_CLASSES:
            self.nest_specs.setdefault(
                label, self.spec(action=ACTION_NEST_OPERANDS, node=node, payload=label)
            )
        if not omission_is_absence:
            self.add(
                kind=KIND_OPERAND_MISSING,
                target=target,
                note=f"omit {node.text}",
                spec=self.reject(action=ACTION_DROP_NODE, node=node),
            )
        if label != SUBGRAMMAR_CLASS:
            self.add(
                kind=KIND_OPERAND_CLASS, target=target, note=f"a valid {label}", spec=self.valid()
            )
        if label in LEXICAL_WRONG:
            self.add(
                kind=KIND_NEGATIVE,
                target=target,
                note=f"{LEXICAL_WRONG[label]} in place of {node.text}",
                spec=self.reject(
                    action=ACTION_REPLACE_NODE,
                    node=node,
                    payload=LEXICALLY_DISTINCT_WRONG_FILL[label],
                ),
            )

    def valid(self) -> Spec:
        """The packed positive case that contains the current position."""
        if self.path:
            return self.spec(action=ACTION_ALL_OPTIONS)
        return self.spec(action=ACTION_MIN_OPTIONS)

    def optional(self, *, node: Node, where: str) -> list[str]:
        text: str = render((node,))
        repeated: bool = all(is_repetition(branch) for branch in node.branches)
        present: str = self.target(text=text)
        note: str = "the list continued at least once" if repeated else ""
        self.add(
            kind=KIND_OPTION_PRESENT,
            target=present,
            note=note,
            spec=self.spec(action=ACTION_ALL_OPTIONS),
        )
        self.add(
            kind=KIND_OPTION_ABSENT,
            target=present,
            note="",
            spec=self.spec(action=ACTION_MIN_OPTIONS),
        )
        if len(node.branches) > 1:
            self._alternatives(node=node, text=text)
        if repeated:
            return []
        outer: frozenset[int] = self.path
        self.path = outer | {id(node)}
        self._branches(node=node, where=present, top=False, branch=False)
        self.path = outer
        return [present]

    def _alternatives(self, *, node: Node, text: str) -> None:
        for index, branch in enumerate(node.branches):
            spec: Spec = self.spec(action=ACTION_CHOICE_BRANCH, node=node, payload=str(index))
            target: str = self.target(text=text, tail=f" → {render(branch)}")
            self.add(kind=KIND_ALTERNATIVE, target=target, note="", spec=spec)

    def _branches(self, *, node: Node, where: str, top: bool, branch: bool) -> None:
        outer: tuple[tuple[int, int], ...] = self.chosen
        for index, nodes in enumerate(node.branches):
            self.chosen = (*outer, (id(node), index)) if len(node.branches) > 1 else outer
            self.sequence(nodes=nodes, where=where, top=top, branch=branch)
        self.chosen = outer

    def choice(self, *, node: Node, where: str) -> None:
        self._alternatives(node=node, text=render((node,)))
        self._branches(node=node, where=where, top=False, branch=True)


def derive(*, form: str, grammar: list[str]) -> list[Draft]:
    """Drafts in a fixed order: positive, then per production its parts, then form-wide cases."""
    return derive_trees(form=form, grammar=grammar)[0]


def derive_trees(*, form: str, grammar: list[str]) -> tuple[list[Draft], list[tuple[Node, ...]]]:
    """The drafts with their case specs, and the parsed productions the specs point into."""
    walk: _Walk = _Walk(form)
    note: str = "the documented form with every required part and no optional part"
    walk.add(kind=KIND_POSITIVE, target="", note=note, spec=walk.spec(action=ACTION_MIN_OPTIONS))
    trees: list[tuple[Node, ...]] = []
    widest: int = 0
    for index, production in enumerate(grammar):
        walk.production = index
        trees.append(parse(production))
        widest = max(widest, len(walk.sequence(nodes=trees[-1], where="", top=True)))
    for keyword, spec in walk.dropped.items():
        walk.add(
            kind=KIND_NEGATIVE,
            target="",
            note=f"the form without its required {keyword}",
            spec=spec,
        )
    if widest >= MIN_INTERACTION_OPTIONS:
        note = "every independent option of one production together"
        walk.add(kind=KIND_INTERACTION_ALL, target="", note=note, spec=Spec(ACTION_ALL_OPTIONS))
    for placeholder in walk.identifiers:
        for label, description in IDENTIFIER_CLASSES:
            reserved: Spec = walk.identifier_nodes[placeholder]
            walk.drafts.append(
                Draft(
                    form=form,
                    kind=KIND_IDENTIFIER,
                    target=placeholder,
                    note=_class_note(label=label, description=description, placeholder=placeholder),
                    anchor=IDENTIFIER_ANCHOR,
                    spec=reserved
                    if label == IDENTIFIER_RESERVED
                    else walk.quote_specs[placeholder],
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
                spec=walk.nest_specs[label],
            )
        )
    return walk.drafts, trees


def _class_note(*, label: str, description: str, placeholder: str) -> str:
    """The identifier class note, blank-free for names where a blank breaks Snowflake."""
    if label == IDENTIFIER_QUOTED and placeholder.strip("<>") in QUOTED_WITHOUT_BLANK_NAMES:
        return f"{label}: {QUOTED_WITHOUT_BLANK}"
    return f"{label}: {description}"
