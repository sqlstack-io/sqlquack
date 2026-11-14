"""`Renderer`: renders the case SQL of one form from its grammar trees and a case spec."""

from __future__ import annotations

import json
from dataclasses import replace

from scripts.grammar_obligations._helpers.elisions import elision_key, is_pure_elision
from scripts.grammar_obligations._helpers.grammar import (
    is_repetition,
    operand_class,
    placeholder_name,
    render,
)
from scripts.grammar_obligations._helpers.qualify import qualified
from scripts.grammar_obligations.constants import (
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
    CLOSE_PAREN,
    CLOSE_REPEAT_AFTER,
    CONTEXT_BLOCK,
    CONTEXT_PROCEDURE,
    CONTEXT_RAW,
    CONTEXT_TOP,
    DOLLAR_QUOTE,
    DOT_TOKEN,
    DOUBLE_QUOTE,
    DOUBLED_STAGE_SIGIL,
    EQUALS_TOKEN,
    EXPECT_ACCEPT,
    EXPECT_REJECT,
    GLUED_INFIX_LEFT,
    GLUED_INFIX_RIGHT,
    GLUED_PREFIXES,
    HAND_ALSO_DOCUMENTED,
    HAND_ALSO_SQL,
    HAND_CORE,
    HAND_DOCUMENTED,
    HAND_SIZE,
    HAND_SQL,
    LABEL_SLOT,
    LIST_ITEM_KEYWORDS,
    LIST_SEPARATOR,
    NAME_FILL,
    NODE_CHOICE,
    NODE_OPTIONAL,
    NODE_TOKEN,
    OPEN_PAREN,
    OPENED,
    PROCEDURE_WRAPPER,
    QUERY_CLASS,
    QUOTED_WITHOUT_BLANK_NAMES,
    REPEAT_FILLS,
    REPEAT_SLOT,
    RESERVED_WORD,
    SET_ANCHOR_PREFERENCE,
    SET_KEYWORDS,
    SINGLE_CHARACTER_LITERAL_LENGTH,
    SINGLE_QUOTE,
    STAGE_GLUES,
    STAGE_KEYWORD,
    STATEMENT_CLASS,
    STATEMENT_TERMINATOR,
    STRING_CLASS,
    TOKEN_ELISION,
    TOKEN_KEYWORD,
    TOKEN_LITERAL,
    TOKEN_PLACEHOLDER,
    TOKEN_PUNCT,
)
from scripts.grammar_obligations.models import Choices, FormStyle, Node, Rendered, Spec


