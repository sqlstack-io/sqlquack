"""The omission audit: GoogleSQL grammar keyword sequences, keywords, functions and types."""

from __future__ import annotations

import itertools
import re
import tomllib
from collections.abc import Iterator
from pathlib import Path

from scripts.grammar_catalogue._helpers.grammar import bare_words
from scripts.syntax_catalogue_bigquery._helpers.hand import strings
from scripts.syntax_catalogue_bigquery._helpers.keywords import keyword_table
from scripts.syntax_catalogue_bigquery._helpers.pages import load_pages
from scripts.syntax_catalogue_bigquery._helpers.tm import (
    as_node,
    dispatched_words,
    node_members,
    read_rules,
)
from scripts.syntax_catalogue_bigquery.constants import (
    CATALOGUE_DIR,
    DATA_TYPES_PAGE,
    DEFINITION_SEPARATOR,
    ESCAPED_TICK,
    FENCED_BLOCK,
    FORM_ALTERNATIVE,
    FORM_CHOICE_OPEN,
    FORM_CLOSERS,
    FORM_ELISION,
    FORM_OPTIONAL_OPEN,
    FORM_PLACEHOLDER,
    FORM_TOKEN,
    FUNCTION_INDEX_ENTRY,
    FUNCTION_INDEX_PAGE,
    INLINE_DEPTH,
    KEYWORD_WORD,
    KW_TOKEN_JOINER,
    KW_TOKEN_PREFIX,
    MAX_EXPANSION,
    MAX_SET,
    MIN_CLAUSE_KEYWORDS,
    NODE_CHOICE,
    NODE_LISTS,
    NODE_LIT,
    NODE_REF,
    NODE_REQUIRED_WRAPPERS,
    NODE_TOKEN,
    NODE_WRAPPERS,
    PAGE_CODE_SPAN,
    REJECTED,
    STATEMENT_DISPATCH,
    TOKEN_ALIASES,
    TYPE_ALIAS_SPAN,
    TYPE_NAME,
    WALK_ROOTS,
    WILDCARD,
    WORD_TOKEN,
)
from scripts.syntax_catalogue_bigquery.exceptions import BigQueryCatalogueError
from scripts.syntax_catalogue_bigquery.models import (
    AuditGap,
    AuditResult,
    BigQueryPage,
    GrammarRule,
)
from scripts.syntax_catalogue_bigquery.types import Nfa, Node, Sequences


def token_keyword(*, name: str, keywords: dict[str, str]) -> str:
    """The keyword a lexer token stands for (KW_EXCEPT_IN_SET_OP is EXCEPT), or ""."""
    if not name.startswith(KW_TOKEN_PREFIX):
        return ""
    parts: list[str] = name.removeprefix(KW_TOKEN_PREFIX).split(KW_TOKEN_JOINER)
    for size in range(len(parts), 0, -1):
        word: str = KW_TOKEN_JOINER.join(parts[:size])
        if word in keywords:
            return word
    return ""


def collapse(sequence: tuple[str, ...]) -> tuple[str, ...]:
    """The sequence with runs of wildcards merged."""
    out: list[str] = []
    for word in sequence:
        if word == WILDCARD and out and out[-1] == WILDCARD:
            continue
        out.append(word)
    return tuple(out)


def concatenations(parts: list[Sequences]) -> Sequences:
    """Every way to pick one sequence from each part, joined in order."""
    size: int = 1
    for part in parts:
        size *= max(len(part), 1)
    if size > MAX_EXPANSION:
        raise BigQueryCatalogueError(f"a rule alternative spells more than {MAX_EXPANSION} runs")
    return list(
        dict.fromkeys(
            collapse(tuple(itertools.chain.from_iterable(pick)))
            for pick in itertools.product(*parts)
        )
    )


def unknown(strict: bool) -> Sequences | None:
    """A node that spells more than keywords: None when strict, else one wildcard."""
    return None if strict else [(WILDCARD,)]


