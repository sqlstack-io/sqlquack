"""Derive obligation drafts mechanically from one form's normalized grammar."""

from __future__ import annotations

from collections import Counter
from dataclasses import replace

from scripts.syntax_obligations._helpers.elisions import is_pure_elision
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
    ARGUMENT_STARTS,
    CALL_SEPARATORS,
    CALLEE_STARTS,
    CLASS_RULES,
    CLAUSE_CALLEES,
    CLOSE_PAREN,
    DEFAULT_CLASS,
    EXPECT_OBSERVE,
    EXPECT_REJECT,
    EXPRESSION_ARGUMENT_FAMILIES,
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
    NON_CALL_KEYWORDS,
    OBSERVED_WRONG,
    OPEN_PAREN,
    OPERAND_KEYWORDS,
    OPTION_ASSIGNMENTS,
    PARENS,
    PATH_SEPARATORS,
    QUOTED_WITHOUT_BLANK,
    QUOTED_WITHOUT_BLANK_NAMES,
    STRING_CLASS,
    SUBGRAMMAR_CLASS,
    TARGET_WIDTH,
    TOKEN_KEYWORD,
    TOKEN_LITERAL,
    TOKEN_PUNCT,
    TOP_LEVEL_FAMILIES,
    UNPACKED_FORMS,
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
        self.expression_arguments: bool = form.split(".")[1] in EXPRESSION_ARGUMENT_FAMILIES
        self.statements: bool = form.split(".")[1] in TOP_LEVEL_FAMILIES
        self.calls: list[bool] = []
        self.callees: list[str] = []
        self.named: bool = False
        self.arity_drops: set[str] = set()
        self.previous: str = ""
        self.argument_classes: Counter[str] = Counter()
        self.widest: int = 0
        self.occurrences: Counter[tuple[int, tuple[tuple[int, int], ...], str]] = Counter()

    def target(self, *, text: str, tail: str = "") -> str:
        """A per-form unique target, shortened before `tail`; repeats get `#2`, `#3` …"""
        width: int = max(TARGET_WIDTH - len(tail), TARGET_WIDTH // 2)
        short: str = (text if len(text) <= width else text[: width - 1] + "…") + tail
        self.seen[short] += 1
        return short if self.seen[short] == 1 else f"{short} #{self.seen[short]}"

    def add(
        self, *, kind: str, target: str, note: str, spec: Spec | None = None, arity: bool = False
    ) -> None:
        self.drafts.append(
            Draft(form=self.form, kind=kind, target=target, note=note, spec=spec, arity=arity)
        )

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
        """Walk a sequence; returns the option_present targets of its options."""
        options: list[str] = []
        omission_is_absence: bool = len(nodes) == 1 and (branch or where.startswith("["))
        for position, node in enumerate(nodes):
            argument: bool = self._call_argument()
            if node.kind == NODE_OPTIONAL:
                before: tuple[str, bool] = (self.previous, self.named)
                options += self.optional(node=node, where=where)
                self.previous, self.named = before
                continue
            if node.kind == NODE_CHOICE:
                self.choice(node=node, where=where)
            else:
                self.token(node=node, where=where, omission_is_absence=omission_is_absence)
                self._track(node)
            if position > 0 and self._droppable(node):
                inside: str = f" in {where}" if not top and where.startswith("[") else ""
                key: str = render((node,)) + inside
                self.occurrences[(self.production, self.chosen, key)] += 1
                if (count := self.occurrences[(self.production, self.chosen, key)]) > 1:
                    key = f"{key} #{count}"
                self.dropped.setdefault(key, self.reject(action=ACTION_DROP_NODE, node=node))
                if argument and node.kind == NODE_CHOICE:
                    self.arity_drops.add(key)
        self.widest = max(self.widest, len(options))
        return options

    def _track(self, node: Node) -> None:
        """Follow call parentheses and the token before the next one, in walk order."""
        if node.text == OPEN_PAREN:
            nested: bool = bool(self.calls) and self.calls[-1] and self._argument_start()
            self.calls.append(nested or self._callee())
            self.callees.append(self.previous.upper() if self._callee() and self.named else "")
        elif node.text == CLOSE_PAREN and self.calls:
            self.calls.pop()
            self.callees.pop()
        self.previous = node.text
        self.named = node.token_kind == TOKEN_KEYWORD

    def _callee(self) -> bool:
        return (
            bool(self.previous)
            and (self.previous[0].isalpha() or self.previous[0] in CALLEE_STARTS)
            and self.previous.upper() not in NON_CALL_KEYWORDS
        )

    def _unsure(self, *, spec: Spec, after: frozenset[str]) -> Spec:
        """A statement-level rejection observed instead of scored when it follows `after` (D97)."""
        if self.statements and self.previous in after:
            return replace(spec, expect=EXPECT_OBSERVE)
        return spec

    def _call_argument(self) -> bool:
        """Whether the next node is a whole argument of an ordinary call (D97 arity)."""
        return (
            self.expression_arguments
            and bool(self.callees)
            and self.callees[-1] not in CLAUSE_CALLEES
            and self.callees[-1][:1].isalpha()
            and self.previous in CALL_SEPARATORS
        )

    def _argument_start(self) -> bool:
        return self.previous.upper() in ARGUMENT_STARTS

    def _whole_argument(self) -> bool:
        """Whether the next operand fills a whole argument of a call, so it is an `<expr>`."""
        if not self.expression_arguments:
            return False
        if not self.previous or self.previous.upper() in OPERAND_KEYWORDS:
            return True
        return bool(self.calls) and self.calls[-1] and self._argument_start()

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
                spec=self._unsure(
                    spec=self.reject(action=ACTION_DROP_NODE, node=node), after=PATH_SEPARATORS
                ),
                arity=self._call_argument(),
            )
        if label != SUBGRAMMAR_CLASS:
            self.add(
                kind=KIND_OPERAND_CLASS, target=target, note=f"a valid {label}", spec=self.valid()
            )
        if label in LEXICAL_WRONG and self._whole_argument():
            self.argument_classes[label] += 1
        elif label in LEXICAL_WRONG:
            wrong: Spec = self._unsure(
                spec=self.reject(
                    action=ACTION_REPLACE_NODE,
                    node=node,
                    payload=LEXICALLY_DISTINCT_WRONG_FILL[label],
                ),
                after=OPTION_ASSIGNMENTS,
            )
            self.add(
                kind=KIND_NEGATIVE,
                target=target,
                note=f"{LEXICAL_WRONG[label]} in place of {node.text}",
                spec=replace(wrong, expect=EXPECT_OBSERVE) if label in OBSERVED_WRONG else wrong,
            )

    def valid(self) -> Spec:
        """The packed positive case that contains the current position."""
        if self.path and self.form in UNPACKED_FORMS:
            return self.spec(action=ACTION_MIN_OPTIONS)
        if self.path:
            return self.spec(action=ACTION_ALL_OPTIONS)
        return self.spec(action=ACTION_MIN_OPTIONS)

    def optional(self, *, node: Node, where: str) -> list[str]:
        if is_pure_elision(node):
            return []
        text: str = render((node,))
        repeated: bool = all(is_repetition(branch) for branch in node.branches)
        present: str = self.target(text=text)
        note: str = "the list continued at least once" if repeated else ""
        self.add(
            kind=KIND_OPTION_PRESENT,
            target=present,
            note=note,
            spec=replace(self.spec(action=ACTION_MIN_OPTIONS), path=self.path | {id(node)})
            if self.form in UNPACKED_FORMS
            else self.spec(action=ACTION_ALL_OPTIONS),
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
            return [present]
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
        before: tuple[str, bool] = (self.previous, self.named)
        for index, nodes in enumerate(node.branches):
            self.previous, self.named = before
            self.chosen = (*outer, (id(node), index)) if len(node.branches) > 1 else outer
            self.sequence(nodes=nodes, where=where, top=top, branch=branch)
        self.chosen = outer

    def choice(self, *, node: Node, where: str) -> None:
        self._alternatives(node=node, text=render((node,)))
        self._branches(node=node, where=where, top=False, branch=True)


def derive(*, form: str, grammar: list[str]) -> list[Draft]:
    """Drafts in a fixed order: positive, then per production its parts, then form-wide cases."""
    return derive_trees(form=form, grammar=grammar)[0]


def derive_trees(
    *, form: str, grammar: list[str]
) -> tuple[list[Draft], list[tuple[Node, ...]], Counter[str]]:
    """The drafts, parsed productions and the classes of skipped argument negatives (D97)."""
    if not grammar:
        return [], [], Counter()
    walk: _Walk = _Walk(form)
    note: str = "the documented form with every required part and no optional part"
    walk.add(kind=KIND_POSITIVE, target="", note=note, spec=walk.spec(action=ACTION_MIN_OPTIONS))
    trees: list[tuple[Node, ...]] = []
    for index, production in enumerate(grammar):
        walk.production = index
        trees.append(parse(production))
        walk.sequence(nodes=trees[-1], where="", top=True)
        if index and not any(d.spec and d.spec.production == index for d in walk.drafts):
            walk.add(
                kind=KIND_ALTERNATIVE,
                target=walk.target(text=production),
                note="a documented production with no option or operand of its own (D118)",
                spec=Spec(ACTION_MIN_OPTIONS, index),
            )
    for keyword, spec in walk.dropped.items():
        walk.add(
            kind=KIND_NEGATIVE,
            target="",
            note=f"the form without its required {keyword}",
            spec=spec,
            arity=keyword in walk.arity_drops,
        )
    if walk.widest >= MIN_INTERACTION_OPTIONS:
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
    return walk.drafts, trees, walk.argument_classes


def _class_note(*, label: str, description: str, placeholder: str) -> str:
    """The identifier class note, blank-free for names where a blank breaks Snowflake."""
    if label == IDENTIFIER_QUOTED and placeholder.strip("<>") in QUOTED_WITHOUT_BLANK_NAMES:
        return f"{label}: {QUOTED_WITHOUT_BLANK}"
    return f"{label}: {description}"
