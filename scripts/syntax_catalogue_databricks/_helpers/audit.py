"""The omission audit: Spark grammar sequences, keywords, helper words and the function index."""

from __future__ import annotations

import hashlib
import itertools
import tomllib
from dataclasses import dataclass
from pathlib import Path

from scripts.syntax_catalogue_databricks._helpers.fetch import page_name
from scripts.syntax_catalogue_databricks._helpers.grammar import read_keywords, read_rules
from scripts.syntax_catalogue_databricks._helpers.pages import read_page
from scripts.syntax_catalogue_databricks._helpers.strict import (
    LINE_TOKEN,
    PLACEHOLDER_NAME,
    Automaton,
    clause_index,
    line_automaton,
)
from scripts.syntax_catalogue_databricks.constants import (
    ANY_RUN,
    BLOCK_DIGEST_WIDTH,
    CATALOGUE_DIR,
    DEFINITION_PREFIX,
    EXAMPLE_HEADING,
    EXAMPLE_PROMPT,
    FUNCTION_INDEX_PAGES,
    HELPER_FILES,
    HELPER_WORD,
    HELPER_WORD_EXCLUDED_LINE,
    HELPER_WORD_LINE,
    IDENTIFIER_RULES,
    INDEX_LINK,
    KEYWORD_SYMBOL,
    LEADING_RULES,
    MANUAL_PREFIX,
    MAX_EXPANSION,
    MAX_SET,
    NAME_RULES,
    NAME_TOKENS,
    NODE_CHOICE,
    NODE_OPTIONAL,
    NODE_PLUS,
    NODE_REF,
    NODE_SEQ,
    NODE_STAR,
    NODE_TOKEN,
    PAGES_DIR,
    PUNCTUATION_TOKENS,
    QUOTED_TEXT,
    SITE,
    SPARK_DIR,
    START_RULES,
    STRUCTURE_SYMBOLS,
    SYNONYM_TOKENS,
    SYNTAX_NOTATION,
)
from scripts.syntax_catalogue_databricks.exceptions import DatabricksCatalogueError
from scripts.syntax_catalogue_databricks.types import Node, Sequences
from scripts.syntax_obligations_databricks.constants import (
    EXPRESSION_CONTEXT,
    FAMILY_CONTEXTS,
    FORM_CONTEXTS,
    VALUE_CONTEXT,
)

TOKEN_SYNONYMS: dict[str, str] = {
    token: target for token, target in SYNONYM_TOKENS.items() if target.isalpha()
}


@dataclass(frozen=True)
class AuditGap:
    """Keyword sequences of one rule alternative that no catalogued line reads."""

    rule: str
    alternative: int
    keywords: str


@dataclass(frozen=True)
class CatalogueLines:
    """Each catalogued line's automaton and every upper-case word the catalogue spells."""

    automata: list[tuple[str, Automaton, int]]
    words: frozenset[str]


@dataclass(frozen=True)
class AuditResult:
    """What the audit found and how much it checked."""

    gaps: list[AuditGap]
    derived_gaps: int
    missing_keywords: list[str]
    unused_keywords: list[str]
    missing_words: list[str]
    missing_index: list[str]
    notation_blocks: list[str]
    walked: list[str]
    unwalked: list[str]
    uncompared_tokens: list[str]
    sequences_checked: int
    productions_checked: int
    keywords_checked: int
    words_checked: int
    index_checked: int
    blocks_checked: int


def catalogue_rows(root: Path) -> tuple[list[dict[str, object]], list[dict[str, object]]]:
    """Every form and definition row of the catalogue, in file order."""
    forms: list[dict[str, object]] = []
    definitions: list[dict[str, object]] = []
    for path in sorted((root / CATALOGUE_DIR).glob("*.toml")):
        data: dict[str, list[dict[str, object]]] = tomllib.loads(path.read_text())
        forms += data.get("form", [])
        definitions += data.get("definition", [])
    return forms, definitions