class _Speller:
    """Keyword sequences of grammar nodes; keyword-only rules inlined, other rules a wildcard."""

    def __init__(self, *, rules: dict[str, GrammarRule], keywords: dict[str, str]) -> None:
        self.rules: dict[str, GrammarRule] = rules
        self.keywords: dict[str, str] = keywords
        self.memo: dict[str, Sequences | None] = {}

    def word(self, node: Node) -> str | None:
        """A literal's or token's keyword; "" for punctuation; None for a non-keyword token."""
        if node[0] == NODE_LIT:
            text: str = TOKEN_ALIASES.get(str(node[1]), str(node[1]))
            return text if KEYWORD_WORD.match(text) else ""
        found: str = token_keyword(name=str(node[1]), keywords=self.keywords)
        return found or None

    def keyword_only(self, name: str) -> Sequences | None:
        """A rule's sequences when it spells only keywords and punctuation, at most MAX_SET."""
        if name in self.memo:
            return self.memo[name]
        self.memo[name] = None
        rule: GrammarRule | None = self.rules.get(name)
        found: Sequences | None = None
        if rule is not None:
            try:
                found = self.spell(node=rule.body, strict=True)
            except BigQueryCatalogueError:
                found = None
            if found is not None and (len(found) > MAX_SET or not any(found)):
                found = None
        self.memo[name] = found
        return found

    def spell_list(self, *, node: Node, strict: bool) -> Sequences | None:
        """A sequence's or choice's keyword sequences."""
        parts: list[Sequences] = []
        for item in node_members(node):
            spelled: Sequences | None = self.spell(node=item, strict=strict)
            if spelled is None:
                return None
            parts.append(spelled)
        if node[0] == NODE_CHOICE:
            return list(dict.fromkeys(itertools.chain.from_iterable(parts)))
        return concatenations(parts)

    def spell(self, *, node: Node, strict: bool = False) -> Sequences | None:
        """The node's keyword sequences; with `strict`, None when it spells more than keywords."""
        kind: object = node[0]
        if kind in (NODE_LIT, NODE_TOKEN):
            word: str | None = self.word(node)
            if word is None:
                return unknown(strict)
            return [(word,)] if word else [()]
        if kind == NODE_REF:
            return self.keyword_only(str(node[1])) or unknown(strict)
        if kind in NODE_LISTS:
            return self.spell_list(node=node, strict=strict)
        if kind in NODE_WRAPPERS:
            inner: Sequences | None = self.spell(node=as_node(node[1]), strict=strict)
            if inner is None or kind in NODE_REQUIRED_WRAPPERS:
                return inner
            return list(dict.fromkeys([(), *inner]))
        return unknown(strict)


def alternatives(rule: GrammarRule) -> list[Node]:
    """A rule's top-level alternatives."""
    return node_members(rule.body)


def references(node: Node) -> Iterator[str]:
    """Every rule a node names."""
    kind: object = node[0]
    if kind == NODE_REF:
        yield str(node[1])
    elif kind in NODE_LISTS:
        for item in node_members(node):
            yield from references(item)
    elif kind in NODE_WRAPPERS:
        yield from references(as_node(node[1]))


def statement_rules(rules: dict[str, GrammarRule]) -> list[str]:
    """The statement rules the statement dispatchers list as a whole alternative."""
    found: list[str] = []
    for dispatcher in STATEMENT_DISPATCH:
        for alternative in alternatives(rules[dispatcher]):
            items: list[Node] = node_members(alternative)
            if len(items) == 1 and items[0][0] == NODE_REF and items[0][1] not in found:
                found.append(str(items[0][1]))
    return found


def walked_rules(rules: dict[str, GrammarRule]) -> list[str]:
    """Every rule reachable from the grammar's entry rules, entries and statements first."""
    order: list[str] = [*WALK_ROOTS, *STATEMENT_DISPATCH, *statement_rules(rules)]
    seen: set[str] = set(order)
    frontier: list[str] = list(order)
    while frontier:
        for found in references(rules[frontier.pop(0)].body):
            if found in rules and found not in seen:
                seen.add(found)
                order.append(found)
                frontier.append(found)
    return order


def descendants(*, name: str, rules: dict[str, GrammarRule]) -> set[str]:
    """Every rule a rule reaches."""
    seen: set[str] = set()
    frontier: list[str] = [name]
    while frontier:
        for found in references(rules[frontier.pop()].body):
            if found in rules and found not in seen:
                seen.add(found)
                frontier.append(found)
    return seen


def segments(sequence: tuple[str, ...]) -> list[tuple[str, ...]]:
    """The contiguous keyword runs of a sequence, split at its wildcards."""
    out: list[tuple[str, ...]] = []
    for is_wild, run in itertools.groupby(sequence, key=lambda w: w == WILDCARD):
        if not is_wild:
            out.append(tuple(run))
    return out


