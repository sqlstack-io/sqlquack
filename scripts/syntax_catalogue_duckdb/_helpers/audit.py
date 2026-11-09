"""The omission audit: grammar keyword sequences, keywords, functions and types vs the catalogue."""

from __future__ import annotations

import itertools
import json
import re
import tomllib
from collections.abc import Iterator
from pathlib import Path
from typing import cast

from scripts.grammar_catalogue._helpers.grammar import bare_words
from scripts.syntax_catalogue_duckdb._helpers.peg import read_rules
from scripts.syntax_catalogue_duckdb.constants import (
    DEFINITION_HEAD,
    ENGINE_EXTRACT,
    KEYWORD_WORD,
    MAX_EXPANSION,
    MAX_SET,
    MIN_CLAUSE_KEYWORDS,
    NAME_SEPARATOR,
    NODE_CALL,
    NODE_CHOICE,
    NODE_LISTS,
    NODE_LIT,
    NODE_OPTIONAL_WRAPPERS,
    NODE_REF,
    NODE_REQUIRED_WRAPPERS,
    NODE_SEQ,
    NODE_WRAPPERS,
    OPTIONAL_CLOSE,
    OPTIONAL_OPEN,
    PLACEHOLDER,
    STATEMENT_RULE,
    WORD,
)
from scripts.syntax_catalogue_duckdb.exceptions import DuckDbCatalogueError
from scripts.syntax_catalogue_duckdb.models import (
    AuditGap,
    AuditResult,
    CatalogueWords,
    EngineName,
    PegRule,
)
from scripts.syntax_catalogue_duckdb.types import Node, Sequences


def inline(*, grammar: str, definitions: dict[str, str], depth: int = 3) -> str:
    """The grammar with each `<name>` its page defines replaced by the definition's body."""
    for _ in range(depth):
        replaced: str = PLACEHOLDER.sub(lambda m: definitions.get(m.group(1), m.group(0)), grammar)
        if replaced == grammar:
            break
        grammar = replaced
    return grammar


def lead_words(grammar: str) -> tuple[str, ...]:
    """A form's required leading keywords: before its first placeholder, choice or other word."""
    out: list[str] = []
    depth: int = 0
    for token in re.findall(r"<[^>]*>|[\[\]{}]|[A-Za-z_][A-Za-z0-9_]*|\S", grammar):
        if token == OPTIONAL_OPEN:
            depth += 1
        elif token == OPTIONAL_CLOSE:
            depth = max(0, depth - 1)
        elif depth:
            continue
        elif KEYWORD_WORD.match(token):
            out.append(token)
        else:
            break
    return tuple(out)


def catalogue_words(root: Path) -> CatalogueWords:
    """Every catalogued form's words with its page's definitions inlined, and every grammar word."""
    tables: list[dict[str, list[dict[str, object]]]] = [
        tomllib.loads(path.read_text()) for path in sorted((root / "catalogue").glob("*.toml"))
    ]
    definitions: dict[str, dict[str, str]] = {}
    every: set[str] = set()
    clauses: list[tuple[str, list[str]]] = []
    for data in tables:
        for definition in data.get("definition", []):
            body: str = DEFINITION_HEAD.sub("", str(definition["grammar"]))
            definitions.setdefault(str(definition["source"]), {})[str(definition["name"])] = body
            every |= {w.upper() for w in WORD.findall(body)}
            clauses.append((f"{definition['id']}", [w.upper() for w in bare_words(body)]))
    forms: list[tuple[str, list[str]]] = []
    leads: dict[str, tuple[str, ...]] = {}
    for data in tables:
        for form in data.get("form", []):
            lines: list[str] = [str(g) for g in cast("list[object]", form["grammar"])]
            page: dict[str, str] = definitions.get(str(form["source"]), {})
            for index, line in enumerate(lines):
                expanded: str = inline(grammar=line, definitions=page)
                key: str = f"{form['id']}#{index}"
                forms.append((key, [w.upper() for w in bare_words(expanded)]))
                leads[key] = lead_words(line)
                every |= {w.upper() for w in WORD.findall(line)}
    return CatalogueWords(forms=forms, every=every, leads=leads, clauses=clauses)