def statement_context(form: dict[str, object]) -> tuple[str, str]:
    """The text around a form in the statement its cases run in, as the obligations write it.

    The parentheses of the expression context only keep a word left at the end of a negative
    from being read as an alias, and are no part of the form: the context's closing parenthesis
    would otherwise close a sequence the form leaves open, such as FIRST ( … IGNORE NULLS ).
    """
    form_id: str = str(form["id"])
    owners: list[str] = [key for key in FORM_CONTEXTS if form_id.startswith(key)]
    context: tuple[str, str] = (
        FORM_CONTEXTS[max(owners, key=len)]
        if owners
        else FAMILY_CONTEXTS.get(str(form["family"]), ("", ""))
    )
    return VALUE_CONTEXT if context == EXPRESSION_CONTEXT else context


def catalogue_lines(root: Path) -> CatalogueLines:
    """Automata of every form's lines, definitions inlined; and every word the catalogue spells."""
    forms, definitions = catalogue_rows(root)
    by_page: dict[str, dict[str, list[str]]] = {}
    for row in definitions:
        body: str = DEFINITION_PREFIX.sub("", str(row["grammar"]), count=1)
        by_page.setdefault(str(row["source"]), {}).setdefault(str(row["name"]), []).append(body)
    joined: dict[str, dict[str, str]] = {
        page: {n: b[0] if len(b) == 1 else "{ " + " | ".join(b) + " }" for n, b in found.items()}
        for page, found in by_page.items()
    }
    everywhere: dict[str, list[str]] = {}
    for found in joined.values():
        for name, body in found.items():
            everywhere.setdefault(name, []).append(body)
    common: dict[str, str] = {
        n: b[0] if len(b) == 1 else "{ " + " | ".join(b) + " }" for n, b in everywhere.items()
    }
    automata: list[tuple[str, Automaton, int]] = []
    words: set[str] = set()
    clauses: dict[tuple[str, ...], list[list[str]]] = clause_index(
        [str(line) for form in forms for line in form["grammar"]]  # type: ignore[attr-defined]
    )
    for form in forms:
        own: dict[str, str] = joined.get(str(form["source"]), {})
        prefix, suffix = statement_context(form)
        for index, line in enumerate(form["grammar"]):  # type: ignore[arg-type]
            for label, text in ((f"{form['id']}#{index}", str(line)),) + (
                ((f"{form['id']}#{index}+context", f"{prefix}{line}{suffix}"),)
                if prefix or suffix
                else ()
            ):
                automaton, start = line_automaton(
                    line=text, own=own, definitions=common, clauses=clauses
                )
                automata.append((label, automaton, start))
            words |= {t for t in LINE_TOKEN.findall(str(line)) if KEYWORD_SYMBOL.match(t)}
    for row in definitions:
        words |= {t for t in LINE_TOKEN.findall(str(row["grammar"])) if KEYWORD_SYMBOL.match(t)}
    return CatalogueLines(automata=automata, words=frozenset(words))


def concatenations(parts: list[Sequences]) -> Sequences:
    """Every way to pick one sequence from each part, joined in order; ANY_RUN runs collapsed."""
    joined: Sequences = [()]
    for part in parts:
        joined = list(dict.fromkeys(squeezed(head + tail) for head in joined for tail in part))
        if len(joined) > MAX_EXPANSION:
            raise DatabricksCatalogueError(f"a production spells more than {MAX_EXPANSION} runs")
    return joined


def squeezed(sequence: tuple[str, ...]) -> tuple[str, ...]:
    """The sequence with runs of ANY_RUN collapsed to one."""
    return tuple(
        word
        for i, word in enumerate(sequence)
        if word != ANY_RUN or sequence[i - 1 : i] != (ANY_RUN,)
    )


