"""The omission audit: parse.y keyword sequences, keywords and engine names vs the catalogue."""

from __future__ import annotations

import itertools
import json
import re
import tomllib
from collections.abc import Iterator
from pathlib import Path
from typing import cast

from scripts.syntax_catalogue._helpers.grammar import bare_words
from scripts.syntax_catalogue_sqlite._helpers.grammar import read_grammar, read_keywords
from scripts.syntax_catalogue_sqlite._helpers.strict import line_automaton
from scripts.syntax_catalogue_sqlite.classes.automaton import Automaton
from scripts.syntax_catalogue_sqlite.constants import (
    AMALGAMATION_FILES,
    ANY_RUN,
    DIAGRAM_SOURCE,
    ENGINE_DIR,
    ENGINE_EXTRACT,
    ENGINE_NAME_WORD,
    FAMILY_FILES,
    GENERATED_RULE_COUNT,
    GROUP_CLOSERS,
    INLINE_DEPTH,
    INLINE_EXCLUDED,
    KEYWORD_WORD,
    MAX_EXPANSION,
    MAX_SET,
    NOTATION_BAR,
    OPTIONAL_OPEN,
    PLACEHOLDER,
    PUNCTUATION_TOKENS,
    STANDARD_TYPES,
    STATEMENT_RULE,
    TOKEN_ALTERNATIVE,
    TOKEN_OR_WORD,
)
from scripts.syntax_catalogue_sqlite.exceptions import SqliteCatalogueError
from scripts.syntax_catalogue_sqlite.models import (
    AuditGap,
    AuditResult,
    CatalogueWords,
    EngineName,
    Grammar,
    Keyword,
    Production,
)
from scripts.syntax_catalogue_sqlite.types import Sequences


def _group(*, tokens: list[str], start: int, closer: str) -> tuple[set[str], bool, int]:
    """The first keywords of `a | b | ...` up to `closer`, whether it can be empty, and its end."""
    first: set[str] = set()
    nullable: bool = False
    index: int = start
    while True:
        found, empty, index = _sequence(tokens=tokens, start=index, closer=closer)
        first |= found
        nullable = nullable or empty
        if index >= len(tokens) or tokens[index] != NOTATION_BAR:
            return first, nullable, index + 1
        index += 1


def _sequence(*, tokens: list[str], start: int, closer: str) -> tuple[set[str], bool, int]:
    """The first keywords of a sequence up to a bar or `closer`, whether it can be empty."""
    first: set[str] = set()
    open_: bool = True
    index: int = start
    while index < len(tokens) and tokens[index] not in (NOTATION_BAR, closer):
        token: str = tokens[index]
        if token in GROUP_CLOSERS:
            found, empty, index = _group(
                tokens=tokens, start=index + 1, closer=GROUP_CLOSERS[token]
            )
            first |= found if open_ else set()
            open_ = open_ and (empty or token == OPTIONAL_OPEN)
            continue
        if open_ and KEYWORD_WORD.match(token):
            first.add(token)
        open_ = False
        index += 1
    return first, open_, index


def lead_words(grammar: str) -> set[str]:
    """The keywords a grammar line can start with, past optional parts and into choices."""
    tokens: list[str] = TOKEN_OR_WORD.findall(grammar)
    return _sequence(tokens=tokens, start=0, closer="")[0]


def inlined(*, line: str, definitions: dict[str, str]) -> str:
    """The line with each `<name>` a syntax diagram defines replaced by its grammar."""
    for _ in range(INLINE_DEPTH):
        replaced: str = PLACEHOLDER.sub(lambda m: definitions.get(m.group(1), m.group(0)), line)
        if replaced == line:
            break
        line = replaced
    return line


def diagram_definitions(root: Path) -> dict[str, str]:
    """Each syntax diagram's grammar as one choice, by its name, but the recursive ones."""
    definitions: dict[str, list[str]] = {}
    for path in sorted((root / FAMILY_FILES).glob("*.toml")):
        for form in tomllib.loads(path.read_text()).get("form", []):
            source: str = str(form["source"])
            if not source.startswith(DIAGRAM_SOURCE):
                continue
            name: str = source.removeprefix(DIAGRAM_SOURCE)
            if name not in INLINE_EXCLUDED:
                definitions.setdefault(name, []).extend(str(g) for g in form["grammar"])
    return {
        name: lines[0] if len(lines) == 1 else "{ " + " | ".join(lines) + " }"
        for name, lines in definitions.items()
    }


