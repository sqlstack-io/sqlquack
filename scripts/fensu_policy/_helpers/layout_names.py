"""Pure checks of crate, enum and script names against the repository's layout policy."""

from __future__ import annotations

import re

from scripts.fensu_policy.constants import (
    BLOCK_COMMENT_MARK,
    BLOCK_COMMENT_OPEN,
    CATALOG_WORD,
    CHAR_LITERAL,
    CRATE_PREFIX,
    CRATE_SEPARATOR,
    ENUM_BODY,
    IDENTIFIER_CHARACTER,
    LEXEME_START,
    LINE_COMMENT_OPEN,
    MAX_NAMED_DIALECTS,
    MIN_CRATE_NAME_PARTS,
    NESTED_PATH_PARTS,
    PRODUCT_ROOT,
    PYTHON_SUFFIX,
    QUOTED_STRING,
    RAW_STRING_START,
    SCRIPT_SEPARATOR,
    SYNTAX_ROLE,
    TOOL_SUFFIX,
    TOOLS_ROOT,
    VARIANT,
    VARIANT_SEPARATOR,
)
from scripts.fensu_policy.models import CrateDependencies, CrateName, SourceLine


def read_crate_name(
    *, name: str, roles: tuple[str, ...], dialects: tuple[str, ...]
) -> CrateName | str:
    """Read a product crate name, or say why it does not follow sqlquack-<role>[-<dialect>]."""
    if name == CRATE_PREFIX:
        return CrateName(role="", dialects=())
    parts: list[str] = name.split(CRATE_SEPARATOR)
    if parts[0] != CRATE_PREFIX or len(parts) < MIN_CRATE_NAME_PARTS:
        return f"`{name}` does not start with `{CRATE_PREFIX}-`"
    role: str = parts[1]
    rest: tuple[str, ...] = tuple(parts[2:])
    if role in dialects:
        return f"`{name}` puts the dialect `{role}` before the role; name the role first"
    if role not in roles:
        return f"`{name}` starts with `{role}`, which is not one of the configured roles"
    unknown: tuple[str, ...] = tuple(part for part in rest if part not in dialects)
    if unknown:
        return f"`{name}` qualifies its role with `{'-'.join(unknown)}`, which is not a dialect"
    if len(rest) > MAX_NAMED_DIALECTS:
        return (
            f"`{name}` names more than one dialect; every conversion targets DuckDB, so a "
            "conversion crate names only its source dialect"
        )
    return CrateName(role=role, dialects=rest)


def crate_name_problem(
    *, name: str, directory: str, roles: tuple[str, ...], dialects: tuple[str, ...]
) -> str | None:
    """Say how a workspace crate's name breaks the naming policy for its folder, if it does."""
    folder_parts: list[str] = directory.split("/")
    if folder_parts[-1] != name:
        return f"crate `{name}` lives in `{directory}`; name the folder after the crate"
    if folder_parts[0] == TOOLS_ROOT:
        if not name.startswith(f"{CRATE_PREFIX}{CRATE_SEPARATOR}") or not name.endswith(
            f"{CRATE_SEPARATOR}{TOOL_SUFFIX}"
        ):
            return (
                f"development tool `{name}` is not named `{CRATE_PREFIX}-<subject>-{TOOL_SUFFIX}`"
            )
        return None
    if folder_parts[0] != PRODUCT_ROOT:
        return f"crate `{name}` lives outside `{PRODUCT_ROOT}/` and `{TOOLS_ROOT}/`"
    read: CrateName | str = read_crate_name(name=name, roles=roles, dialects=dialects)
    return read if isinstance(read, str) else None


def named_dialects(*, name: str, dialects: tuple[str, ...]) -> frozenset[str]:
    """Return the dialects a crate name mentions anywhere after the prefix."""
    return frozenset(part for part in name.split(CRATE_SEPARATOR)[1:] if part in dialects)


