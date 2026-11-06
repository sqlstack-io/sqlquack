"""The omission audit: gram.y statement alternatives and kwlist.h keywords against the catalogue."""

from __future__ import annotations

import itertools
import math
import re
import tomllib
from pathlib import Path

from scripts.syntax_catalogue._helpers.grammar import bare_words
from scripts.syntax_catalogue_pg._helpers.gram import keywords, rules, token_text
from scripts.syntax_catalogue_pg._helpers.groups import required_without_options
from scripts.syntax_catalogue_pg.constants import GRAM_Y, KWLIST_H
from scripts.syntax_catalogue_pg.models import AuditGap, AuditResult, Keyword, Rule

STATEMENT_RULE: str = "stmt"
TOPLEVEL_RULE: str = "toplevel_stmt"
EXPRESSION_RULES: tuple[str, ...] = (
    "a_expr",
    "b_expr",
    "c_expr",
    "func_expr_common_subexpr",
    "func_expr_windowless",
    "AexprConst",
    "SimpleTypename",
    "GenericType",
    "Numeric",
    "Bit",
    "Character",
    "ConstDatetime",
    "ConstInterval",
    "json_aggregate_func",
    "xmltable",
    "json_table",
    "table_ref",
    "joined_table",
    "select_clause",
    "simple_select",
)
MAX_SET: int = 60
LEAD_TOKEN: re.Pattern[str] = re.compile(r"<[^<>]*>|\{|[A-Za-z_][A-Za-z0-9_]*")
leads: dict[str, tuple[str, ...]] = {}
MIN_CLAUSE_KEYWORDS: int = 2
KEYWORD_LISTS: frozenset[str] = frozenset(
    {
        "unreserved_keyword",
        "col_name_keyword",
        "type_func_name_keyword",
        "reserved_keyword",
        "bare_label_keyword",
    }
)
PLACEHOLDER: re.Pattern[str] = re.compile(r"<([^<>]+)>")
DEFINITION_HEAD: re.Pattern[str] = re.compile(r"^\\s*<[^<>]+>\\s*::=\\s*")
MAX_EXPANSION: int = 64
MAX_KEYWORD_ALTERNATIVE: int = 4


def inline(grammar: str, definitions: dict[str, str], depth: int = 3) -> str:
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


def catalogue_words(root: Path) -> tuple[list[tuple[str, list[str]]], set[str]]:
    """(form id, upper-case words) for every catalogued form, its page's definitions inlined,
    and every word of every grammar."""
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
    leads.clear()
    for data in tables:
        for form in data.get("form", []):
            grammar: object = form["grammar"]
            lines: list[str] = [str(g) for g in grammar] if isinstance(grammar, list) else []
            page: dict[str, str] = definitions.get(str(form["source"]), {})
            expanded: list[str] = [inline(line, page) for line in lines]
            words: list[str] = [w.upper() for line in expanded for w in bare_words(line)]
            forms.append((str(form["id"]), words))
            leads[str(form["id"])] = lead_words(lines[0]) if lines else ()
            every |= set(words) | {w.upper() for line in lines for w in plain_words(line)}
    return forms, every


def plain_words(line: str) -> list[str]:
    """Every word of a grammar line, placeholders and quoted text included."""
    word: str = ""
    out: list[str] = []
    for ch in line + " ":
        if ch.isalnum() or ch == "_":
            word += ch
        elif word:
            out.append(word)
            word = ""
    return out


def keyword_set(
    rule: Rule | None, grammar: dict[str, Rule], keyword_tokens: dict[str, str], depth: int = 2
) -> list[tuple[str, ...]] | None:
    """A rule's alternatives as keyword sequences when every alternative is a short run of
    keywords and keyword-set rules (or empty), else None."""
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
                grammar.get(token), grammar, keyword_tokens, depth - 1
            )
            if inner is None:
                return None
            parts.append(inner)
        out += [tuple(w for piece in combo for w in piece) for combo in itertools.product(*parts)]
    return out