def substitute(*, node: Node, binding: dict[str, Node]) -> Node:
    """The node with macro parameters replaced by their arguments."""
    kind: object = node[0]
    if kind == NODE_REF and node[1] in binding:
        return binding[str(node[1])]
    if kind in NODE_LISTS:
        return (kind, [substitute(node=item, binding=binding) for item in node[1]])  # type: ignore[union-attr]
    if kind in NODE_WRAPPERS:
        return (kind, substitute(node=node[1], binding=binding))  # type: ignore[arg-type]
    if kind == NODE_CALL:
        return (kind, node[1], [substitute(node=a, binding=binding) for a in node[2]])  # type: ignore[union-attr]
    return node


def expand_call(*, node: Node, rules: dict[str, PegRule]) -> Node:
    """A macro call as its body with the arguments bound."""
    macro: PegRule = rules[str(node[1])]
    arguments: list[Node] = list(node[2])  # type: ignore[arg-type]
    return substitute(node=macro.body, binding=dict(zip(macro.parameters, arguments, strict=True)))


def concatenations(parts: list[Sequences]) -> Sequences:
    """Every way to pick one sequence from each part, joined in order."""
    if any(not part for part in parts):
        return []
    size: int = 1
    for part in parts:
        size *= len(part)
    if size > MAX_EXPANSION:
        raise DuckDbCatalogueError(f"a sequence spells more than {MAX_EXPANSION} keyword runs")
    return list(
        dict.fromkeys(
            tuple(itertools.chain.from_iterable(pick)) for pick in itertools.product(*parts)
        )
    )


class _Speller:
    """Keyword sequences of grammar nodes, keyword-only rules inlined."""

    def __init__(self, rules: dict[str, PegRule]) -> None:
        self.rules: dict[str, PegRule] = rules
        self.memo: dict[str, Sequences | None] = {}

    def keyword_only(self, name: str) -> Sequences | None:
        """A rule's sequences when it spells only keywords and punctuation, at most MAX_SET."""
        if name in self.memo:
            return self.memo[name]
        self.memo[name] = None
        rule: PegRule | None = self.rules.get(name)
        found: Sequences | None = None
        if rule is not None and not rule.parameters:
            try:
                found = self.strict(rule.body)
            except DuckDbCatalogueError:
                found = None
            if found is not None and (len(found) > MAX_SET or not any(found)):
                found = None
        self.memo[name] = found
        return found

    def strict(self, node: Node) -> Sequences | None:
        """The node's sequences, or None when it holds anything but keywords and punctuation."""
        kind: object = node[0]
        if kind == NODE_LIT:
            return [(str(node[1]),)] if KEYWORD_WORD.match(str(node[1])) else [()]
        if kind == NODE_REF:
            return self.keyword_only(str(node[1]))
        if kind == NODE_CALL:
            return self.strict(expand_call(node=node, rules=self.rules))
        if kind in NODE_LISTS:
            parts: list[Sequences] = []
            for item in node[1]:  # type: ignore[union-attr]
                inner: Sequences | None = self.strict(item)
                if inner is None:
                    return None
                parts.append(inner)
            if kind == NODE_CHOICE:
                return list(dict.fromkeys(itertools.chain.from_iterable(parts)))
            return concatenations(parts)
        if kind in NODE_OPTIONAL_WRAPPERS:
            inner = self.strict(node[1])  # type: ignore[arg-type]
            return None if inner is None else list(dict.fromkeys([(), *inner]))
        if kind in NODE_REQUIRED_WRAPPERS:
            return self.strict(node[1])  # type: ignore[arg-type]
        return None

    def spell(self, node: Node) -> Sequences:
        """The keyword sequences a node spells; other nonterminals contribute nothing."""
        kind: object = node[0]
        if kind == NODE_LIT:
            return [(str(node[1]),)] if KEYWORD_WORD.match(str(node[1])) else [()]
        if kind == NODE_REF:
            return self.keyword_only(str(node[1])) or [()]
        if kind == NODE_CALL:
            return self.spell(expand_call(node=node, rules=self.rules))
        if kind == NODE_SEQ:
            return concatenations([self.spell(item) for item in node[1]])  # type: ignore[union-attr]
        if kind == NODE_CHOICE:
            spellings: list[Sequences] = [self.spell(item) for item in node[1]]  # type: ignore[union-attr]
            return list(dict.fromkeys(itertools.chain.from_iterable(spellings)))
        if kind in NODE_OPTIONAL_WRAPPERS:
            return list(dict.fromkeys([(), *self.spell(node[1])]))  # type: ignore[arg-type]
        if kind in NODE_REQUIRED_WRAPPERS:
            return self.spell(node[1])  # type: ignore[arg-type]
        return [()]