def unnamed_dependency_dialects(
    *,
    crate: CrateDependencies,
    dialects: tuple[str, ...],
    front_doors: tuple[str, ...],
    gated: frozenset[str],
) -> tuple[tuple[str, str], ...]:
    """Return (dependency, dialect) pairs a crate uses without naming or feature-gating them."""
    if crate.name in front_doors or crate.directory.split("/")[0] == TOOLS_ROOT:
        return ()
    own: frozenset[str] = named_dialects(name=crate.name, dialects=dialects)
    allowed: frozenset[str] = frozenset() if own else gated
    found: list[tuple[str, str]] = []
    for dependency in sorted(set(crate.runtime_dependencies) - allowed):
        missing: frozenset[str] = named_dialects(name=dependency, dialects=dialects) - own
        found.extend((dependency, dialect) for dialect in sorted(missing))
    return tuple(found)


def parser_crate_dialects(
    *, crate_names: tuple[str, ...], dialects: tuple[str, ...]
) -> tuple[str, ...]:
    """Return the dialect of every sqlquack-syntax-<dialect> parser crate."""
    prefix: str = f"{CRATE_PREFIX}{CRATE_SEPARATOR}{SYNTAX_ROLE}{CRATE_SEPARATOR}"
    return tuple(
        sorted(
            name.removeprefix(prefix)
            for name in crate_names
            if name.startswith(prefix) and name.removeprefix(prefix) in dialects
        )
    )


def variant_map(*, entries: tuple[str, ...]) -> dict[str, str]:
    """Read dialect=Variant option entries into a dialect to variant mapping."""
    mapping: dict[str, str] = {}
    for entry in entries:
        dialect, _, variant = entry.partition(VARIANT_SEPARATOR)
        mapping[dialect.strip()] = variant.strip()
    return mapping


def _blank(text: str) -> str:
    return re.sub(r"[^\n]", " ", text)


def _block_comment_end(*, source: str, start: int) -> int:
    """Return where the block comment opening at `start` closes; Rust block comments nest."""
    depth: int = 0
    for mark in BLOCK_COMMENT_MARK.finditer(source, start):
        depth += 1 if mark.group(0) == BLOCK_COMMENT_OPEN else -1
        if depth == 0:
            return mark.end()
    return len(source)


def _literal_end(*, source: str, start: int) -> int | None:
    """Return where the string or char literal starting at `start` ends, or None if none does."""
    if start > 0 and IDENTIFIER_CHARACTER.match(source, start - 1):
        return None
    raw: re.Match[str] | None = RAW_STRING_START.match(source, start)
    if raw is not None:
        close: int = source.find('"' + raw.group(1), raw.end())
        return len(source) if close < 0 else close + 1 + len(raw.group(1))
    literal: re.Match[str] | None = QUOTED_STRING.match(source, start) or CHAR_LITERAL.match(
        source, start
    )
    return None if literal is None else literal.end()


def _lexeme(*, source: str, start: int) -> tuple[int, bool] | None:
    """Return the end of the comment or literal at `start` and whether it is a comment."""
    if source.startswith(LINE_COMMENT_OPEN, start):
        end: int = source.find("\n", start)
        return (len(source) if end < 0 else end), True
    if source.startswith(BLOCK_COMMENT_OPEN, start):
        return _block_comment_end(source=source, start=start), True
    literal: int | None = _literal_end(source=source, start=start)
    return None if literal is None else (literal, False)


def _blanked(*, source: str, strings: bool) -> str:
    """Blank out comments, and string and char literals when `strings`, keeping line breaks."""
    parts: list[str] = []
    index: int = 0
    while (found := LEXEME_START.search(source, index)) is not None:
        lexeme: tuple[int, bool] | None = _lexeme(source=source, start=found.start())
        if lexeme is None:
            parts.append(source[index : found.start() + 1])
            index = found.start() + 1
            continue
        end, comment = lexeme
        text: str = source[found.start() : end]
        parts.extend([source[index : found.start()], _blank(text) if comment or strings else text])
        index = end
    parts.append(source[index:])
    return "".join(parts)