class _Compiled:
    """One catalogued grammar as a keyword automaton: placeholders and punctuation are empty."""

    def __init__(self, *, grammar: str, definitions: dict[str, str]) -> None:
        self.nfa: Nfa = [[]]
        self.words: set[str] = set()
        self.definitions: dict[str, str] = definitions
        self.end: int = self.build(grammar=grammar, start=0, stack=())

    def state(self) -> int:
        """A new state."""
        self.nfa.append([])
        return len(self.nfa) - 1

    def build(self, *, grammar: str, start: int, stack: tuple[str, ...]) -> int:
        """The grammar's states from `start`; its end state."""
        tokens: list[str] = FORM_TOKEN.findall(grammar)
        end, rest = self.alternation(tokens=tokens, index=0, start=start, stack=stack)
        while rest < len(tokens):
            end, rest = self.alternation(tokens=tokens, index=rest + 1, start=end, stack=stack)
        return end

    def alternation(
        self, *, tokens: list[str], index: int, start: int, stack: tuple[str, ...]
    ) -> tuple[int, int]:
        """Alternatives separated by `|` up to a closing bracket; (end state, next index)."""
        ends: list[int] = []
        while True:
            end, index = self.sequence(tokens=tokens, index=index, start=start, stack=stack)
            ends.append(end)
            if index < len(tokens) and tokens[index] == FORM_ALTERNATIVE:
                index += 1
                continue
            break
        joined: int = self.state()
        for end in ends:
            self.nfa[end].append(("", joined))
        return joined, index

    def item(self, *, token: str, current: int, stack: tuple[str, ...]) -> int:
        """The state after one elision, placeholder (its definition inlined) or keyword."""
        placeholder: re.Match[str] | None = FORM_PLACEHOLDER.fullmatch(token)
        if token == FORM_ELISION:
            loop: int = self.state()
            self.nfa[current].append(("", loop))
            self.nfa[loop].append((WILDCARD, loop))
            return loop
        if placeholder:
            name: str = placeholder.group(1)
            if name in self.definitions and name not in stack and len(stack) < INLINE_DEPTH:
                return self.build(
                    grammar=self.definitions[name], start=current, stack=(*stack, name)
                )
            return current
        if KEYWORD_WORD.match(token):
            following: int = self.state()
            self.nfa[current].append((token, following))
            self.words.add(token)
            return following
        return current

    def sequence(
        self, *, tokens: list[str], index: int, start: int, stack: tuple[str, ...]
    ) -> tuple[int, int]:
        """Items up to `|` or a closing bracket."""
        current: int = start
        while index < len(tokens) and tokens[index] not in FORM_CLOSERS:
            token: str = tokens[index]
            index += 1
            if token in (FORM_OPTIONAL_OPEN, FORM_CHOICE_OPEN):
                inner, index = self.alternation(
                    tokens=tokens, index=index, start=current, stack=stack
                )
                index += 1
                if token == FORM_OPTIONAL_OPEN:
                    self.nfa[current].append(("", inner))
                current = inner
            else:
                current = self.item(token=token, current=current, stack=stack)
        return current, index

    def closure(self, states: set[int]) -> set[int]:
        """The states reachable through empty moves."""
        frontier: list[int] = list(states)
        seen: set[int] = set(states)
        while frontier:
            for label, target in self.nfa[frontier.pop()]:
                if not label and target not in seen:
                    seen.add(target)
                    frontier.append(target)
        return seen

    def moves(
        self, *, at: tuple[int, int, int], runs: list[tuple[str, ...]], anchored: bool
    ) -> Iterator[tuple[int, int, int] | None]:
        """The next (state, run, position) triples; None once every run is spelled."""
        state, run, position = at
        for here in self.closure({state}):
            for label, target in self.nfa[here]:
                if not label:
                    continue
                if label != WILDCARD and label == runs[run][position]:
                    last_word: bool = position + 1 == len(runs[run])
                    if last_word and run + 1 == len(runs):
                        yield None
                    yield (target, run + 1, 0) if last_word else (target, run, position + 1)
                if position == 0 and (run > 0 or not anchored):
                    yield (target, run, 0)

    def spells(self, *, runs: list[tuple[str, ...]], anchored: bool) -> bool:
        """True when a path spells each run contiguously and in order, other words only between."""
        if not set(itertools.chain.from_iterable(runs)) <= self.words:
            return False
        seen: set[tuple[int, int, int]] = set()
        frontier: list[tuple[int, int, int]] = [(0, 0, 0)]
        while frontier:
            for move in self.moves(at=frontier.pop(), runs=runs, anchored=anchored):
                if move is None:
                    return True
                if move not in seen:
                    seen.add(move)
                    frontier.append(move)
        return False


