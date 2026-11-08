"""Turn a page's blocks and tables into catalogue productions: statements, calls, definitions."""

from __future__ import annotations

import re

from scripts.syntax_catalogue._helpers.grammar import normalize, slug
from scripts.syntax_catalogue.constants import KIND_CALL, KIND_SPECIAL
from scripts.syntax_catalogue.models import FormGroup
from scripts.syntax_catalogue_bigquery._helpers.notation import convert, productions
from scripts.syntax_catalogue_bigquery.constants import (
    ANGLE_LITERALS,
    ARGUMENT_BULLET,
    ARRAY_ARGUMENT,
    BRACKET_OPENERS,
    BRACKET_PAIRS,
    CALL_DEFAULT,
    CALL_PRODUCTION,
    CAMEL_PLACEHOLDER,
    CODE_SPAN,
    CODE_TICK,
    COMMA_BEFORE_OPTIONAL,
    DOUBLED_COMMA,
    GRAPH_HEADING,
    IS_PREFIX,
    LOWER_KEYWORD_PLACEHOLDERS,
    MISPAIRED_QUOTE,
    ML_PREFIX,
    OPERATOR_NAMES,
    OPERATOR_SHAPES,
    OPERATOR_TABLE_CELLS,
    OPERATOR_TABLE_HEADER,
    PAREN_CLOSE,
    PAREN_DEPTH,
    PLACEHOLDER_OPEN,
    QUOTED,
    STANDARD_SQL,
    STRUCT_FIELD,
    STRUCT_FIELD_FRAME,
    STRUCT_FIELDS,
    STRUCT_OPEN,
    SUSPECT_TEXT,
    SYNTAX_COLUMN,
    TABLE_CELL,
    TYPE_NAME,
    TYPE_TABLE_CELLS,
    TYPE_TEMPLATES,
    UNARY_ARITY,
)
from scripts.syntax_catalogue_bigquery.models import BigQueryPage, Located


def page_placeholders(page: BigQueryPage) -> frozenset[str]:
    """Upper-case argument names a BigQuery ML page lists as `- `NAME`:` bullets."""
    if not page.path.removeprefix(STANDARD_SQL).startswith(ML_PREFIX):
        return frozenset()
    return frozenset(ARGUMENT_BULLET.findall(page.text))


def converted(*, text: str, placeholders: frozenset[str]) -> list[tuple[str, str]]:
    """(definition name or "", grammar) of each production of a block, in catalogue notation."""
    return [
        (
            name,
            array_arguments(
                struct_fields(single_commas(convert(text=body, placeholders=placeholders)))
            ),
        )
        for name, body in productions(text)
    ]


def struct_fields(grammar: str) -> str:
    """BigQuery ML `STRUCT ( NAME AS name )` read as the value NAME under the field name NAME."""
    out: list[str] = []
    rest: str = grammar
    while STRUCT_OPEN in rest:
        before, rest = rest.split(STRUCT_OPEN, 1)
        depth: int = 1
        end: int = 0
        while depth and end < len(rest):
            depth += PAREN_DEPTH.get(rest[end], 0)
            end += 1
        span: str = rest[:end]
        for pattern, replacement in STRUCT_FIELDS:
            span = pattern.sub(lambda m, r=replacement: r.format(m[1], m[2].upper()), span)
        out.append(f"{before}{STRUCT_OPEN}{field_list(span)}")
        rest = rest[end:]
    return "".join(out) + rest


def field_list(span: str) -> str:
    """A STRUCT of optional fields, however the page groups them, as a `[ { A | B } [ , ... ] ]`."""
    inner: str = span.removesuffix(PAREN_CLOSE).strip()
    items: list[str] = STRUCT_FIELD.findall(inner)
    if STRUCT_FIELD_FRAME.fullmatch(STRUCT_FIELD.sub("", inner)) is None:
        return span
    leading: list[str] = []
    rest: str = inner
    for item in items:
        if not rest.startswith(item):
            break
        rest = rest.removeprefix(item).strip().removeprefix(",").strip()
        leading.append(item)
    optional: list[str] = items[len(leading) :]
    if not optional:
        return span
    listed: str = f"{{ {' | '.join(optional)} }} [ , ... ]" if len(optional) > 1 else optional[0]
    head: str = " , ".join(leading)
    return f"{head} [ , {listed} ] {PAREN_CLOSE}" if head else f"[ {listed} ] {PAREN_CLOSE}"


