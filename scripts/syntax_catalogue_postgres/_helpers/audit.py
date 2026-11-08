"""The omission audit: gram.y statement alternatives and kwlist.h keywords against the catalogue."""

from __future__ import annotations

import itertools
import math
import tomllib
from collections.abc import Iterator
from pathlib import Path

from scripts.syntax_catalogue._helpers.grammar import bare_words
from scripts.syntax_catalogue_postgres._helpers.gram import keywords, rules, token_text
from scripts.syntax_catalogue_postgres._helpers.groups import required_without_options
from scripts.syntax_catalogue_postgres.constants import (
    DEFINITION_HEAD,
    EXPRESSION_RULES,
    GRAM_Y,
    KEYWORD_LISTS,
    KWLIST_H,
    LEAD_TOKEN,
    MAX_EXPANSION,
    MAX_KEYWORD_ALTERNATIVE,
    MAX_SET,
    MIN_CLAUSE_KEYWORDS,
    PLACEHOLDER,
    STATEMENT_RULE,
    TOPLEVEL_RULE,
    WORD_JOINER,
)
from scripts.syntax_catalogue_postgres.exceptions import PgCatalogueError
from scripts.syntax_catalogue_postgres.models import (
    AuditGap,
    AuditResult,
    CatalogueWords,
    Keyword,
    Rule,
)


def inline(*, grammar: str, definitions: dict[str, str], depth: int = 3) -> str:
    """The grammar with each `<name>` its page defines replaced by the definition's body."""
    for _ in range(depth):
        replaced: str = PLACEHOLDER.sub(lambda m: definitions.get(m.group(1), m.group(0)), grammar)
        if replaced == grammar:
            break
        grammar = replaced
    return grammar


def lead_words(grammar: str) -> tuple[str, ...]:
    """A form's required keywords before its first placeholder, options and choices removed."""
    text: str = required_without_options(grammar)
    out: list[str] = []
    for token in LEAD_TOKEN.findall(text):
        if token.startswith(("<", "{")) or not token.isupper():
            break
        out.append(token)
    return tuple(out)


def upper_bare_words(lines: list[str]) -> list[str]:
    """The upper-cased bare words of every line, in order."""
    words: list[str] = []
    for line in lines:
        words += [w.upper() for w in bare_words(line)]
    return words


def upper_plain_words(lines: list[str]) -> set[str]:
    """The upper-cased plain words of every line."""
    words: set[str] = set()
    for line in lines:
        words |= {w.upper() for w in plain_words(line)}
    return words


def catalogue_words(root: Path) -> CatalogueWords:
    """Every catalogued form's words with its page's definitions inlined, and every grammar word."""
    tables: list[dict[str, list[dict[str, object]]]] = [
        tomllib.loads(path.read_text()) for path in sorted((root / "catalogue").glob("*.toml"))
    ]
    definitions: dict[str, dict[str, str]] = {}
    every: set[str] = set()
    for data in tables:
        for definition in data.get("definition", []):
            body: str = DEFINITION_HEAD.sub("", str(definition["grammar"]))
            name: str = str(definition["name"]).split(" #")[0]
            definitions.setdefault(str(definition["source"]), {}).setdefault(name, body)
            every |= {w.upper() for w in plain_words(body)}
    forms: list[tuple[str, list[str]]] = []
    leads: dict[str, tuple[str, ...]] = {}
    for data in tables:
        for form in data.get("form", []):
            grammar: object = form["grammar"]
            lines: list[str] = [str(g) for g in grammar] if isinstance(grammar, list) else []
            page: dict[str, str] = definitions.get(str(form["source"]), {})
            expanded: list[str] = [inline(grammar=line, definitions=page) for line in lines]
            words: list[str] = upper_bare_words(expanded)
            forms.append((str(form["id"]), words))
            leads[str(form["id"])] = lead_words(lines[0]) if lines else ()
            every |= set(words) | upper_plain_words(lines)
    return CatalogueWords(forms=forms, every=every, leads=leads)


def plain_words(line: str) -> list[str]:
    """Every word of a grammar line, placeholders and quoted text included."""
    word: str = ""
    out: list[str] = []
    for ch in line + " ":
        if ch.isalnum() or ch == WORD_JOINER:
            word += ch
        elif word:
            out.append(word)
            word = ""
    return out


def concatenations(parts: list[list[tuple[str, ...]]]) -> list[tuple[str, ...]]:
    """Every way to pick one sequence from each part, joined in order."""
    out: list[tuple[str, ...]] = []
    for combination in itertools.product(*parts):
        words: list[str] = []
        for piece in combination:
            words.extend(piece)
        out.append(tuple(words))
    return out


def keyword_set(
    *, rule: Rule | None, grammar: dict[str, Rule], keyword_tokens: dict[str, str], depth: int = 2
) -> list[tuple[str, ...]] | None:
    """A rule's alternatives as keyword sequences when all are short keyword runs, else None."""
    if rule is None or depth == 0 or len(rule.alternatives) > MAX_SET:
        return None
    out: list[tuple[str, ...]] = []
    for alternative in rule.alternatives:
        if len(alternative) > MAX_KEYWORD_ALTERNATIVE:
            return None
        parts: list[list[tuple[str, ...]]] = []
        for token in alternative:
            if token in keyword_tokens:
                parts.append([(keyword_tokens[token],)])
                continue
            inner: list[tuple[str, ...]] | None = keyword_set(
                rule=grammar.get(token),
                grammar=grammar,
                keyword_tokens=keyword_tokens,
                depth=depth - 1,
            )
            if inner is None:
                return None
            parts.append(inner)
        out += concatenations(parts)
    return out