def catalogue_tables(root: Path) -> list[dict[str, list[dict[str, object]]]]:
    """The root's catalogue files, in path order."""
    return [
        tomllib.loads(path.read_text()) for path in sorted((root / CATALOGUE_DIR).glob("*.toml"))
    ]


def definition_body(definition: dict[str, object]) -> str:
    """A catalogued definition's grammar after `::=`."""
    return str(definition["grammar"]).split(DEFINITION_SEPARATOR, 1)[1].strip()


def page_definitions(
    tables: list[dict[str, list[dict[str, object]]]],
) -> dict[str, dict[str, str]]:
    """Each page's definitions by name; a repeated name keeps its first body."""
    definitions: dict[str, dict[str, str]] = {}
    for data in tables:
        for definition in data.get("definition", []):
            page: dict[str, str] = definitions.setdefault(str(definition["source"]), {})
            page.setdefault(str(definition["name"]), definition_body(definition))
    return definitions


def catalogue_grammars(
    root: Path,
) -> tuple[list[tuple[str, _Compiled]], list[_Compiled], set[str]]:
    """Each form line and definition compiled, and every bare word the catalogue spells."""
    tables: list[dict[str, list[dict[str, object]]]] = catalogue_tables(root)
    definitions: dict[str, dict[str, str]] = page_definitions(tables)
    words: set[str] = set()
    forms: list[tuple[str, _Compiled]] = []
    clauses: list[_Compiled] = []
    for data in tables:
        for form in data.get("form", []):
            page: dict[str, str] = definitions.get(str(form["source"]), {})
            for line in strings(form["grammar"]):
                forms.append((str(form["id"]), _Compiled(grammar=line, definitions=page)))
                words |= {w.upper() for w in bare_words(line)}
        for definition in data.get("definition", []):
            body: str = definition_body(definition)
            page = definitions.get(str(definition["source"]), {})
            clauses.append(_Compiled(grammar=body, definitions=page))
            words |= {w.upper() for w in bare_words(body)}
    return forms, clauses, words


def covered(
    *,
    sequence: tuple[str, ...],
    leading: bool,
    forms: list[tuple[str, _Compiled]],
    clauses: list[_Compiled],
) -> bool:
    """True when one catalogued grammar spells the sequence (a statement's from its start)."""
    runs: list[tuple[str, ...]] = segments(sequence)
    anchored: bool = leading and sequence[0] != WILDCARD
    if any(compiled.spells(runs=runs, anchored=anchored) for _, compiled in forms):
        return True
    return not leading and any(c.spells(runs=runs, anchored=False) for c in clauses)


def is_subsequence(*, needle: tuple[str, ...], haystack: tuple[str, ...]) -> bool:
    """True when the needle's words appear in the haystack in order."""
    iterator: Iterator[str] = iter(haystack)
    return all(word in iterator for word in needle)


def contains_run(*, big: tuple[str, ...], small: tuple[str, ...]) -> bool:
    """True when the small sequence appears as a contiguous run of the big one."""
    return any(big[i : i + len(small)] == small for i in range(len(big) - len(small) + 1))


def bare(keywords: str) -> tuple[str, ...]:
    """A gap's keywords without its wildcards."""
    return tuple(w for w in keywords.split() if w != WILDCARD)


def runs_below(
    *, reached: set[str], by_rule: dict[str, list[tuple[str, ...]]]
) -> list[tuple[str, ...]]:
    """Every gap sequence of the rules a gap's rule reaches."""
    runs: list[tuple[str, ...]] = []
    for name in reached:
        runs.extend(by_rule.get(name, []))
    return runs


