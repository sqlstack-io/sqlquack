"""Split one Snowflake documentation page (Markdown) into its documented syntax productions.

The rules are mechanical so the catalogue can be regenerated when the documentation changes and
the result compared form by form; coverage/syntax/README.md states them.
"""

from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass, field

FENCE = re.compile(r"^\s*(```+|~~~+)\s*([\w+-]*)\s*$")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")
DEFINITION = re.compile(r"^\s*<?([A-Za-z_][\w ]*?)>?\s*(\([^)]*\))?\s*::=")
WORD = re.compile(r"[A-Za-z_][A-Za-z0-9_$]*")
ELISION_LINE = re.compile(r"^\s*(\[\s*)?\.\.\.(\s*\])?\s*$")
OPENERS = {"[": "]", "{": "}", "(": ")"}
CLOSERS = {v: k for k, v in OPENERS.items()}
# Headings under which a syntax block documents something other than a statement form.
NON_FORM_SECTIONS = re.compile(
    r"^(parameters|required parameters|optional parameters|arguments|returns|usage notes|"
    r"examples?|output|access control|.*\bparameters\b.*|.*\bexamples?\b.*|.*usage notes.*|"
    r"collation details|error handling|.*\bactions\b.*\(`.*`\))$",
    re.I,
)
# Keywords that, outside placeholders, give a function its own grammar instead of a plain call.
SPECIAL_WORDS = {
    "AS",
    "FROM",
    "IN",
    "FOR",
    "OVER",
    "WITHIN",
    "IGNORE",
    "RESPECT",
    "DISTINCT",
    "BOTH",
    "LEADING",
    "TRAILING",
    "USING",
    "ORDER",
    "PARTITION",
    "AT",
    "BEFORE",
    "BY",
    "ON",
    "INTO",
    "ALL",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "IS",
    "NOT",
    "NULL",
    "CASE",
    "PLACING",
    "SEPARATOR",
    "RETURNING",
    "WITH",
    "KEEP",
    "FILTER",
}