class _Speller:
    """Keyword sequences of grammar trees; rules that spell only keywords are inlined."""

    def __init__(self, *, rules: dict[str, Node], keywords: dict[str, tuple[str, ...]]) -> None:
        self.rules: dict[str, Node] = rules
        self.keywords: dict[str, tuple[str, ...]] = keywords
        self.memo: dict[str, Sequences | None] = {}

    def token(self, name: str) -> Sequences:
        name = TOKEN_SYNONYMS.get(name, name)
        if name in self.keywords:
            return [(word,) for word in self.keywords[name]]
        if name in PUNCTUATION_TOKENS:
            return [(spelling,) for spelling in PUNCTUATION_TOKENS[name]]
        return [()]

    def keyword_only(self, name: str) -> Sequences | None:
        """A rule's sequences when it spells only keywords in at most MAX_SET ways, else None."""
        if name in IDENTIFIER_RULES or name in NAME_RULES:
            return [()]
        if name in self.memo:
            return self.memo[name]
        self.memo[name] = None
        try:
            found: Sequences | None = self.spell(node=self.rules[name], strict=True)
        except DatabricksCatalogueError:
            found = None
        if found is not None and len(found) > MAX_SET:
            found = None
        self.memo[name] = found
        return found

    def spell(self, *, node: Node, strict: bool) -> Sequences | None:
        """The node's sequences; with `strict`, None when a rule in it spells more than keywords."""
        kind: object = node[0]
        if kind == NODE_TOKEN:
            return self.token(str(node[1]))
        if kind == NODE_REF:
            inner: Sequences | None = self.keyword_only(str(node[1]))
            if inner is None:
                return None if strict else [(ANY_RUN,)]
            return inner
        if kind == NODE_SEQ:
            parts: list[Sequences] = []
            for item in node[1]:  # type: ignore[union-attr]
                found: Sequences | None = self.spell(node=item, strict=strict)
                if found is None:
                    return None
                parts.append(found)
            return concatenations(parts)
        if kind == NODE_CHOICE:
            out: Sequences = []
            for item in node[1]:  # type: ignore[union-attr]
                found = self.spell(node=item, strict=strict)
                if found is None:
                    return None
                out += found
            return list(dict.fromkeys(out))
        if kind in (NODE_OPTIONAL, NODE_STAR, NODE_PLUS):
            found = self.spell(node=node[1], strict=strict)  # type: ignore[arg-type]
            if found is None:
                return None
            return list(dict.fromkeys(found if kind == NODE_PLUS else [(), *found]))
        return [()]


def alternatives(node: Node) -> tuple[Node, ...]:
    """A rule's top-level alternatives."""
    return node[1] if node[0] == NODE_CHOICE else (node,)  # type: ignore[return-value]


def references(node: Node) -> list[str]:
    """Every rule a tree names."""
    kind: object = node[0]
    if kind == NODE_REF:
        return [str(node[1])]
    if kind in (NODE_SEQ, NODE_CHOICE):
        return list(itertools.chain.from_iterable(references(n) for n in node[1]))  # type: ignore[union-attr]
    if kind in (NODE_OPTIONAL, NODE_STAR, NODE_PLUS):
        return references(node[1])  # type: ignore[arg-type]
    return []


def tokens_of(node: Node) -> set[str]:
    """Every token a tree names."""
    kind: object = node[0]
    if kind == NODE_TOKEN:
        return {str(node[1])}
    if kind in (NODE_SEQ, NODE_CHOICE):
        return set(itertools.chain.from_iterable(tokens_of(n) for n in node[1]))  # type: ignore[union-attr]
    if kind in (NODE_OPTIONAL, NODE_STAR, NODE_PLUS):
        return tokens_of(node[1])  # type: ignore[arg-type]
    return set()


def walked_rules(rules: dict[str, Node]) -> list[str]:
    """Every rule reachable from the entry rules, breadth first; the identifier lists are not."""
    order: list[str] = [name for name in START_RULES if name in rules]
    if len(order) != len(START_RULES):
        raise DatabricksCatalogueError("an entry rule is missing from the grammar")
    seen: set[str] = set(order)
    frontier: list[str] = list(order)
    while frontier:
        for found in references(rules[frontier.pop(0)]):
            if found not in seen and found not in IDENTIFIER_RULES:
                seen.add(found)
                order.append(found)
                frontier.append(found)
    return order


