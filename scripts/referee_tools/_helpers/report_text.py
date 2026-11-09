"""The comparison report as Markdown, and every disagreement as a replayable case."""

from __future__ import annotations

import json
from collections import Counter
from typing import Any

from scripts.referee_tools._helpers.classify import is_syntax_error
from scripts.referee_tools.constants import (
    ACCEPTED_OUTCOME,
    BOTH_ACCEPT,
    BOTH_SYNTAX,
    CAUGHT_AT_PARSE,
    FUNCTIONS_FILE,
    MISSED,
    NOT_RECORDED,
    REASON_NOISE,
    SWEEP_KINDS,
    TOO_LENIENT,
    TOO_STRICT,
    TOP_PATTERNS,
    VALID_KIND,
)
from scripts.referee_tools.models import Comparison

_CATEGORIES: tuple[str, ...] = (
    BOTH_ACCEPT,
    TOO_STRICT,
    TOO_LENIENT,
    BOTH_SYNTAX,
    CAUGHT_AT_PARSE,
    MISSED,
    NOT_RECORDED,
)
_DISAGREEMENTS: frozenset[str] = frozenset({TOO_STRICT, TOO_LENIENT, CAUGHT_AT_PARSE, MISSED})


def report_markdown(
    *, comparisons: list[Comparison], arity_rows: list[tuple[str, str, str]], catalogue: str
) -> str:
    """Corpus sizes, categories, rates, top disagreement patterns and arity mismatches."""
    lines: list[str] = ["# Snowflake referee: SQLQuack parse against Snowflake", ""]
    lines += _sizes(comparisons) + _categories(comparisons) + _rates(comparisons)
    lines += _mutants(comparisons) + _arity_sweep(comparisons)
    for title, chosen in (
        ("SQLQuack too strict", TOO_STRICT),
        ("SQLQuack too lenient on syntax", TOO_LENIENT),
        ("Binding errors caught at parse", CAUGHT_AT_PARSE),
        ("Missed errors", MISSED),
    ):
        lines += _patterns(title=title, comparisons=comparisons, chosen=chosen)
    lines += [f"## SQLQuack's arity table against the catalogue ({catalogue})", ""]
    lines += ["| function | SQLQuack | Snowflake |", "|---|---|---|"]
    lines += [f"| {name} | {ours} | {theirs} |" for name, ours, theirs in arity_rows]
    return "\n".join(lines) + "\n"


def disagreement_lines(comparisons: list[Comparison]) -> str:
    """Every disagreeing case as one JSON line: hash, reference, statement and both verdicts."""
    rows: list[str] = []
    for comparison in comparisons:
        if comparison.category not in _DISAGREEMENTS:
            continue
        rows.append(
            json.dumps(
                {
                    "category": comparison.category,
                    "case_hash": comparison.case.case_hash,
                    "case_reference": comparison.case.case_reference,
                    "environment": comparison.case.environment.name,
                    "statement": comparison.case.statement,
                    "snowflake": comparison.outcome,
                    "sqlquack_parse_error": comparison.verdict.reason or None,
                },
                sort_keys=True,
            )
        )
    return "\n".join(rows) + "\n"


def _corpus(comparison: Comparison) -> str:
    return comparison.case.case_reference.split("#", 1)[0]


def _sizes(comparisons: list[Comparison]) -> list[str]:
    counts: Counter[str] = Counter(map(_corpus, comparisons))
    lines: list[str] = ["## Corpus sizes", "", "| file | statements |", "|---|---|"]
    lines += [f"| {name} | {count} |" for name, count in sorted(counts.items())]
    return lines + [f"| total | {len(comparisons)} |", ""]


def _categories(comparisons: list[Comparison]) -> list[str]:
    corpora: list[str] = sorted(set(map(_corpus, comparisons)))
    lines: list[str] = ["## Categories", "", "| category | " + " | ".join(corpora) + " | all |"]
    lines.append("|---" * (len(corpora) + 2) + "|")
    for chosen in _CATEGORIES:
        counts: Counter[str] = Counter(_corpus(c) for c in comparisons if c.category == chosen)
        cells: list[int] = [counts[corpus] for corpus in corpora]
        total: int = sum(c.category == chosen for c in comparisons)
        lines.append(f"| {chosen} | " + " | ".join(map(str, cells)) + f" | {total} |")
    return lines + [""]


def _ratio(*, numerator: int, denominator: int) -> str:
    return (
        "n/a" if denominator == 0 else f"{numerator}/{denominator} = {numerator / denominator:.1%}"
    )