def catalogue_lines(root: Path) -> list[tuple[str, str]]:
    """Every catalogued form's grammar lines, keyed `id#index`, in catalogue file order."""
    lines: list[tuple[str, str]] = []
    for path in sorted((root / FAMILY_FILES).glob("*.toml")):
        for form in tomllib.loads(path.read_text()).get("form", []):
            for index, line in enumerate(form["grammar"]):
                lines.append((f"{form['id']}#{index}", str(line)))
    return lines


def catalogue_words(*, root: Path, keywords: frozenset[str]) -> CatalogueWords:
    """Each line's words with diagrams inlined, its automaton, and every word it spells."""
    definitions: dict[str, str] = diagram_definitions(root)
    forms: list[tuple[str, list[str]]] = []
    leads: dict[str, tuple[str, ...]] = {}
    automata: list[tuple[str, Automaton, int]] = []
    every: set[str] = set()
    for key, line in catalogue_lines(root):
        expanded: str = inlined(line=line, definitions=definitions)
        forms.append((key, [word.upper() for word in bare_words(expanded)]))
        leads[key] = tuple(sorted(lead_words(line)))
        every |= {word.upper() for word in bare_words(line)}
        automaton, start = line_automaton(line=line, keywords=keywords, definitions=definitions)
        automata.append((key, automaton, start))
    return CatalogueWords(forms=forms, every=every, leads=leads, clauses=[], automata=automata)


def concatenations(parts: list[Sequences]) -> Sequences:
    """Every way to pick one sequence from each part, joined in order."""
    size: int = 1
    for part in parts:
        size *= len(part)
    if size > MAX_EXPANSION:
        raise SqliteCatalogueError(f"a production spells more than {MAX_EXPANSION} keyword runs")
    return list(
        dict.fromkeys(
            tuple(itertools.chain.from_iterable(pick)) for pick in itertools.product(*parts)
        )
    )


def token_words(keywords: list[Keyword]) -> dict[str, tuple[str, ...]]:
    """The keywords each token scans from, by token name."""
    words: dict[str, list[str]] = {}
    for keyword in keywords:
        words.setdefault(keyword.token, []).append(keyword.word)
    return {token: tuple(sorted(found)) for token, found in words.items()}


class _Speller:
    """Keyword sequences of parse.y symbols, keyword-only rules inlined."""

    def __init__(self, *, grammar: Grammar, words: dict[str, tuple[str, ...]]) -> None:
        self.grammar: Grammar = grammar
        self.words: dict[str, tuple[str, ...]] = words
        self.memo: dict[str, Sequences | None] = {}

    def terminal(self, symbol: str) -> Sequences:
        """A terminal or `A|B` alternation: its keywords, or nothing for other tokens."""
        found: Sequences = []
        for part in symbol.split(TOKEN_ALTERNATIVE):
            spelled: tuple[str, ...] = self.words.get(part, ())
            if part in PUNCTUATION_TOKENS:
                spelled = (PUNCTUATION_TOKENS[part],)
            found += [(word,) for word in spelled] if spelled else [()]
        return list(dict.fromkeys(found))

    def is_terminal(self, symbol: str) -> bool:
        """Terminals start upper case; a token class stands for identifier tokens."""
        return symbol[:1].isupper() or symbol in self.grammar.token_classes

    def keyword_only(self, name: str) -> Sequences | None:
        """A rule's sequences when every production spells only keywords, at most MAX_SET."""
        if name in self.memo:
            return self.memo[name]
        self.memo[name] = None
        found: Sequences | None = []
        for production in self.grammar.rules.get(name, []):
            parts: list[Sequences] = []
            for symbol in production.symbols:
                inner: Sequences | None = self.strict(symbol)
                if inner is None:
                    found = None
                    break
                parts.append(inner)
            if found is None:
                break
            try:
                found = list(dict.fromkeys([*found, *concatenations(parts)]))
            except SqliteCatalogueError:
                found = None
                break
        if found is not None and len(found) > MAX_SET:
            found = None
        self.memo[name] = found
        return found

    def strict(self, symbol: str) -> Sequences | None:
        """A symbol's sequences, or None when it is a rule that spells more than keywords."""
        if self.is_terminal(symbol):
            return [()] if symbol in self.grammar.token_classes else self.terminal(symbol)
        return self.keyword_only(symbol)

    def spell(self, production: Production) -> Sequences:
        """The keyword sequences one production spells; other rules contribute nothing."""
        parts: list[Sequences] = []
        for symbol in production.symbols:
            inner: Sequences | None = self.strict(symbol)
            parts.append([(ANY_RUN,)] if inner is None else inner or [()])
        spelled: Sequences = []
        for found in concatenations(parts):
            spelled.append(
                tuple(
                    word
                    for i, word in enumerate(found)
                    if word != ANY_RUN or found[i - 1 : i] != (ANY_RUN,)
                )
            )
        return spelled