def array_arguments(grammar: str) -> str:
    """A named argument `=> [ <X> ]` read as an array literal: its brackets are not optional."""
    return ARRAY_ARGUMENT.sub(r'=> "[" \1 "]"', grammar)


def single_commas(grammar: str) -> str:
    """The grammar with `a , [ , b ]` and `a , [ b ]` (an optional argument) read as `a [ , b ]`."""
    grammar = DOUBLED_COMMA.sub("[ , ", grammar)
    return COMMA_BEFORE_OPTIONAL.sub("[ , ", grammar)


def suspect(*, text: str, grammars: list[tuple[str, str]]) -> str:
    """Why a block cannot be read as notation without review, or "" when it can."""
    if SUSPECT_TEXT.search(text):
        return "holds a URL, a triple-quoted string, a table or a comment block"
    if not grammars:
        return "yields no production"
    defined: set[str] = {name for name, _ in grammars if name}
    for _, grammar in grammars:
        words: list[str] = [w for w in re.findall(r"<([^<>]*)>", grammar) if w not in defined]
        mixed: list[str] = [word for word in words if CAMEL_PLACEHOLDER.match(f"<{word}")]
        if mixed:
            return f"has a mixed-case word ({mixed[0]}), as an example's names do"
        lowered: list[str] = [word for word in words if word in LOWER_KEYWORD_PLACEHOLDERS]
        if lowered:
            return f"has a lower-case keyword ({lowered[0]}), as an example does"
        if _unbalanced(grammar):
            return "has unbalanced brackets"
        if grammar.startswith("'") and grammar.endswith("'"):
            return "is one string literal, the content of a string value"
        if any(MISPAIRED_QUOTE.search(m.group(0)) for m in QUOTED.finditer(grammar)):
            return "has a string literal spanning notation, as a mistyped quote makes"
        if CALL_DEFAULT.search(grammar):
            return "writes an argument default as DEFAULT NULL"
    return ""


def _unbalanced(grammar: str) -> bool:
    """Brackets, braces or parentheses that do not pair up outside quotes and placeholders."""
    bare: str = re.sub(r"<[^<>]*>|'[^']*'|\"[^\"]*\"", "", grammar)
    stack: list[str] = []
    for ch in bare:
        if ch in BRACKET_OPENERS:
            stack.append(ch)
        elif ch in BRACKET_PAIRS:
            if not stack or stack.pop() != BRACKET_PAIRS[ch]:
                return True
    return bool(stack)


def call_name(grammar: str) -> str:
    """The upper-case (possibly dotted) name a call grammar starts with, or ""."""
    match: re.Match[str] | None = CALL_PRODUCTION.match(grammar.replace(" . ", "."))
    return match.group(1) if match else ""


def section_label(heading: tuple[str, ...]) -> str:
    """The heading a group is named by: its last heading that is not a generic one."""
    for text in reversed(heading):
        plain: str = text.replace("`", "").strip()
        if plain and not re.fullmatch(r"(?i)syntax|signature \d+|arguments|details", plain):
            return plain
    return ""


def statement_group(*, kind: str, grammar: str, heading: tuple[str, ...]) -> FormGroup:
    """One statement or special form of a single grammar line."""
    return FormGroup(kind, "", (grammar,), ("", *heading))


def call_groups(calls: dict[str, list[tuple[str, tuple[str, ...]]]]) -> list[FormGroup]:
    """One call form per function name, its signatures in first-seen order."""
    groups: list[FormGroup] = []
    for name, items in sorted(calls.items()):
        signatures: tuple[str, ...] = tuple(dict.fromkeys(grammar for grammar, _ in items))
        groups.append(FormGroup(KIND_CALL, slug(name), signatures, ("", *items[0][1])))
    return groups


def cells_of(row: str) -> list[str]:
    """A table row's cells; a `|` inside a code span stays in its cell."""
    cells: list[str] = []
    current: list[str] = []
    in_code: bool = False
    for ch in row.strip().strip(TABLE_CELL):
        if ch == CODE_TICK:
            in_code = not in_code
        if ch == TABLE_CELL and not in_code:
            cells.append("".join(current).strip())
            current = []
        else:
            current.append(ch)
    cells.append("".join(current).strip())
    return cells


