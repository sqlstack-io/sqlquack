"""Moving a crate's items into new module files without changing what any of them say."""

from __future__ import annotations

from scripts.fensu._helpers.rust.items import parse_items
from scripts.fensu._helpers.rust.lexer import tokenize
from scripts.fensu.classes.crate import Crate
from scripts.fensu.constants import (
    BANG,
    BODYLESS_STRUCT_ENDS,
    CLOSE_ANGLE,
    CLOSE_BRACKET,
    COLON,
    COMMA,
    CRATE_VISIBILITY,
    DOT,
    EQUALS,
    FIELD_GROUP_CLOSERS,
    FIELD_GROUP_OPENERS,
    HASH,
    IDENT,
    IMPL,
    MINUS,
    MOD,
    NAMED_KINDS,
    OPEN_ANGLE,
    OPEN_BRACE,
    OPEN_BRACKET,
    PATH_SEPARATOR_TOKENS,
    PUB,
    PUNCT,
    SERDE_PATH_KEYS,
    SERDE_PATH_TOKENS,
    STRUCT,
    STRUCT_BODY_STARTS,
    TRIVIA_KINDS,
)
from scripts.fensu.models import Item, SourceModule, Token, UseLeaf


def destination_module(file: str) -> tuple[str, ...]:
    """The module path of a file under `src/`: `a/b/models.rs` is `crate::a::b::models`."""
    parts: list[str] = file.removesuffix(".rs").split("/")
    if parts[-1] == MOD:
        parts = parts[:-1]
    return ("crate", *parts)


def item_key(item: Item) -> str:
    """An item's selector: `kind:name`, `impl:Type` or `impl:Trait for Type`."""
    if item.kind == IMPL and item.trait_name:
        return f"impl:{item.trait_name} for {item.name}"
    return f"{item.kind}:{item.name}"


def innermost(*, module: SourceModule, offset: int) -> SourceModule:
    """The inline module of `module`'s file (or `module` itself) whose body holds `offset`."""
    for child in module.children.values():
        if child.inline is not None and child.file == module.file:
            if child.inline.body_start <= offset < child.inline.body_end:
                return innermost(module=child, offset=offset)
    return module


def leaf_text(leaf: UseLeaf) -> str:
    if leaf.glob:
        return "::".join((*leaf.path, "*"))
    text: str = "::".join(leaf.path)
    return text if leaf.name == leaf.path[-1] else f"{text} as {leaf.name}"


def leaf_module(*, module: SourceModule, item: Item, crate: Crate) -> SourceModule:
    """The module a `use` item was written in (a parent's, for leaves a glob brought in)."""
    current: SourceModule | None = module
    while current is not None:
        if any(candidate is item for _leaf, candidate in current.uses):
            return current
        current = current.parent
    return module


def is_serde_path(*, tokens: list[Token], significant: list[int], position: int) -> bool:
    if position < SERDE_PATH_TOKENS:
        return False
    equals: Token = tokens[significant[position - 1]]
    key: Token = tokens[significant[position - 2]]
    return equals.text == EQUALS and key.text in SERDE_PATH_KEYS


def follows_path_or_dot(*, tokens: list[Token], significant: list[int], position: int) -> bool:
    if position == 0:
        return False
    previous: Token = tokens[significant[position - 1]]
    if previous.kind == PUNCT and previous.text == DOT:
        return True
    if previous.kind == PUNCT and previous.text == COLON and position >= PATH_SEPARATOR_TOKENS:
        before: Token = tokens[significant[position - 2]]
        return before.kind == PUNCT and before.text == COLON and before.end == previous.start
    return False


def path_at(
    *, tokens: list[Token], significant: list[int], position: int
) -> tuple[tuple[str, ...], int]:
    segments: list[str] = [tokens[significant[position]].text]
    last: int = position
    while last + 3 < len(significant):
        first: Token = tokens[significant[last + 1]]
        second: Token = tokens[significant[last + 2]]
        following: Token = tokens[significant[last + 3]]
        if (
            first.text == COLON
            and second.text == COLON
            and first.end == second.start
            and following.kind == IDENT
        ):
            segments.append(following.text)
            last += 3
        else:
            break
    return tuple(segments), last