class Renderer:
    """Renders the cases of one form."""

    def __init__(self, *, trees: list[tuple[Node, ...]], style: FormStyle) -> None:
        self.trees: list[tuple[Node, ...]] = trees
        self.style: FormStyle = style
        self._used: dict[str, list[str]] = {}
        self._keywords: list[str] = []
        self._last_choice: tuple[Node, int, list[str]] | None = None

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
        if self.style.embedded:
            text = text.rstrip().removesuffix(STATEMENT_TERMINATOR).rstrip()
        whole: str = f"{self.style.prefix}{opened}{self._terminated(text)}{suffix}".strip()
        declarations: str = self._declarations()
        if not declarations:
            return whole
        if whole.startswith("DECLARE "):
            return f"DECLARE {declarations} {whole.removeprefix('DECLARE ')}"
        return f"DECLARE {declarations} {whole}"

    def text(self, *, production: int, choices: Choices) -> str:
        """The rendered form text of one production; records the names it fills in."""
        self._used = {}
        self._keywords = []
        self._last_choice = None
        joined: str = DOUBLED_STAGE_SIGIL.sub(
            "", " ".join(self._sequence(nodes=self.trees[production], choices=choices))
        )
        glued: str = GLUED_INFIX_RIGHT.sub("!", GLUED_PREFIXES.sub(r"\1", joined))
        staged: str = _stage_glued(GLUED_INFIX_LEFT.sub("!", glued))
        keywords: set[str] = set(self._keywords)
        return STAGE_KEYWORD.sub(
            lambda m: f"{m[1]} {m[2]}" if m[2] in keywords - {RESERVED_WORD} else m[0], staged
        )

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

    def _choices(self, spec: Spec) -> Choices:
        choices: Choices = Choices(full=spec.action == ACTION_ALL_OPTIONS, path=spec.path)
        choices.branch.update(dict(spec.chosen))
        if spec.action == ACTION_CHOICE_BRANCH:
            index: int = int(spec.payload)
            for twin in self._twins(spec):
                choices.branch.setdefault(twin, index)
            choices.branch[spec.node] = index
            choices = replace(choices, path=spec.path | {spec.node})
        elif spec.action == ACTION_DROP_NODE:
            choices = replace(choices, drop=frozenset({spec.node}))
        elif spec.action == ACTION_REPLACE_NODE:
            choices.replace[spec.node] = spec.payload
        elif spec.action == ACTION_QUOTE_IDENTIFIERS:
            choices = replace(choices, quote=spec.payload)
        else:
            choices = replace(choices, absent=spec.node)
        return choices

    @staticmethod
    def _at(*, spec: Spec, replace: str) -> Choices:
        return Choices(path=spec.path, branch=dict(spec.chosen), replace={spec.node: replace})

    def _nest_at(self, *, spec: Spec, depth: int) -> Choices:
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
            sizes: tuple[int, ...] = (int(fields[HAND_SIZE]),) if HAND_SIZE in fields else depths
            return [
                Rendered(self.nested_hand(fields=fields, depth=depth), expect, stage)
                for depth in sizes
            ]
        context: str = fields.get("context", CONTEXT_BLOCK)
        texts: list[tuple[str, str, str, bool]] = []
        if HAND_SQL in fields:
            texts.append((fields[HAND_SQL], expect, stage, bool(fields.get(HAND_DOCUMENTED))))
        if HAND_ALSO_SQL in fields:
            texts.append(
                (
                    fields[HAND_ALSO_SQL],
                    fields.get("also_expect", expect),
                    fields.get("also_stage", ""),
                    bool(fields.get(HAND_ALSO_DOCUMENTED)),
                )
            )
        rendered: list[Rendered] = []
        for text, want, later, documented in texts or [("", expect, stage, False)]:
            self._used = {}
            self._keywords = []
            body: str = text or self.text(production=0, choices=Choices())
            if context == CONTEXT_RAW:
                sql: str = body
            elif context == CONTEXT_TOP:
                sql = self._terminated(body)
            elif context == CONTEXT_PROCEDURE:
                sql = PROCEDURE_WRAPPER.format(block=self.wrap(body))
            else:
                sql = self.wrap(body)
            rendered.append(Rendered(sql, want, later, documented))
        return rendered

    def nested_hand(self, *, fields: dict[str, str], depth: int) -> str:
        """A hand limit `depth` deep: outer_open, open*(depth-1), core, closes; {n} counts."""
        self._used = {}
        self._keywords = []
        inner: int = depth - 1
        opens: list[str] = [
            fields.get("open", "").replace(REPEAT_SLOT, str(level)) for level in range(1, depth)
        ]
        parts: list[str] = [
            fields.get("outer_open", ""),
            *opens,
            fields[HAND_CORE],
            *[fields.get("close", "")] * inner,
            fields.get("outer_close", ""),
        ]
        text: str = " ".join(part for part in parts if part)
        return text if fields.get("context") == CONTEXT_RAW else self.wrap(text)

    def _nest(self, *, label: str, depth: int) -> str:
        """An operand nested `depth` levels; a statement's innermost is the form's own fill."""
        if label == QUERY_CLASS:
            return "SELECT * FROM ( " * (depth - 1) + "SELECT 1" + " )" * (depth - 1)
        core: str = self.style.fills.get(STATEMENT_CLASS, CLASS_FILL[STATEMENT_CLASS])
        return "BEGIN " * depth + f"{core} ;" + " END ;" * (depth - 1) + " END"

    def _sequence(self, *, nodes: tuple[Node, ...], choices: Choices) -> list[str]:
        """Rendered tokens; a replaced operand is still declared under its valid name."""
        out: list[str] = []
        dangling: int = -1
        dropped: bool = False
        for position, node in enumerate(nodes):
            key: int = id(node)
            if node.token_kind == TOKEN_KEYWORD and node.text.upper() in SET_KEYWORDS:
                choices = self._set_anchor(rest=nodes[position + 1 :], choices=choices)
            if key in choices.drop:
                if dangling >= 0 and node.text == CLOSE_PAREN:
                    del out[dangling]
                    dangling = -1
                dropped = True
                continue
            after_drop: bool = dropped
            dropped = False
            emitted: int = len(out)
            if key in choices.replace:
                text: str = choices.replace[key]
                name: str = placeholder_name(node)
                if text in (RESERVED_WORD, f'"{RESERVED_WORD}"'):
                    self._used.setdefault(name, []).append(text)
                elif name:
                    self._token(node=node, choices=Choices())
                out.append(qualified(prefix=self.style.qualify.get(name, ""), text=text))
            elif node.kind == NODE_OPTIONAL:
                closes: bool = nodes[position + 1 : position + 2] != () and (
                    nodes[position + 1].text == CLOSE_PAREN
                )
                out += self._optional(node=node, choices=choices, before=out, closes=closes)
            elif node.kind == NODE_CHOICE:
                index: int = choices.branch.get(key, 0)
                chosen: list[str] = self._sequence(nodes=node.branches[index], choices=choices)
                self._last_choice = (node, index, chosen)
                out += chosen
            elif node.token_kind == TOKEN_ELISION and out and out[-1] == LIST_SEPARATOR:
                if not after_drop:
                    out.pop()
            elif node.token_kind == TOKEN_ELISION:
                fill: str = self.style.elisions.get(
                    elision_key(keywords=self._keywords, known=self.style.elisions), ""
                )
                out += [fill] if fill else []
            else:
                if node.token_kind == TOKEN_KEYWORD:
                    self._keywords.append(node.text.upper())
                quoted: bool = "".join(out[-2:]).endswith(DOLLAR_QUOTE)
                out.append(self._token(node=node, choices=choices, dollar_quoted=quoted))
            dangling = _dangling_separator(out=out, emitted=emitted, dangling=dangling, node=node)
        return out

    @staticmethod
    def _set_anchor(*, rest: tuple[Node, ...], choices: Choices) -> Choices:
        """Choices with one property of an all-optional SET list, if none would render."""
        options: list[Node] = _set_options(rest)
        if choices.full or not options or any(id(n) in choices.path for n in options):
            return choices
        named: dict[str, Node] = {
            _tokens(n.branches[0])[0].text.upper(): n for n in reversed(options) if n.branches[0]
        }
        ranked: list[Node] = [named[k] for k in SET_ANCHOR_PREFERENCE if k in named] + options
        anchor: Node | None = next((n for n in ranked if id(n) != choices.absent), None)
        if anchor is None:
            return choices
        return replace(choices, path=choices.path | {id(anchor)})

    def _optional(
        self, *, node: Node, choices: Choices, before: list[str], closes: bool = False
    ) -> list[str]:
        """An optional part; a `[ , ... ]` repeats the list item it continues."""
        key: int = id(node)
        if is_pure_elision(node) or not (choices.full or key in choices.path):
            return []
        clause: str | None = self.style.clauses.get(render((node,)))
        branch: tuple[Node, ...] = node.branches[choices.branch.get(key, 0)]
        if clause is not None:
            return _without_dropped(written=[clause], branch=branch, choices=choices)
        if is_repetition(branch) and not any(placeholder_name(n) for n in _tokens(branch)):
            start: int = _repeat_start(before=before, keywords=set(self._keywords))
            cut_by_keyword: bool = start >= 0 and before[start] not in CLOSE_REPEAT_AFTER
            item: list[str] = before[start + 1 :]
            separator: int = _separator_start(before=before)
            if cut_by_keyword and (
                item[:1] == [EQUALS_TOKEN]
                or (
                    closes
                    and separator >= 0
                    and before[separator] == OPEN_PAREN
                    and before[separator + 1].upper() not in self._keywords
                )
            ):
                item = before[separator + 1 :]
            elif cut_by_keyword:
                item = self._whole_choice(item=item, before=before)
            other: list[str] = self._other_branch(item=item, choices=choices)
            repeated: list[str] = [",", *(other or [self._repeat(text) for text in item])]
            return _without_dropped(written=repeated, branch=branch, choices=choices)
        rendered: list[str] = self._sequence(nodes=branch, choices=choices)
        if is_repetition(branch) and _ends_in_continuation(branch) and rendered:
            return [*rendered, LIST_SEPARATOR, *(self._repeat(text) for text in rendered)]
        return rendered

    def _whole_choice(self, *, item: list[str], before: list[str]) -> list[str]:
        """The item, widened to the last chosen branch when a keyword in it cut the item short."""
        if self._last_choice is None:
            return item
        chosen: list[str] = self._last_choice[2]
        if len(chosen) > len(item) and before[-len(chosen) :] == chosen:
            return chosen
        return item

    def _other_branch(self, *, item: list[str], choices: Choices) -> list[str]:
        """The next branch of the choice a list item copies, so a list names distinct items."""
        if self._last_choice is None:
            return []
        node, index, chosen = self._last_choice
        if chosen != item or len(node.branches) == 1:
            return []
        if not _keywords_only(node.branches[index]):
            return []
        for step in range(1, len(node.branches)):
            following: tuple[Node, ...] = node.branches[(index + step) % len(node.branches)]
            if not _keywords_only(following):
                continue
            keywords: int = len(self._keywords)
            other: list[str] = self._sequence(nodes=following, choices=choices)
            if other != chosen:
                return other
            del self._keywords[keywords:]
        return []

    def _repeat(self, text: str) -> str:
        """A copied list item, continued with its REPEAT_FILLS value when one is declared."""
        for (name, first), second in REPEAT_FILLS.items():
            if first == text and text in self._used.get(name, []):
                return second
        return text

    def _token(self, *, node: Node, choices: Choices, dollar_quoted: bool = False) -> str:
        if node.token_kind == TOKEN_LITERAL and node.text in self.style.literal_fills:
            return self.style.literal_fills[node.text]
        name: str = placeholder_name(node)
        if not name:
            quoted: bool = (
                node.token_kind == TOKEN_LITERAL
                and len(node.text) == SINGLE_CHARACTER_LITERAL_LENGTH
            )
            return node.text[1] if quoted and node.text[0] == DOUBLE_QUOTE else node.text
        fill: str = self.style.fills.get(name) or NAME_FILL.get(name) or ""
        if node.token_kind == TOKEN_LITERAL and fill and not fill.startswith("'"):
            fill = ""
        label: str = operand_class(name)
        if not fill:
            fill = CLASS_FILL.get(STRING_CLASS if node.token_kind == TOKEN_LITERAL else label, "v1")
        if self._used.get(name):
            fill = REPEAT_FILLS.get((name, fill), fill)
        if choices.quote == name and node.token_kind == TOKEN_PLACEHOLDER:
            fill = f'"{fill}Q"' if name in QUOTED_WITHOUT_BLANK_NAMES else f'"{fill} Q"'
        fill = qualified(prefix=self.style.qualify.get(name, ""), text=fill)
        if dollar_quoted:
            fill = _dollar_body(fill)
        self._used.setdefault(name, []).append(fill)
        return fill