def references(*, grammar: Grammar, name: str) -> Iterator[str]:
    """The nonterminals a rule's productions name."""
    for production in grammar.rules.get(name, []):
        for symbol in production.symbols:
            if symbol in grammar.rules:
                yield symbol


def walked_rules(grammar: Grammar) -> list[str]:
    """Every rule reachable from the start rule, breadth first."""
    seen: set[str] = {grammar.start}
    order: list[str] = [grammar.start]
    frontier: list[str] = [grammar.start]
    while frontier:
        for found in references(grammar=grammar, name=frontier.pop(0)):
            if found not in seen:
                seen.add(found)
                order.append(found)
                frontier.append(found)
    return order


def spelled(*, sequence: tuple[str, ...], leading: bool, words: CatalogueWords) -> bool:
    """True when one catalogued line reads the sequence gaplessly; a statement from its start."""
    return any(
        automaton.reads(sequence=sequence, start=start if leading else None)
        for _, automaton, start in words.automata
    )


def descendants(*, grammar: Grammar, name: str) -> set[str]:
    """Every rule a rule reaches."""
    seen: set[str] = set()
    frontier: list[str] = [name]
    while frontier:
        for found in references(grammar=grammar, name=frontier.pop()):
            if found not in seen:
                seen.add(found)
                frontier.append(found)
    return seen


def contains_run(*, big: tuple[str, ...], small: tuple[str, ...]) -> bool:
    """True when the small sequence appears as a contiguous run of the big one."""
    return any(big[i : i + len(small)] == small for i in range(len(big) - len(small) + 1))


def split_derived(
    *, gaps: list[AuditGap], grammar: Grammar, own: frozenset[str]
) -> tuple[list[AuditGap], int]:
    """Root gaps and the count of derived ones; a gap `own` names stays a root."""
    by_rule: dict[str, list[tuple[str, ...]]] = {}
    for gap in gaps:
        by_rule.setdefault(gap.rule, []).append(tuple(gap.keywords.split()))
    roots: list[AuditGap] = []
    derived: int = 0
    for gap in gaps:
        words: tuple[str, ...] = tuple(gap.keywords.split())
        below: set[str] = descendants(grammar=grammar, name=gap.rule) - {gap.rule}
        runs: list[tuple[str, ...]] = []
        for rule in below:
            runs += by_rule.get(rule, [])
        if f"{gap.rule}: {gap.keywords}" not in own and any(
            contains_run(big=words, small=run) for run in runs
        ):
            derived += 1
        else:
            roots.append(gap)
    return roots, derived


def grammar_tokens(*, grammar: Grammar, walked: list[str]) -> set[str]:
    """Every terminal a walked production names, token classes' members included."""
    found: set[str] = set()
    for name in walked:
        for production in grammar.rules[name]:
            for symbol in production.symbols:
                members: tuple[str, ...] = grammar.token_classes.get(symbol, ())
                found |= set(members) | set(symbol.split(TOKEN_ALTERNATIVE))
    return found


def engine_extract(cache: Path) -> dict[str, list[object]]:
    """The pinned engine's keywords, functions, pragmas and collations."""
    return json.loads((cache / ENGINE_EXTRACT).read_text())


def standard_types(cache: Path) -> list[str]:
    """The STRICT table type names the amalgamation's sqlite3StdType[] array lists."""
    text: str = (cache / ENGINE_DIR / AMALGAMATION_FILES[0]).read_text(errors="replace")
    match: re.Match[str] | None = STANDARD_TYPES.search(text)
    if match is None:
        raise SqliteCatalogueError("sqlite3.c: no sqlite3StdType[] array")
    return re.findall(r'"(\w+)"', match.group(1))


def generated_rules(cache: Path) -> int:
    """YYNRULE of the parser the amalgamation was generated with."""
    text: str = (cache / ENGINE_DIR / AMALGAMATION_FILES[0]).read_text(errors="replace")
    match: re.Match[str] | None = GENERATED_RULE_COUNT.search(text)
    if match is None:
        raise SqliteCatalogueError("sqlite3.c: no YYNRULE")
    return int(match.group(1))