def split_derived(
    *, gaps: list[AuditGap], rules: dict[str, GrammarRule], own: frozenset[str]
) -> tuple[list[AuditGap], int]:
    """Root gaps and the count of derived ones; a gap `own` names stays a root."""
    by_rule: dict[str, list[tuple[str, ...]]] = {}
    by_alternative: dict[tuple[str, int], list[tuple[str, ...]]] = {}
    for gap in gaps:
        by_rule.setdefault(gap.rule, []).append(tuple(gap.keywords.split()))
        by_alternative.setdefault((gap.rule, gap.alternative), []).append(bare(gap.keywords))
    roots: list[AuditGap] = []
    derived: int = 0
    reach: dict[str, set[str]] = {}
    for gap in gaps:
        words: tuple[str, ...] = tuple(gap.keywords.split())
        if gap.rule not in reach:
            reach[gap.rule] = descendants(name=gap.rule, rules=rules) - {gap.rule}
        smaller: list[tuple[str, ...]] = runs_below(reached=reach[gap.rule], by_rule=by_rule)
        fewer: list[tuple[str, ...]] = [
            run
            for run in by_alternative[(gap.rule, gap.alternative)]
            if len(run) < len(bare(gap.keywords))
        ]
        if f"{gap.rule}: {gap.keywords}" not in own and (
            any(contains_run(big=words, small=run) for run in smaller if run != words)
            or any(is_subsequence(needle=run, haystack=bare(gap.keywords)) for run in fewer)
        ):
            derived += 1
        else:
            roots.append(gap)
    return roots, derived


def documented_functions(pages: list[BigQueryPage]) -> list[str]:
    """The function names the reference's all-functions index lists."""
    for page in pages:
        if page.path == FUNCTION_INDEX_PAGE:
            return sorted(set(FUNCTION_INDEX_ENTRY.findall(page.text)))
    raise BigQueryCatalogueError(f"{FUNCTION_INDEX_PAGE} is not in the pinned cache")


def documented_types(pages: list[BigQueryPage]) -> list[str]:
    """The SQL type names and aliases of the data types page's summary table."""
    names: set[str] = set()
    for page in pages:
        if page.path == DATA_TYPES_PAGE:
            for match in TYPE_NAME.finditer(page.text):
                names.add(match.group(1))
                names |= set(TYPE_ALIAS_SPAN.findall(match.group(2) or ""))
    return sorted(names)


def page_texts(page: BigQueryPage) -> list[str]:
    """A page's code lines and its inline code spans, read outside fenced blocks."""
    texts: list[str] = []
    for block in page.blocks:
        texts.extend(block.text.splitlines())
    prose: str = FENCED_BLOCK.sub("", page.text).replace(ESCAPED_TICK, "")
    return texts + [span for _, span in PAGE_CODE_SPAN.findall(prose)]


def doc_streams(pages: list[BigQueryPage]) -> list[tuple[str, list[str]]]:
    """Every code line and code span of the pages as a stream of upper-case words."""
    streams: list[tuple[str, list[str]]] = []
    for page in pages:
        for text in page_texts(page):
            words: list[str] = [w for w in WORD_TOKEN.findall(text) if KEYWORD_WORD.match(w)]
            if words:
                streams.append((page.id, words))
    return streams


def run_at(*, words: list[str], run: tuple[str, ...], start: int, last: int) -> int:
    """The first index from `start` to `last` where the run starts, or -1."""
    for index in range(start, last + 1):
        if tuple(words[index : index + len(run)]) == run:
            return index
    return -1


def documented_at(*, sequence: str, streams: list[tuple[str, list[str]]], leading: bool) -> str:
    """The first page with a code line or span spelling the runs in order (a statement first)."""
    runs: list[tuple[str, ...]] = segments(tuple(sequence.split()))
    for page, words in streams:
        position: int = 0
        for number, run in enumerate(runs):
            last: int = 0 if leading and number == 0 else len(words) - len(run)
            found: int = run_at(words=words, run=run, start=position, last=last)
            if found < 0:
                break
            position = found + len(run)
        else:
            return page
    return ""


def checked_sequence(*, sequence: tuple[str, ...], leading: bool) -> tuple[str, ...]:
    """The sequence with outer wildcards cut; empty when it spells too few keywords to compare."""
    stripped: tuple[str, ...] = sequence
    while stripped and stripped[-1] == WILDCARD:
        stripped = stripped[:-1]
    while not leading and stripped and stripped[0] == WILDCARD:
        stripped = stripped[1:]
    count: int = len(bare(" ".join(stripped)))
    return () if not count or (not leading and count < MIN_CLAUSE_KEYWORDS) else stripped