def _stage_glued(text: str) -> str:
    """`@ns . stage / path /` written as the one token `@ns.stage/path/`."""
    previous: str = ""
    while previous != text:
        previous = text
        for pattern in STAGE_GLUES:
            text = pattern.sub(r"\1", text)
    return text


def _dollar_body(fill: str) -> str:
    """A fill written inside the page's own `$$ … $$`, without its own dollar or single quotes."""
    for quote in (DOLLAR_QUOTE, SINGLE_QUOTE):
        if len(fill) > 2 * len(quote) and fill.startswith(quote) and fill.endswith(quote):
            return fill[len(quote) : -len(quote)].strip()
    return fill


def _ends_in_continuation(branch: tuple[Node, ...]) -> bool:
    """Whether a list is `<item> , ...`, so that its rendered item stands for the whole list."""
    tokens: list[Node] = [node for node in branch if node.kind == NODE_TOKEN]
    texts: list[str] = [node.text for node in tokens]
    return (
        texts[:1] != [LIST_SEPARATOR]
        and texts[-2:-1] == [LIST_SEPARATOR]
        and [node.token_kind for node in tokens[-1:]] == [TOKEN_ELISION]
    )


def _without_dropped(
    *, written: list[str], branch: tuple[Node, ...], choices: Choices
) -> list[str]:
    """A written repeat or clause less the dropped token: the first `(` or `,`, the last `)`."""
    dropped: list[str] = [token.text for token in _tokens(branch) if id(token) in choices.drop]
    if not dropped:
        return written
    words: list[str] = " ".join(written).split()
    for text in dropped:
        if text not in words:
            continue
        at: int = words.index(text)
        if text == CLOSE_PAREN:
            at = len(words) - 1 - words[::-1].index(text)
        del words[at]
    return words