def alternatives(node: Node) -> list[Node]:
    """A rule body's top-level alternatives."""
    return list(node[1]) if node[0] == NODE_CHOICE else [node]  # type: ignore[arg-type]


def references(node: Node) -> Iterator[str]:
    """Every rule a node names, macro calls included."""
    kind: object = node[0]
    if kind == NODE_REF:
        yield str(node[1])
    elif kind == NODE_CALL:
        yield str(node[1])
        for argument in node[2]:  # type: ignore[union-attr]
            yield from references(argument)
    elif kind in NODE_LISTS:
        for item in node[1]:  # type: ignore[union-attr]
            yield from references(item)
    elif kind in NODE_WRAPPERS:
        yield from references(node[1])  # type: ignore[arg-type]


def statement_rules(rules: dict[str, PegRule]) -> list[str]:
    """The statement rules `Statement` lists."""
    return [
        str(item[1]) for item in alternatives(rules[STATEMENT_RULE].body) if item[0] == NODE_REF
    ]


def walked_rules(rules: dict[str, PegRule]) -> list[str]:
    """Every rule reachable from `Statement`, statements first."""
    statements: list[str] = statement_rules(rules)
    seen: set[str] = {STATEMENT_RULE, *statements}
    order: list[str] = list(statements)
    frontier: list[str] = list(statements)
    while frontier:
        name: str = frontier.pop(0)
        for found in references(rules[name].body):
            if found in rules and found not in seen and not rules[found].parameters:
                seen.add(found)
                order.append(found)
                frontier.append(found)
    return order


def is_subsequence(*, needle: tuple[str, ...], haystack: list[str]) -> bool:
    """True when the words appear in order in the haystack."""
    iterator: Iterator[str] = iter(haystack)
    return all(word in iterator for word in needle)


def spelled(*, sequence: tuple[str, ...], leading: bool, words: CatalogueWords) -> bool:
    """True when a catalogued grammar (or, for a clause, a definition) spells the sequence."""
    if not leading and any(
        is_subsequence(needle=sequence, haystack=body) for _, body in words.clauses
    ):
        return True
    return any(
        is_subsequence(needle=sequence, haystack=form_words)
        and (
            not leading
            or (bool(form_words) and sequence[0] in (form_words[0], *words.leads[key][:1]))
        )
        for key, form_words in words.forms
    )


def descendants(*, name: str, rules: dict[str, PegRule]) -> set[str]:
    """Every rule a rule reaches, macro bodies included."""
    seen: set[str] = set()
    frontier: list[str] = [name]
    while frontier:
        for found in references(rules[frontier.pop()].body):
            if found in rules and found not in seen:
                seen.add(found)
                frontier.append(found)
    return seen


def contains_run(*, big: tuple[str, ...], small: tuple[str, ...]) -> bool:
    """True when the small sequence appears as a contiguous run of the big one."""
    return any(big[i : i + len(small)] == small for i in range(len(big) - len(small) + 1))


def runs_of(*, rules: set[str], by_rule: dict[str, list[tuple[str, ...]]]) -> list[tuple[str, ...]]:
    """Every gap sequence recorded for any of the rules."""
    runs: list[tuple[str, ...]] = []
    for name in rules:
        runs.extend(by_rule.get(name, []))
    return runs


def split_derived(
    *, gaps: list[AuditGap], rules: dict[str, PegRule], own: frozenset[str]
) -> tuple[list[AuditGap], int]:
    """Root gaps and the count of derived ones; a gap `own` names stays a root."""
    by_rule: dict[str, list[tuple[str, ...]]] = {}
    for gap in gaps:
        by_rule.setdefault(gap.rule, []).append(tuple(gap.keywords.split()))
    roots: list[AuditGap] = []
    derived: int = 0
    reach: dict[str, set[str]] = {}
    for gap in gaps:
        words: tuple[str, ...] = tuple(gap.keywords.split())
        reached: set[str] = reach.setdefault(gap.rule, descendants(name=gap.rule, rules=rules))
        below: set[str] = reached - {gap.rule}
        if f"{gap.rule}: {gap.keywords}" not in own and any(
            contains_run(big=words, small=other) for other in runs_of(rules=below, by_rule=by_rule)
        ):
            derived += 1
        else:
            roots.append(gap)
    return roots, derived


