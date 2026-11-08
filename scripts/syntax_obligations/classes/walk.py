"""`Walk`: collects one form's obligation drafts while walking its grammar trees."""

from __future__ import annotations

from collections import Counter
from dataclasses import replace

from scripts.syntax_obligations._helpers.elisions import is_pure_elision
from scripts.syntax_obligations._helpers.grammar import (
    is_repetition,
    operand_class,
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
    ARROWS,
    CALL_SEPARATORS,
    CALLEE_STARTS,
    CLAUSE_CALLEES,
    CLOSE_PAREN,
    EXPECT_OBSERVE,
    EXPECT_REJECT,
    EXPRESSION_ARGUMENT_FAMILIES,
    IDENTIFIER_CLASS,
    KIND_ALTERNATIVE,
    KIND_NEGATIVE,
    KIND_OPERAND_CLASS,
    KIND_OPERAND_MISSING,
    KIND_OPTION_ABSENT,
    KIND_OPTION_PRESENT,
    LEXICAL_WRONG,
    LEXICALLY_DISTINCT_WRONG_FILL,
    NESTED_CLASSES,
    NODE_CHOICE,
    NODE_OPTIONAL,
    NON_CALL_KEYWORDS,
    OBSERVED_WRONG,
    OPEN_PAREN,
    OPERAND_KEYWORDS,
    OPTION_ASSIGNMENTS,
    PARENS,
    PATH_SEPARATORS,
    STRING_CLASS,
    SUBGRAMMAR_CLASS,
    TARGET_BREAK,
    TARGET_ELLIPSIS,
    TARGET_WIDTH,
    TOKEN_KEYWORD,
    TOKEN_LITERAL,
    TOKEN_PUNCT,
    TOP_LEVEL_FAMILIES,
    UNPACKED_FORMS,
)
from scripts.syntax_obligations.models import Draft, Node, Spec


class Walk:
    """Collects drafts for one form while walking its grammar trees."""

    def __init__(self, form: str) -> None:
        self.form: str = form
        self.drafts: list[Draft] = []
        self.seen: Counter[str] = Counter()
        self.identifiers: dict[str, None] = {}
        self.classes: set[str] = set()
        self.dropped: dict[str, Spec] = {}
        self.dropped_at: dict[str, tuple[int, tuple[tuple[int, int], ...]]] = {}
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
        short: str = _cut_at_word(text=text, width=width) + tail
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
            prior: str = self.previous
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
            leading: bool = (
                position == 0
                and branch
                and len(nodes) > 1
                and node.kind != NODE_CHOICE
                and node.text not in PARENS
            )
            if (position > 0 or leading) and self._droppable(node):
                inside: str = f" in {where}" if not top and where.startswith("[") else ""
                key: str = render((node,)) + inside
                if not leading:
                    self.occurrences[(self.production, self.chosen, key)] += 1
                    if (count := self.occurrences[(self.production, self.chosen, key)]) > 1:
                        key = f"{key} #{count}"
                key = self._position_key(
                    key=key, node=node, prior=prior, inside=inside, leading=leading
                )
                self.dropped.setdefault(key, self.reject(action=ACTION_DROP_NODE, node=node))
                self.dropped_at.setdefault(key, (self.production, self.chosen))
                if argument and node.kind == NODE_CHOICE:
                    self.arity_drops.add(key)
        self.widest = max(self.widest, len(options))
        return options

    def _position_key(self, *, key: str, node: Node, prior: str, inside: str, leading: bool) -> str:
        """A keyword or arrow already dropped elsewhere, or leading a branch, gets its own key."""
        if node.kind == NODE_CHOICE or node.text in PARENS:
            return key
        here: tuple[int, tuple[tuple[int, int], ...]] = (self.production, self.chosen)
        if not leading and self.dropped_at.get(key, here) == here:
            return key
        base: str = f"{render((node,))} after {prior}{inside}"
        unique: str = base
        count: int = 1
        while unique in self.dropped_at and self.dropped_at[unique] != here:
            count += 1
            unique = f"{base} #{count}"
        return unique

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
        """A statement-level rejection observed instead of scored when it follows `after`."""
        if self.statements and self.previous in after:
            return replace(spec, expect=EXPECT_OBSERVE)
        return spec

    def _call_argument(self) -> bool:
        """Whether the next node is a whole argument of an ordinary call (arity)."""
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
            node.token_kind == TOKEN_PUNCT and node.text in PARENS | ARROWS
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
            spec=self.spec(action=ACTION_MIN_OPTIONS, node=node),
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


def _cut_at_word(*, text: str, width: int) -> str:
    """`text` if it fits in `width`, else cut at a word boundary to fit with an ellipsis."""
    if len(text) <= width:
        return text
    cut: str = text[: width - 1]
    if text[width - 1] != TARGET_BREAK and TARGET_BREAK in cut:
        cut = cut[: cut.rindex(TARGET_BREAK)]
    return cut.rstrip() + TARGET_ELLIPSIS