def is_macro_call(*, tokens: list[Token], significant: list[int], position: int) -> bool:
    if position + 1 >= len(significant):
        return False
    following: Token = tokens[significant[position + 1]]
    return following.kind == PUNCT and following.text == BANG


def bump_visibility(*, text: str, visibility: str = CRATE_VISIBILITY) -> str:
    """`text` with private items, fields and members visible, top-level ones as `visibility`."""
    tokens: list[Token] = tokenize(text=text)
    inserts: list[int] = []
    replacements: list[tuple[int, int]] = []
    for item in parse_items(text=text, tokens=tokens, start=0, end=len(text)):
        if (
            visibility != CRATE_VISIBILITY
            and item.kind in NAMED_KINDS
            and item.visibility == CRATE_VISIBILITY
        ):
            replacements.append((item.header_start, item.header_start + len(item.visibility)))
        inserts = _collect_bumps(text=text, tokens=tokens, item=item, inserts=inserts)
    edits: list[tuple[int, int, str]] = [(start, end, visibility) for start, end in replacements]
    for offset in set(inserts):
        top_level: bool = any(
            item.header_start == offset
            for item in parse_items(text=text, tokens=tokens, start=0, end=len(text))
        )
        edits.append((offset, offset, (visibility if top_level else CRATE_VISIBILITY) + " "))
    for start, end, replacement in sorted(edits, reverse=True):
        text = text[:start] + replacement + text[end:]
    return text


def _collect_bumps(*, text: str, tokens: list[Token], item: Item, inserts: list[int]) -> list[int]:
    collected: list[int] = list(inserts)
    if item.kind in NAMED_KINDS and not item.visibility:
        collected.append(item.header_start)
    if item.kind == STRUCT:
        collected.extend(_private_fields(tokens=tokens, item=item))
    if item.kind == IMPL and not item.trait_name:
        for member in parse_items(
            text=text, tokens=tokens, start=item.body_start, end=item.body_end
        ):
            if member.kind in NAMED_KINDS and not member.visibility:
                collected.append(member.header_start)
    return collected


def _private_fields(*, tokens: list[Token], item: Item) -> list[int]:
    significant: list[Token] = [
        t for t in tokens if item.keyword_start <= t.start < item.end and t.kind not in TRIVIA_KINDS
    ]
    position: int = 2
    angle: int = 0
    while position < len(significant):
        text: str = significant[position].text
        if text == OPEN_ANGLE:
            angle += 1
        elif text == CLOSE_ANGLE:
            angle -= 1
        elif angle == 0 and text in STRUCT_BODY_STARTS:
            break
        position += 1
    if position >= len(significant) or significant[position].text in BODYLESS_STRUCT_ENDS:
        return []
    closing: str = "}" if significant[position].text == OPEN_BRACE else ")"
    inserts: list[int] = []
    depth: int = 0
    expect_field: bool = True
    position += 1
    while position < len(significant):
        token: Token = significant[position]
        if depth == 0 and token.text == closing:
            break
        if expect_field and depth == 0:
            while token.text == HASH:
                position = _attribute_end(significant=significant, position=position) + 1
                token = significant[position]
            if token.text == closing:
                break
            if token.text != PUB:
                inserts.append(token.start)
            expect_field = False
        if token.text in FIELD_GROUP_OPENERS:
            depth += 1
        elif token.text in FIELD_GROUP_CLOSERS or (
            token.text == CLOSE_ANGLE and significant[position - 1].text != MINUS
        ):
            depth -= 1
        elif token.text == COMMA and depth == 0:
            expect_field = True
        position += 1
    return inserts


def _attribute_end(*, significant: list[Token], position: int) -> int:
    level: int = 0
    cursor: int = position + 1
    while True:
        if significant[cursor].text == OPEN_BRACKET:
            level += 1
        elif significant[cursor].text == CLOSE_BRACKET:
            level -= 1
            if level == 0:
                return cursor
        cursor += 1