def sha256(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def slug(text: str) -> str:
    return re.sub(r"-+", "-", re.sub(r"[^a-z0-9]+", "-", text.lower())).strip("-")


@dataclass
class Block:
    heading: tuple[str, ...]
    text: str
    elided: bool


@dataclass
class Page:
    title: str
    headings: list[tuple[int, str]]
    blocks: list[Block]
    syntax_sections_without_block: list[str]


@dataclass
class Production:
    kind: str  # statement | definition | fragment
    grammar: str
    heading: tuple[str, ...]
    name: str | None = None
    notes: list[str] = field(default_factory=list)
    elaborations: list[str] = field(default_factory=list)


def parse_page(markdown: str) -> Page:
    """Headings and fenced `sqlsyntax` blocks, each with the heading path above it."""
    title = ""
    path: list[tuple[int, str]] = []
    blocks: list[Block] = []
    headings: list[tuple[int, str]] = []
    fence = None
    buf: list[str] = []
    heading_has_block: dict[tuple[str, ...], bool] = {}
    for line in markdown.splitlines():
        m = FENCE.match(line)
        if fence is None and m:
            fence = (m.group(1)[:3], (m.group(2) or "plain").lower())
            buf = []
            continue
        if fence is not None:
            if m and m.group(1)[:3] == fence[0] and not m.group(2):
                if fence[1] == "sqlsyntax":
                    key = tuple(h for _, h in path)
                    elided = any(ELISION_LINE.match(b) for b in buf)
                    blocks.append(Block(key, "\n".join(buf), elided))
                    heading_has_block[key] = True
                fence = None
            else:
                buf.append(line)
            continue
        h = HEADING.match(line)
        if h:
            level, text = len(h.group(1)), h.group(2).strip()
            if level == 1 and not title:
                title = text
            path = [(lv, t) for lv, t in path if lv < level] + [(level, text)]
            headings.append((level, text))
            heading_has_block.setdefault(tuple(t for _, t in path), False)
    # A heading inside a Syntax/Variant syntax section with no block of its own or below it.
    missing = []
    for key, has in heading_has_block.items():
        if has or len(key) < 3 or not re.search(r"syntax", key[1], re.I):
            continue
        if not any(k[: len(key)] == key and v for k, v in heading_has_block.items()):
            missing.append(key[-1])
    return Page(title, headings, blocks, missing)


def strip_comments(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    return "\n".join(re.sub(r"(^|\s)--.*$", "", ln) for ln in text.splitlines())


def split_productions(block: str) -> list[str]:
    """Split a block at blank lines, and before each `name ::=`, where every bracket is closed."""
    out: list[str] = []
    cur: list[str] = []
    depth = 0
    for raw in strip_comments(block).splitlines():
        line = raw.rstrip()
        if (not line.strip() or DEFINITION.match(line)) and depth <= 0 and cur:
            out.append("\n".join(cur))
            cur, depth = [], 0
        if not line.strip():
            continue
        cur.append(line)
        for ch in re.sub(r"'[^']*'|<[^<>]*>", "", line):
            if ch in OPENERS:
                depth += 1
            elif ch in CLOSERS:
                depth -= 1
    if cur:
        out.append("\n".join(cur))
    return [n for n in (normalize(p) for p in out) if n]


def normalize(text: str) -> str:
    """The grammar on one line with single spaces: the normalized grammar the catalogue tracks."""
    return re.sub(r"\s+", " ", text).strip()


def bare_words(grammar: str) -> list[str]:
    """Words outside placeholders and quotes, in order."""
    return WORD.findall(re.sub(r"<[^<>]*>|'[^']*'|\"[^\"]*\"", " ", grammar))


def required_skeleton(grammar: str) -> str:
    """The grammar without optional `[ ... ]` parts, placeholders and quoted text."""
    g = re.sub(r"<[^<>]*>|'[^']*'|\"[^\"]*\"", " ", grammar)
    out, depth = [], 0
    for ch in g:
        if ch == "[":
            depth += 1
        elif ch == "]":
            depth = max(0, depth - 1)
        elif depth == 0:
            out.append(ch)
    return normalize("".join(out))


def call_name(grammar: str) -> str:
    m = re.match(r"^\s*([A-Za-z_][\w$.]*)\s*\(", grammar)
    return m.group(1).upper() if m else ""


SYMBOLS = {
    "::": "double-colon",
    "||": "concat",
    "->": "arrow",
    "=>": "named-arg",
    ":": "colon",
    "[": "subscript",
    ".": "dot",
    "!": "bang",
}


def symbol_name(grammar: str) -> str:
    """A name for a form with no words outside placeholders, from its operator symbols."""
    g = re.sub(r"<[^<>]*>|'[^']*'", " ", grammar)
    names = []
    for k, v in SYMBOLS.items():
        if k in g:
            names.append(v)
            g = g.replace(k, " ")
    return "-".join(names)


def is_special_call(grammar: str) -> bool:
    """True when a function's syntax has its own grammar rather than a plain call."""
    if "=>" in grammar or "::" in grammar:
        return True
    m = re.match(r"^\s*([A-Za-z_][\w$.]*)\s*\(", grammar)
    if not m:
        return True
    return any(w in SPECIAL_WORDS for w in bare_words(grammar[m.end() :]) if w.isupper())


def in_form_section(heading: tuple[str, ...]) -> bool:
    return not any(NON_FORM_SECTIONS.match(h) for h in heading[1:])


def productions(page: Page, leading_words: set[str] | None) -> list[Production]:
    """Classify each production.

    - `name ::= ...` is a definition; a following chunk that is neither a definition nor a
      statement continues it (definitions written with blank lines between alternatives).
    - With leading_words (command pages), a statement must start with one of them.
    - A block outside a syntax section (parameters, usage notes, examples) yields fragments.
    - A block that elides with a `...` line, under the same heading as an earlier statement,
      elaborates that statement rather than documenting a new form.
    """
    result: list[Production] = []
    for block in page.blocks:
        last: Production | None = None
        for text in split_productions(block.text):
            d = DEFINITION.match(text)
            if d:
                last = Production("definition", text, block.heading, name=d.group(1).strip())
                result.append(last)
                continue
            first = (bare_words(text) or [""])[0].upper()
            starts = leading_words is None or first in leading_words
            # Within one block, a definition runs until the next definition, and a statement
            # continues over blank lines until a chunk starts a new statement.
            if last is not None and (last.kind == "definition" or not starts):
                last.grammar += " " + text
                continue
            if not in_form_section(block.heading) or not starts:
                last = Production("fragment", text, block.heading)
                result.append(last)
                continue
            earlier = [p for p in result if p.kind == "statement" and p.heading == block.heading]
            if block.elided and earlier:
                earlier[-1].elaborations.append(text)
                earlier[-1].notes.append("elided block under the same heading merged")
                last = earlier[-1]
                continue
            last = Production("statement", text, block.heading)
            result.append(last)
    return result