def operator_grammar(*, operator: str, arity: str) -> str:
    """`<x> op <y>`, `op <x>` or `<x> IS ...` for one precedence-table operator."""
    if operator in OPERATOR_SHAPES:
        return OPERATOR_SHAPES[operator]
    spelled: str = normalize(operator.replace("[", " [ ").replace("]", " ] "))
    spelled = " ".join(f'"{t}"' if t in ANGLE_LITERALS else t for t in spelled.split())
    if arity == UNARY_ARITY and spelled.startswith(IS_PREFIX):
        return f"<x> {spelled}"
    if arity == UNARY_ARITY:
        return f"{spelled} <x>"
    return f"<x> {spelled} <y>"


def operator_groups(rows: tuple[Located, ...]) -> list[FormGroup]:
    """One special form per operator of the precedence table, unary and binary uses together."""
    tables: set[int] = {row.table for row in rows if OPERATOR_TABLE_HEADER.match(row.text)}
    found: dict[str, list[str]] = {}
    for row in rows:
        if row.table not in tables or OPERATOR_TABLE_HEADER.match(row.text):
            continue
        cells: list[str] = cells_of(row.text)
        if len(cells) < OPERATOR_TABLE_CELLS or all(
            cell and set(cell) <= set(":- ") for cell in cells
        ):
            continue
        for operator in CODE_SPAN.findall(cells[1]):
            grammar: str = operator_grammar(operator=operator, arity=cells[4].strip())
            found.setdefault(operator, []).append(grammar)
    return [
        FormGroup(
            KIND_SPECIAL, slug(f"op {_operator_name(operator)}"), tuple(dict.fromkeys(g)), ("",)
        )
        for operator, g in sorted(found.items())
    ]


def _operator_name(operator: str) -> str:
    """A slug-able name for an operator spelled with symbols."""
    return OPERATOR_NAMES.get(operator, operator)


def syntax_table_groups(*, rows: tuple[Located, ...], known: set[str]) -> list[FormGroup]:
    """One special form per Syntax-column span with an operand, unless `known` spells it."""
    columns: dict[int, int] = {}
    groups: list[FormGroup] = []
    known_keys: set[str] = set()
    for grammar in known:
        key: str = _operator_key(grammar)
        known_keys |= {key, key.replace("[ NOT ] ", ""), key.replace("[ NOT ] ", "NOT ")}
    for row in rows:
        cells: list[str] = cells_of(row.text)
        if row.table not in columns:
            names: list[str] = [cell.strip() for cell in cells]
            columns[row.table] = next(
                (i for i, name in enumerate(names) if SYNTAX_COLUMN.fullmatch(name)), -1
            )
            continue
        column: int = columns[row.table]
        if column < 0 or column >= len(cells) or all(set(c) <= set(":- ") for c in cells):
            continue
        general: bool = not any(GRAPH_HEADING.search(h) for h in row.heading)
        for span in CODE_SPAN.findall(cells[column]):
            grammar: str = convert(text=span)
            if PLACEHOLDER_OPEN not in re.sub(r'"<" <\w+> ">"', "", grammar).replace('"<"', ""):
                continue
            if general and _operator_key(grammar) in known_keys:
                continue
            label: str = slug(f"op {CODE_SPAN.sub(lambda m: m.group(1), cells[0])}")
            groups.append(FormGroup(KIND_SPECIAL, label, (grammar,), ("", *row.heading)))
    return groups


def _operator_key(grammar: str) -> str:
    """An operator grammar with lower-case placeholders and `<<`, `>>` written as one symbol."""
    key: str = re.sub(r"<(\w+)>", lambda m: f"<{m.group(1).lower()}>", grammar)
    return key.replace('"<" "<"', "<<").replace('">" ">"', ">>")


def type_groups(rows: tuple[Located, ...]) -> list[FormGroup]:
    """One special form per data type of the summary table, its aliases as alternatives."""
    groups: list[FormGroup] = []
    for row in rows:
        if row.table != 1:
            continue
        cells: list[str] = cells_of(row.text)
        if len(cells) < TYPE_TABLE_CELLS:
            continue
        for entry in TYPE_NAME.finditer(cells[1]):
            aliases: list[str] = CODE_SPAN.findall(entry.group(2) or "")
            name: str = entry.group(1)
            spelled: tuple[str, ...] = tuple(TYPE_TEMPLATES.get(n, n) for n in (name, *aliases))
            groups.append(FormGroup(KIND_SPECIAL, slug(name), spelled, ("", *row.heading)))
    return groups