def descendants(*, rules: dict[str, Node], name: str) -> set[str]:
    seen: set[str] = set()
    frontier: list[str] = [name]
    while frontier:
        for found in references(rules[frontier.pop()]):
            if found not in seen and found in rules:
                seen.add(found)
                frontier.append(found)
    return seen


def split_derived(
    *, gaps: list[AuditGap], rules: dict[str, Node], own: frozenset[str]
) -> tuple[list[AuditGap], int]:
    """Root gaps and the count of derived ones; a gap `own` names stays a root."""
    by_rule: dict[str, list[tuple[str, ...]]] = {}
    for gap in gaps:
        by_rule.setdefault(gap.rule, []).append(tuple(gap.keywords.split()))
    roots: list[AuditGap] = []
    derived: int = 0
    below: dict[str, set[tuple[str, ...]]] = {}
    for gap in gaps:
        words: tuple[str, ...] = tuple(gap.keywords.split())
        if gap.rule not in below:
            below[gap.rule] = {
                run
                for rule in descendants(rules=rules, name=gap.rule) - {gap.rule}
                for run in by_rule.get(rule, [])
            }
        pieces: set[tuple[str, ...]] = {
            words[i:j] for i in range(len(words)) for j in range(i + 1, len(words) + 1)
        }
        if f"{gap.rule}: {gap.keywords}" not in own and not pieces.isdisjoint(below[gap.rule]):
            derived += 1
        else:
            roots.append(gap)
    return roots, derived


def gap_walk(
    *,
    rules: dict[str, Node],
    keywords: dict[str, tuple[str, ...]],
    lines: CatalogueLines,
    walked: list[str],
) -> tuple[list[AuditGap], int, int]:
    """Every walked alternative's keyword sequences no catalogued line reads, and the counts."""
    speller: _Speller = _Speller(rules=rules, keywords=keywords)
    holding: dict[str, set[int]] = {}
    for position, (_, automaton, _) in enumerate(lines.automata):
        for label in {label for edges in automaton.edges for label, _ in edges}:
            holding.setdefault(label, set()).add(position)
    every: set[int] = set(range(len(lines.automata)))
    known: dict[tuple[bool, tuple[str, ...]], bool] = {}

    def read(sequence: tuple[str, ...], leading: bool) -> bool:
        """True when a line holding every symbol of the sequence reads it; memoised."""
        key: tuple[bool, tuple[str, ...]] = (leading, sequence)
        if key not in known:
            candidates: set[int] = set(every)
            for symbol in set(sequence) - {ANY_RUN}:
                candidates &= holding.get(symbol, set())
            known[key] = any(
                lines.automata[i][1].reads(
                    sequence=sequence, start=lines.automata[i][2] if leading else None
                )
                for i in sorted(candidates)
            )
        return known[key]

    gaps: list[AuditGap] = []
    checked: int = 0
    productions: int = 0
    for name in walked:
        for index, alternative in enumerate(alternatives(rules[name]), 1):
            productions += 1
            try:
                spelled: Sequences | None = speller.spell(node=alternative, strict=False)
            except DatabricksCatalogueError as error:
                raise DatabricksCatalogueError(f"rule {name} #{index}: {error}") from error
            for sequence in spelled or []:
                if not any(
                    KEYWORD_SYMBOL.match(word) or word not in STRUCTURE_SYMBOLS for word in sequence
                ):
                    continue
                checked += 1
                if not read(sequence, name in LEADING_RULES):
                    gaps.append(AuditGap(rule=name, alternative=index, keywords=" ".join(sequence)))
    return gaps, checked, productions


def helper_words(cache: Path) -> list[str]:
    """Words the parser helpers compare identifiers or option text with, upper-cased."""
    found: set[str] = set()
    for name in HELPER_FILES:
        for line in (cache / SPARK_DIR / Path(name).name).read_text().splitlines():
            if HELPER_WORD_LINE.search(line) and not HELPER_WORD_EXCLUDED_LINE.search(line):
                found |= {word.upper() for word in HELPER_WORD.findall(line)}
    return sorted(found)