def _dangling_separator(*, out: list[str], emitted: int, dangling: int, node: Node) -> int:
    """Drop the `,` omitted optionals leave beside `(` or `)`; returns the pending comma index."""
    if len(out) < emitted:
        return -1 if dangling >= len(out) else dangling
    if len(out) == emitted:
        if node.kind != NODE_OPTIONAL or dangling >= 0:
            return dangling
        if out[-1:] == [LIST_SEPARATOR]:
            return len(out) - 1
        return OPENED if out[-1:] == [OPEN_PAREN] else dangling
    if dangling == OPENED and out[emitted] == LIST_SEPARATOR:
        del out[emitted]
    elif dangling >= 0 and out[emitted] == CLOSE_PAREN:
        del out[dangling]
    return -1


def _repeat_start(*, before: list[str], keywords: set[str]) -> int:
    """The repeated item starts after the nearest open `(`, `,` or clause keyword (not a call)."""
    depth: int = 0
    for index in range(len(before) - 1, -1, -1):
        text: str = before[index]
        calls: bool = before[index + 1 : index + 2] == [OPEN_PAREN]
        if text == CLOSE_PAREN:
            depth += 1
        elif text == OPEN_PAREN and depth:
            depth -= 1
        elif depth == 0 and (
            text in CLOSE_REPEAT_AFTER
            or (
                index < len(before) - 1
                and text.upper() in keywords
                and text.upper() not in LIST_ITEM_KEYWORDS
                and not calls
            )
        ):
            return index
    return -1