def literal_keywords(node: Node) -> set[str]:
    """Every keyword literal a node spells."""
    kind: object = node[0]
    found: set[str] = set()
    if kind == NODE_LIT and KEYWORD_WORD.match(str(node[1])):
        found.add(str(node[1]))
    elif kind in NODE_LISTS:
        for item in node[1]:  # type: ignore[union-attr]
            found |= literal_keywords(item)
    elif kind == NODE_CALL:
        for item in node[2]:  # type: ignore[union-attr]
            found |= literal_keywords(item)
    elif kind in NODE_WRAPPERS:
        found |= literal_keywords(node[1])  # type: ignore[arg-type]
    return found


def grammar_keywords(rules: dict[str, PegRule]) -> set[str]:
    """Every keyword literal the grammar spells."""
    found: set[str] = set()
    for rule in rules.values():
        found |= literal_keywords(rule.body)
    return found


def engine_names(cache: Path) -> tuple[list[EngineName], list[EngineName], list[EngineName]]:
    """The pinned engine's built-in functions, keywords and types."""
    extract: dict[str, list[list[object]]] = json.loads((cache / ENGINE_EXTRACT).read_text())
    kinds: dict[str, set[str]] = {}
    for row in extract["functions"]:
        kinds.setdefault(str(row[0]), set()).add(str(row[1]))
    functions: list[EngineName] = [
        EngineName(name=name, kind="/".join(sorted(kind))) for name, kind in sorted(kinds.items())
    ]
    keywords: list[EngineName] = [
        EngineName(name=str(row[0]).upper(), kind=str(row[1])) for row in extract["keywords"]
    ]
    types: list[EngineName] = [
        EngineName(name=str(row[0]), kind=str(row[1])) for row in extract["types"]
    ]
    return functions, keywords, types


def name_covered(*, name: str, words: CatalogueWords, grammars: str) -> bool:
    """A function or type name some catalogued grammar spells: as a word, or as an operator."""
    if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
        return name.upper() in words.every
    if NAME_SEPARATOR in name:
        return all(part in words.every for part in name.upper().split())
    return (
        re.search(rf"(?<![^\s<>\w\]\)']){re.escape(name)}(?![^\s<>\w\[\('])", grammars) is not None
    )


def catalogue_grammars(root: Path) -> str:
    """Every catalogued form's grammar lines, one per line, in catalogue file order."""
    lines: list[str] = []
    for path in sorted((root / "catalogue").glob("*.toml")):
        for form in tomllib.loads(path.read_text()).get("form", []):
            lines.extend(str(g) for g in form["grammar"])
    return "\n".join(lines)


def audit(
    *, cache: Path, grammar: Path, root: Path, own: frozenset[str] = frozenset()
) -> AuditResult:
    """Grammar sequences, keywords, functions and types no form spells; `own` gaps stay roots."""
    rules: dict[str, PegRule] = read_rules(grammar)
    words: CatalogueWords = catalogue_words(root)
    speller: _Speller = _Speller(rules)
    statements: set[str] = set(statement_rules(rules))
    walked: list[str] = walked_rules(rules)
    gaps: list[AuditGap] = []
    checked: int = 0
    for name in walked:
        leading: bool = name in statements
        for index, alternative in enumerate(alternatives(rules[name].body), 1):
            for sequence in speller.spell(alternative):
                if not sequence or (not leading and len(sequence) < MIN_CLAUSE_KEYWORDS):
                    continue
                checked += 1
                if not spelled(sequence=sequence, leading=leading, words=words):
                    gaps.append(AuditGap(rule=name, alternative=index, keywords=" ".join(sequence)))
    roots, derived = split_derived(gaps=gaps, rules=rules, own=own)
    used: set[str] = grammar_keywords(rules)
    functions, keywords, types = engine_names(cache)
    grammars: str = catalogue_grammars(root)
    return AuditResult(
        gaps=roots,
        derived_gaps=derived,
        missing_keywords=[k for k in keywords if k.name in used and k.name not in words.every],
        unused_keywords=[k for k in keywords if k.name not in used and k.name not in words.every],
        missing_functions=[
            f for f in functions if not name_covered(name=f.name, words=words, grammars=grammars)
        ],
        missing_types=[
            t for t in types if not name_covered(name=t.name, words=words, grammars=grammars)
        ],
        sequences_checked=checked,
        rules_checked=len(walked),
        walked=walked,
        unwalked=sorted(
            name
            for name, rule in rules.items()
            if name not in walked and name != STATEMENT_RULE and not rule.parameters
        ),
        functions_checked=len(functions),
        keywords_checked=len(keywords),
        types_checked=len(types),
    )