def strip_comments(*, source: str) -> str:
    """Blank out comments, keeping line breaks so line numbers hold."""
    return _blanked(source=source, strings=False)


def strip_comments_and_strings(*, source: str) -> str:
    """Blank out comments and string and char literals, keeping line breaks so line numbers hold."""
    return _blanked(source=source, strings=True)


def enum_variants(*, source: str, name: str) -> tuple[str, ...] | None:
    """Return the variant names of `enum <name>` in Rust source, or None when it is not there."""
    code: str = strip_comments_and_strings(source=source)
    found: re.Match[str] | None = re.search(ENUM_BODY.format(name=re.escape(name)), code)
    if found is None:
        return None
    return tuple(VARIANT.findall(found.group("body")))


def enum_mismatch(
    *,
    variants: tuple[str, ...],
    parser_dialects: tuple[str, ...],
    variants_by_dialect: dict[str, str],
) -> tuple[tuple[str, ...], tuple[str, ...]]:
    """Return (missing, extra) variants of the Dialect enum against the parser crates."""
    expected: frozenset[str] = frozenset(
        variants_by_dialect.get(dialect, dialect) for dialect in parser_dialects
    )
    present: frozenset[str] = frozenset(variants)
    return tuple(sorted(expected - present)), tuple(sorted(present - expected))


def dialect_path_parts(*, parts: tuple[str, ...], dialects: tuple[str, ...]) -> tuple[str, ...]:
    """Return the dialects a path names in any folder or file name segment."""
    words: set[str] = set()
    for part in parts:
        stem: str = part.rsplit(".", 1)[0]
        words.update(re.split(r"[_\-.]", stem))
    return tuple(sorted(words & set(dialects)))


def dialect_branches(
    *, source: str, enum_name: str, variants: tuple[str, ...]
) -> tuple[SourceLine, ...]:
    """Return the code lines that name a specific variant of the dialect enum."""
    code: str = strip_comments_and_strings(source=source)
    pattern: re.Pattern[str] = re.compile(
        rf"\b{re.escape(enum_name)}::({'|'.join(re.escape(variant) for variant in variants)})\b"
    )
    return tuple(
        SourceLine(line=number, text=line.strip())
        for number, line in enumerate(code.split("\n"), start=1)
        if variants and pattern.search(line)
    )


def entry_words(*, entry: str) -> tuple[str, ...]:
    """Split a script module or package name into its words."""
    return tuple(entry.removesuffix(PYTHON_SUFFIX).split(SCRIPT_SEPARATOR))


def unnamed_defaults(*, entries: tuple[str, ...], dialects: tuple[str, ...]) -> tuple[str, ...]:
    """Return entries that leave a dialect unnamed while a sibling names one for the same job."""
    dialect_set: frozenset[str] = frozenset(dialects)
    shapes_with_dialect: set[tuple[str, ...]] = set()
    plain: dict[tuple[str, ...], str] = {}
    for entry in entries:
        words: tuple[str, ...] = entry_words(entry=entry)
        shape: tuple[str, ...] = tuple(word for word in words if word not in dialect_set)
        if len(shape) == len(words):
            plain[shape] = entry
        else:
            shapes_with_dialect.add(shape)
    return tuple(sorted(entry for shape, entry in plain.items() if shape in shapes_with_dialect))


def catalog_spellings(*, entries: tuple[str, ...]) -> tuple[str, ...]:
    """Return entries that spell the documented catalogue as `catalog`."""
    return tuple(sorted(entry for entry in entries if CATALOG_WORD in entry_words(entry=entry)))


def crate_module(*, parts: tuple[str, ...]) -> tuple[str, str]:
    """Return (section, top-level module) of a path inside a crate, such as ("src", "parser")."""
    section: str = parts[0]
    module: str = parts[1] if len(parts) > NESTED_PATH_PARTS else parts[-1].rsplit(".", 1)[0]
    return section, module