def _separator_start(*, before: list[str]) -> int:
    """The index of the nearest open `(` or `,` outside parentheses, or -1."""
    depth: int = 0
    for index in range(len(before) - 1, -1, -1):
        text: str = before[index]
        if text == CLOSE_PAREN:
            depth += 1
        elif text == OPEN_PAREN and depth:
            depth -= 1
        elif depth == 0 and text in CLOSE_REPEAT_AFTER:
            return index
    return -1


def _keywords_only(branch: tuple[Node, ...]) -> bool:
    """Whether a branch is fixed words: keywords, dotted ones such as SNOWFLAKE.ML.FORECAST too."""
    return all(
        token.token_kind == TOKEN_KEYWORD
        or (token.token_kind == TOKEN_PUNCT and token.text == DOT_TOKEN)
        for token in _tokens(branch)
    )


def _tokens(nodes: tuple[Node, ...]) -> list[Node]:
    found: list[Node] = []
    for node in nodes:
        if node.branches:
            for branch in node.branches:
                found += _tokens(branch)
        else:
            found.append(node)
    return found


def _set_options(rest: tuple[Node, ...]) -> list[Node]:
    """The plain options of a list made only of options and one-branch groups, else none."""
    found: list[Node] = []
    for node in rest:
        if node.kind == NODE_CHOICE and len(node.branches) == 1:
            inner: list[Node] = _set_options(node.branches[0])
            if not inner:
                return []
            found += inner
        elif node.kind != NODE_OPTIONAL:
            return []
        elif not is_pure_elision(node) and not all(is_repetition(b) for b in node.branches):
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