def expansions(
    *, alternative: tuple[str, ...], grammar: dict[str, Rule], keyword_tokens: dict[str, str]
) -> list[tuple[str, ...]]:
    """The keyword sequences an alternative spells, keyword-set nonterminals expanded."""
    parts: list[list[tuple[str, ...]]] = []
    for token in alternative:
        text: str = token_text(token=token, keyword_tokens=keyword_tokens)
        if token in keyword_tokens:
            parts.append([(text,)])
            continue
        options: list[tuple[str, ...]] | None = keyword_set(
            rule=grammar.get(token), grammar=grammar, keyword_tokens=keyword_tokens
        )
        if options and any(options):
            parts.append(options)
    if math.prod(len(part) for part in parts) > MAX_EXPANSION:
        message: str = f"{' '.join(alternative)} spells more than {MAX_EXPANSION} sequences"
        raise PgCatalogueError(message)
    out: list[tuple[str, ...]] = concatenations(parts)
    return list(dict.fromkeys(out))


def is_subsequence(*, needle: tuple[str, ...], haystack: list[str]) -> bool:
    """True when the words appear in order in the haystack."""
    iterator: Iterator[str] = iter(haystack)
    return all(word in iterator for word in needle)


def statement_rules(grammar: dict[str, Rule]) -> list[str]:
    """The statement rules `toplevel_stmt` and `stmt` list."""
    names: list[str] = []
    for rule in (TOPLEVEL_RULE, STATEMENT_RULE):
        names += [
            alternative[0]
            for alternative in grammar[rule].alternatives
            if alternative and alternative[0] != STATEMENT_RULE
        ]
    return list(dict.fromkeys(names))


def clause_rules(*, grammar: dict[str, Rule], statements: list[str]) -> list[str]:
    """Every rule reachable from a statement or expression rule, other than those and keywords."""
    seen: set[str] = set(statements) | set(EXPRESSION_RULES) | KEYWORD_LISTS
    frontier: list[str] = [*statements, *EXPRESSION_RULES]
    found: list[str] = []
    while frontier:
        name: str = frontier.pop(0)
        for alternative in grammar[name].alternatives:
            for token in alternative:
                if token in grammar and token not in seen:
                    seen.add(token)
                    found.append(token)
                    frontier.append(token)
    return found


def unaudited_rules(grammar: dict[str, Rule]) -> list[str]:
    """Rules the audit does not walk: those no statement or expression rule reaches."""
    statements: list[str] = statement_rules(grammar)
    walked: set[str] = (
        set(statements)
        | set(EXPRESSION_RULES)
        | set(clause_rules(grammar=grammar, statements=statements))
    )
    return sorted(set(grammar) - walked)


def keyword_uses(grammar: dict[str, Rule]) -> set[str]:
    """Grammar tokens used by any rule other than the keyword category lists."""
    used: set[str] = set()
    for name, rule in grammar.items():
        if name in KEYWORD_LISTS:
            continue
        for alternative in rule.alternatives:
            used |= set(alternative)
    return used


def spelled(*, sequence: tuple[str, ...], leading: bool, words: CatalogueWords) -> bool:
    """True when a catalogued form spells the sequence, led by its first keyword for statements."""
    return any(
        is_subsequence(needle=sequence, haystack=form_words)
        and (
            not leading
            or (
                bool(words.leads[form_id])
                and words.leads[form_id][0] == sequence[0]
                and is_subsequence(needle=words.leads[form_id], haystack=list(sequence))
            )
        )
        for form_id, form_words in words.forms
    )


def audit(*, tree: Path, root: Path) -> AuditResult:
    """Grammar alternatives no catalogued form spells and keywords no grammar names."""
    grammar: dict[str, Rule] = rules(tree / GRAM_Y)
    found_keywords: list[Keyword] = keywords(tree / KWLIST_H)
    keyword_tokens: dict[str, str] = {k.token: k.text for k in found_keywords}
    words: CatalogueWords = catalogue_words(root)
    statements: list[str] = statement_rules(grammar)
    clauses: list[str] = clause_rules(grammar=grammar, statements=statements)
    gaps: list[AuditGap] = []
    checked: int = 0
    for name in [*statements, *EXPRESSION_RULES, *clauses]:
        leading: bool = name in statements
        for index, alternative in enumerate(grammar[name].alternatives, 1):
            for sequence in expansions(
                alternative=alternative, grammar=grammar, keyword_tokens=keyword_tokens
            ):
                if not sequence or (not leading and len(sequence) < MIN_CLAUSE_KEYWORDS):
                    continue
                checked += 1
                if not spelled(sequence=sequence, leading=leading, words=words):
                    gaps.append(
                        AuditGap(
                            rule=name,
                            alternative=index,
                            keywords=" ".join(sequence),
                            production=" ".join(alternative),
                        )
                    )
    used: set[str] = keyword_uses(grammar)
    missing: list[Keyword] = [k for k in found_keywords if k.text not in words.every]
    return AuditResult(
        gaps=gaps,
        unused=[k for k in missing if k.token not in used],
        missing=[k for k in missing if k.token in used],
        alternatives_checked=checked,
        rules_checked=len(statements) + len(EXPRESSION_RULES) + len(clauses),
        unwalked=unaudited_rules(grammar),
        keywords_checked=len(found_keywords),
    )