class _Walk:
    """The gap walk over every walked rule alternative."""

    def __init__(self, *, cache: Path, root: Path) -> None:
        self.rules: dict[str, GrammarRule] = read_rules(cache)
        self.keywords: dict[str, str] = keyword_table(cache)
        self.forms, self.clauses, self.words = catalogue_grammars(root)
        self.statements: set[str] = set(statement_rules(self.rules))
        self.walked: list[str] = walked_rules(self.rules)
        self.checked: int = 0
        self.rejected: int = 0

    def gaps(self) -> list[AuditGap]:
        """Every distinct uncovered sequence, first alternative first."""
        speller: _Speller = _Speller(rules=self.rules, keywords=self.keywords)
        unique: dict[str, AuditGap] = {}
        for name in self.walked:
            leading: bool = name in self.statements
            for index, alternative in enumerate(alternatives(self.rules[name]), 1):
                features: tuple[str, ...] = self.rules[name].features[index - 1]
                if REJECTED in features:
                    self.rejected += 1
                    continue
                for sequence in speller.spell(node=alternative) or []:
                    found: tuple[str, ...] = checked_sequence(sequence=sequence, leading=leading)
                    if not found:
                        continue
                    self.checked += 1
                    if covered(
                        sequence=found, leading=leading, forms=self.forms, clauses=self.clauses
                    ):
                        continue
                    gap: AuditGap = AuditGap(
                        rule=name, alternative=index, keywords=" ".join(found), features=features
                    )
                    unique.setdefault(f"{name}: {gap.keywords}", gap)
        return list(unique.values())


def form_grammars(root: Path) -> str:
    """Every catalogued form's grammar lines, one per line."""
    lines: list[str] = []
    for data in catalogue_tables(root):
        for form in data.get("form", []):
            lines.extend(strings(form["grammar"]))
    return "\n".join(lines)


def audit(*, cache: Path, root: Path, own: frozenset[str] = frozenset()) -> AuditResult:
    """Grammar sequences, keywords, functions and types no form spells; `own` gaps stay roots."""
    walk: _Walk = _Walk(cache=cache, root=root)
    roots, derived = split_derived(gaps=walk.gaps(), rules=walk.rules, own=own)
    pages: list[BigQueryPage] = load_pages(cache)
    functions: list[str] = documented_functions(pages)
    grammars: str = form_grammars(root)
    types: list[str] = documented_types(pages)
    return AuditResult(
        gaps=roots,
        derived_gaps=derived,
        missing_keywords=sorted(k for k in walk.keywords if k not in walk.words),
        dispatched_words=sorted(d for d in dispatched_words(cache) if d not in walk.words),
        missing_functions=[
            f for f in functions if not _function_spelled(name=f, grammars=grammars)
        ],
        missing_types=[t for t in types if t not in walk.words],
        sequences_checked=walk.checked,
        rules_checked=len(walk.walked),
        walked=walk.walked,
        unwalked=sorted(name for name in walk.rules if name not in set(walk.walked)),
        keywords_checked=len(walk.keywords),
        functions_checked=len(functions),
        types_checked=len(types),
        features=_feature_sites(walk.rules),
        rejected_alternatives=walk.rejected,
        statements=frozenset(walk.statements),
        form_ids=frozenset(form_id for form_id, _ in walk.forms),
        doc_streams=doc_streams(pages),
    )


def _feature_sites(rules: dict[str, GrammarRule]) -> dict[str, tuple[str, ...]]:
    """Each language feature the actions test, with the `rule#alternative` sites testing it."""
    sites: dict[str, list[str]] = {}
    for name, rule in rules.items():
        for index, tested in enumerate(rule.features, 1):
            sites.update(
                {f: [*sites.get(f, []), f"{name}#{index}"] for f in tested if f != REJECTED}
            )
    return {feature: tuple(found) for feature, found in sorted(sites.items())}


def _function_spelled(*, name: str, grammars: str) -> bool:
    """True when a catalogued grammar writes the name as a word or a dotted name."""
    return re.search(rf"(?<![\w.]){re.escape(name)}(?![\w.])", grammars) is not None