def name_covered(*, name: str, words: CatalogueWords, grammars: str) -> bool:
    """A function, PRAGMA, type or collation name some catalogued grammar spells."""
    if ENGINE_NAME_WORD.fullmatch(name):
        return name.upper() in words.every
    pattern: str = rf"(?<![^\s<>\w\]\)']){re.escape(name)}(?![^\s<>\w\[\('])"
    return re.search(pattern, grammars) is not None


def engine_names(cache: Path) -> dict[str, list[EngineName]]:
    """The engine's built-in functions, PRAGMAs, collations and STRICT types, by kind."""
    extract: dict[str, list[object]] = engine_extract(cache)
    kinds: dict[str, set[str]] = {}
    for row in extract["functions"]:
        values: list[object] = cast("list[object]", row)
        kinds.setdefault(str(values[0]), set()).add(str(values[2]))
    return {
        "function": [EngineName(name, "/".join(sorted(k))) for name, k in sorted(kinds.items())],
        "pragma": [
            EngineName(str(cast("list[object]", row)[0]), "pragma") for row in extract["pragmas"]
        ],
        "collation": [
            EngineName(str(cast("list[object]", row)[0]), "collation")
            for row in extract["collations"]
        ],
        "type": [EngineName(name, "strict type") for name in standard_types(cache)],
    }


def gap_walk(
    *, grammar: Grammar, keywords: list[Keyword], words: CatalogueWords, walked: list[str]
) -> tuple[list[AuditGap], int]:
    """Every walked production's keyword sequences no catalogued line spells, and the count."""
    speller: _Speller = _Speller(grammar=grammar, words=token_words(keywords))
    gaps: list[AuditGap] = []
    checked: int = 0
    for name in walked:
        leading: bool = name == STATEMENT_RULE
        for index, production in enumerate(grammar.rules[name], 1):
            for sequence in speller.spell(production):
                if not any(KEYWORD_WORD.match(token) for token in sequence):
                    continue
                checked += 1
                if not spelled(sequence=sequence, leading=leading, words=words):
                    gaps.append(AuditGap(rule=name, alternative=index, keywords=" ".join(sequence)))
    return gaps, checked


def audit(*, cache: Path, root: Path, own: frozenset[str] = frozenset()) -> AuditResult:
    """parse.y sequences, keywords and engine names no form spells; `own` gaps stay roots."""
    grammar: Grammar = read_grammar(cache)
    keywords: list[Keyword] = read_keywords(cache)
    words: CatalogueWords = catalogue_words(
        root=root, keywords=frozenset(keyword.word for keyword in keywords)
    )
    walked: list[str] = walked_rules(grammar)
    gaps, checked = gap_walk(grammar=grammar, keywords=keywords, words=words, walked=walked)
    roots, derived = split_derived(gaps=gaps, grammar=grammar, own=own)
    used: set[str] = grammar_tokens(grammar=grammar, walked=walked)
    engine: list[str] = [str(word).upper() for word in engine_extract(cache)["keywords"]]
    table: set[str] = {keyword.word for keyword in keywords}
    names: dict[str, list[EngineName]] = engine_names(cache)
    grammars: str = "\n".join(line for _, line in catalogue_lines(root))
    missing: dict[str, list[EngineName]] = {}
    for kind, found in names.items():
        missing[kind] = [
            n for n in found if not name_covered(name=n.name, words=words, grammars=grammars)
        ]
    productions: int = sum(len(found) for found in grammar.rules.values())
    return AuditResult(
        gaps=roots,
        derived_gaps=derived,
        missing_keywords=[
            EngineName(k.word, k.token)
            for k in keywords
            if k.token in used and k.word not in words.every
        ],
        unused_keywords=[
            EngineName(k.word, k.token)
            for k in keywords
            if k.token not in used and k.word not in words.every
        ],
        engine_keywords=sorted(table.symmetric_difference(engine)),
        missing_functions=missing["function"],
        missing_pragmas=missing["pragma"],
        missing_types=missing["type"] + missing["collation"],
        sequences_checked=checked,
        rules_checked=len(walked),
        productions_checked=sum(len(grammar.rules[name]) for name in walked),
        productions_read=productions,
        productions_generated=generated_rules(cache),
        walked=walked,
        unwalked=sorted(set(grammar.rules) - set(walked)),
        functions_checked=len(names["function"]),
        keywords_checked=len(keywords),
        pragmas_checked=len(names["pragma"]),
        types_checked=len(names["type"]) + len(names["collation"]),
    )