def expansions(
    alternative: tuple[str, ...], grammar: dict[str, Rule], keyword_tokens: dict[str, str]
) -> list[tuple[str, ...]]:
    """The keyword sequences an alternative spells: its keywords in order, each keyword-set
    nonterminal expanded into its alternatives."""
    parts: list[list[tuple[str, ...]]] = []
    for token in alternative:
        text: str = token_text(token, keyword_tokens)
        if token in keyword_tokens:
            parts.append([(text,)])
            continue
        options: list[tuple[str, ...]] | None = keyword_set(
            grammar.get(token), grammar, keyword_tokens
        )
        if options and any(options):
            parts.append(options)
    if math.prod(len(part) for part in parts) > MAX_EXPANSION:
        message: str = f"{' '.join(alternative)} spells more than {MAX_EXPANSION} sequences"
        raise ValueError(message)
    out: list[tuple[str, ...]] = [
        tuple(word for piece in combination for word in piece)
        for combination in itertools.product(*parts)
    ]
    return list(dict.fromkeys(out))


def is_subsequence(needle: tuple[str, ...], haystack: list[str]) -> bool:
    """True when the words appear in order in the haystack."""
    iterator = iter(haystack)
    return all(word in iterator for word in needle)


def statement_rules(grammar: dict[str, Rule]) -> list[str]:
    """The statement rules `toplevel_stmt` and `stmt` list."""
    names: list[str] = [
        alternative[0]
        for rule in (TOPLEVEL_RULE, STATEMENT_RULE)
        for alternative in grammar[rule].alternatives
        if alternative and alternative[0] != STATEMENT_RULE
    ]
    return list(dict.fromkeys(names))


def clause_rules(grammar: dict[str, Rule], statements: list[str]) -> list[str]:
    """Every rule reachable from a statement or expression rule, other than those and the
    keyword category lists."""
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
        set(statements) | set(EXPRESSION_RULES) | set(clause_rules(grammar, statements))
    )
    return sorted(set(grammar) - walked)


def keyword_uses(grammar: dict[str, Rule]) -> set[str]:
    """Grammar tokens used by any rule other than the keyword category lists."""
    return {
        token
        for name, rule in grammar.items()
        if name not in KEYWORD_LISTS
        for alternative in rule.alternatives
        for token in alternative
    }


def audit(*, tree: Path, root: Path) -> AuditResult:
    """Grammar alternatives no catalogued form spells and keywords no grammar names, with
    what was checked."""
    grammar: dict[str, Rule] = rules(tree / GRAM_Y)
    words: list[Keyword] = keywords(tree / KWLIST_H)
    keyword_tokens: dict[str, str] = {k.token: k.text for k in words}
    forms, every = catalogue_words(root)
    statements: list[str] = statement_rules(grammar)
    clauses: list[str] = clause_rules(grammar, statements)
    gaps: list[AuditGap] = []
    checked: int = 0
    for name in [*statements, *EXPRESSION_RULES, *clauses]:
        leading: bool = name in statements
        for index, alternative in enumerate(grammar[name].alternatives, 1):
            for sequence in expansions(alternative, grammar, keyword_tokens):
                if not sequence or (not leading and len(sequence) < MIN_CLAUSE_KEYWORDS):
                    continue
                checked += 1
                found: bool = any(
                    is_subsequence(sequence, form_words)
                    and (
                        not leading
                        or (
                            bool(leads[form_id])
                            and leads[form_id][0] == sequence[0]
                            and is_subsequence(leads[form_id], list(sequence))
                        )
                    )
                    for form_id, form_words in forms
                )
                if not found:
                    gaps.append(
                        AuditGap(
                            rule=name,
                            alternative=index,
                            keywords=" ".join(sequence),
                            production=" ".join(alternative),
                        )
                    )
    used: set[str] = keyword_uses(grammar)
    missing: list[Keyword] = [k for k in words if k.text not in every]
    return AuditResult(
        gaps=gaps,
        unused=[k for k in missing if k.token not in used],
        missing=[k for k in missing if k.token in used],
        alternatives_checked=checked,
        rules_checked=len(statements) + len(EXPRESSION_RULES) + len(clauses),
        unwalked=unaudited_rules(grammar),
        keywords_checked=len(words),
    )