def _rates(comparisons: list[Comparison]) -> list[str]:
    counts: Counter[str] = Counter(c.category for c in comparisons)
    accepted: int = counts[BOTH_ACCEPT] + counts[TOO_STRICT]
    syntax: int = counts[TOO_LENIENT] + counts[BOTH_SYNTAX]
    binding: int = counts[CAUGHT_AT_PARSE] + counts[MISSED]
    agree: int = counts[BOTH_ACCEPT] + counts[BOTH_SYNTAX] + counts[MISSED]
    return [
        "## Rates (parse level)",
        "",
        f"- False parse-error rate (rejects what Snowflake accepts): "
        f"{_ratio(numerator=counts[TOO_STRICT], denominator=accepted)}",
        f"- Parser agreement (parses exactly when Snowflake finds no syntax error): "
        f"{_ratio(numerator=agree, denominator=accepted + syntax + binding)}",
        "- Syntax errors SQLQuack also rejects: "
        f"{_ratio(numerator=counts[BOTH_SYNTAX], denominator=syntax)}",
        "- Binding errors SQLQuack rejects (at parse): "
        f"{_ratio(numerator=counts[CAUGHT_AT_PARSE], denominator=binding)}",
        "- Type agreement: not measurable; the CLI does not expose inferred output types.",
        "",
    ]


def _mutants(comparisons: list[Comparison]) -> list[str]:
    bases: dict[str, Comparison] = {
        c.case.case_reference.rsplit("#", 1)[1]: c for c in comparisons if c.case.kind == VALID_KIND
    }
    lines: list[str] = [
        "## Mutants",
        "",
        "Mutants whose base Snowflake accepts and which Snowflake rejects; recall is the share",
        "SQLQuack rejects (parse level only).",
        "",
        "| mutation | Snowflake rejects | of which syntax | SQLQuack rejects | recall | "
        "Snowflake still accepts |",
        "|---|---|---|---|---|---|",
    ]
    mutations: list[str] = sorted({c.case.mutation for c in comparisons if c.case.mutation})
    for mutation in mutations:
        group: list[dict[str, Any]] = [
            c.outcome
            for c in comparisons
            if c.case.mutation == mutation
            and c.outcome is not None
            and _accepted(bases.get(c.case.base or ""))
        ]
        rejected: list[Comparison] = [
            c
            for c in comparisons
            if c.case.mutation == mutation
            and c.outcome is not None
            and c.outcome["outcome"] != ACCEPTED_OUTCOME
            and _accepted(bases.get(c.case.base or ""))
        ]
        caught: int = sum(not c.verdict.parses for c in rejected)
        syntax: int = sum(is_syntax_error(c.outcome or {}) for c in rejected)
        lines.append(
            f"| {mutation} | {len(rejected)} | {syntax} | {caught} | "
            f"{_ratio(numerator=caught, denominator=len(rejected))} | "
            f"{len(group) - len(rejected)} |"
        )
    return lines + [""]


def _accepted(comparison: Comparison | None) -> bool:
    return (
        comparison is not None
        and comparison.outcome is not None
        and comparison.outcome["outcome"] == ACCEPTED_OUTCOME
    )


def _arity_sweep(comparisons: list[Comparison]) -> list[str]:
    strict: Counter[str] = Counter(
        c.case.case_reference.rsplit("#", 1)[1].split("/", 1)[0]
        for c in comparisons
        if c.category == TOO_STRICT
        and c.case.kind in SWEEP_KINDS
        and c.case.case_reference.startswith(FUNCTIONS_FILE)
    )
    lines: list[str] = [
        "## Function sweeps SQLQuack rejects though Snowflake accepts",
        "",
        "| function | statements |",
        "|---|---|",
    ]
    lines += [f"| {name.upper()} | {count} |" for name, count in strict.most_common()]
    return lines + [""]


def _pattern(comparison: Comparison) -> str:
    if comparison.category == TOO_STRICT:
        return REASON_NOISE.sub("…", comparison.verdict.reason)
    error: dict[str, Any] = (comparison.outcome or {}).get("error") or {}
    message: str = str(error.get("message") or "")
    return f"{error.get('errno')}: {REASON_NOISE.sub('…', message)[:140]}"


def _patterns(*, title: str, comparisons: list[Comparison], chosen: str) -> list[str]:
    group: list[Comparison] = [c for c in comparisons if c.category == chosen]
    patterns: dict[str, list[Comparison]] = {}
    for comparison in group:
        patterns.setdefault(_pattern(comparison), []).append(comparison)
    lines: list[str] = [f"## {title} ({len(group)})", ""]
    ranked: list[tuple[str, list[Comparison]]] = sorted(
        patterns.items(), key=lambda item: (-len(item[1]), item[0])
    )
    for pattern, members in ranked[:TOP_PATTERNS]:
        shortest: Comparison = min(members, key=lambda c: (len(c.case.statement), c.case.statement))
        lines += [
            f"- **{len(members)}** × `{pattern}`",
            f"  - e.g. `{shortest.case.statement[:300]}` ({shortest.case.case_reference})",
        ]
    return lines + [""]