def index_pages(*, cache: Path, record: dict[str, object]) -> tuple[list[str], int]:
    """Function pages the alphabetical indexes link that the pinned page set lacks."""
    pages: dict[str, str] = record["pages"]  # type: ignore[assignment]
    linked: set[str] = set()
    for index in FUNCTION_INDEX_PAGES:
        text: str = (cache / PAGES_DIR / f"{index}.html").read_text()
        linked |= {SITE + path.rstrip("/") for path in INDEX_LINK.findall(text)}
    missing: list[str] = sorted(
        url.removeprefix(MANUAL_PREFIX) for url in linked if url not in pages
    )
    return missing, len(linked)


def notation_blocks(*, cache: Path, record: dict[str, object]) -> tuple[list[str], int]:
    """Code blocks the build does not read that write syntax notation, and how many it read.

    Each is named `<page> § <heading> § <sha256 prefix of its text>`, so a changed block is a new
    name and its old review row goes stale.
    """
    pages: dict[str, str] = record["pages"]  # type: ignore[assignment]
    found: list[str] = []
    checked: int = 0
    for url in sorted(pages):
        try:
            page = read_page(cache=cache, url=url)
        except DatabricksCatalogueError:
            continue
        for block in page.others:
            checked += 1
            if EXAMPLE_HEADING.match(block.heading) or EXAMPLE_PROMPT.search(block.text):
                continue
            if SYNTAX_NOTATION.search(QUOTED_TEXT.sub("''", block.text)):
                digest: str = hashlib.sha256(block.text.encode()).hexdigest()[:BLOCK_DIGEST_WIDTH]
                found.append(f"{page.path} § {block.heading} § {digest}")
    return sorted(set(found)), checked


def audit(
    *, cache: Path, root: Path, record: dict[str, object], own: frozenset[str] = frozenset()
) -> AuditResult:
    """Grammar sequences, keywords, helper words and index entries the catalogue lacks."""
    rules: dict[str, Node] = read_rules(cache)
    keywords: dict[str, tuple[str, ...]] = read_keywords(cache)
    lines: CatalogueLines = catalogue_lines(root)
    walked: list[str] = walked_rules(rules)
    gaps, checked, productions = gap_walk(
        rules=rules, keywords=keywords, lines=lines, walked=walked
    )
    roots, derived = split_derived(gaps=gaps, rules=rules, own=own)
    used: set[str] = set(itertools.chain.from_iterable(tokens_of(rules[n]) for n in walked))
    words: list[str] = helper_words(cache)
    missing_index, linked = index_pages(cache=cache, record=record)
    blocks, blocks_checked = notation_blocks(cache=cache, record=record)
    return AuditResult(
        gaps=roots,
        derived_gaps=derived,
        missing_keywords=sorted(
            word
            for token, spelled in keywords.items()
            if token in used
            for word in spelled
            if word not in lines.words
        ),
        unused_keywords=sorted(
            word
            for token, spelled in keywords.items()
            if token not in used
            for word in spelled
            if word not in lines.words
        ),
        missing_words=[word for word in words if word not in lines.words],
        missing_index=missing_index,
        notation_blocks=blocks,
        walked=walked,
        unwalked=sorted(set(rules) - set(walked) - IDENTIFIER_RULES),
        uncompared_tokens=sorted(
            token
            for token in used
            if token not in keywords
            and token not in PUNCTUATION_TOKENS
            and token not in SYNONYM_TOKENS
            and token not in NAME_TOKENS
            and token not in STRUCTURE_SYMBOLS
        ),
        sequences_checked=checked,
        productions_checked=productions,
        keywords_checked=sum(len(spelled) for spelled in keywords.values()),
        words_checked=len(words),
        index_checked=linked,
        blocks_checked=blocks_checked,
    )


__all__ = ["PLACEHOLDER_NAME", "AuditGap", "AuditResult", "audit", "page_name"]
